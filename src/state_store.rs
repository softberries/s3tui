//! This module provides functionality for interactions between UI and state
use crate::model::action::Action;
use crate::model::download_progress_item::DownloadProgressItem;
use crate::model::error::{LocalError, S3Error};
use crate::model::has_children::flatten_items;
use crate::model::local_data_item::LocalDataItem;
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_data_item::S3DataItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::state::{ActivePage, State};
use crate::model::transfer_state::TransferState;
use crate::model::upload_progress_item::UploadProgressItem;
use crate::services::local_data_fetcher::LocalDataFetcher;
use crate::services::s3_data_fetcher::S3DataFetcher;
use crate::services::transfer_manager::{JobId, TransferManager};
use crate::settings::file_credentials::FileCredential;
use crate::termination::{Interrupted, Terminator};
use color_eyre::eyre;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{broadcast, mpsc, Mutex};

/// Maximum simultaneous uploads/downloads
static S3_OPERATIONS_CONCURRENCY_LEVEL: usize = 8;

/// Message sent when a transfer job completes or fails
#[derive(Debug, Clone)]
pub enum TransferResult {
    UploadComplete(LocalSelectedItem),
    UploadFailed(LocalSelectedItem, String),
    DownloadComplete(S3SelectedItem),
    DownloadFailed(S3SelectedItem, String),
}

/// Handles all the actions, calls methods on external services and updates the state when necessary
pub struct StateStore {
    state_tx: UnboundedSender<State>,
}

impl StateStore {
    pub fn new() -> (Self, UnboundedReceiver<State>) {
        let (state_tx, state_rx) = mpsc::unbounded_channel::<State>();

        (StateStore { state_tx }, state_rx)
    }
}

impl StateStore {
    /// Enqueue downloads in the transfer manager and return updated items with job_ids
    async fn enqueue_downloads(
        transfer_manager: &TransferManager,
        s3_selected_items: Vec<S3SelectedItem>,
        job_item_map: &Arc<Mutex<HashMap<JobId, TransferJobInfo>>>,
    ) -> Vec<S3SelectedItem> {
        let items_with_children = flatten_items(s3_selected_items);
        let mut updated_items = Vec::new();

        for mut item in items_with_children {
            if !item.is_bucket && !item.is_directory {
                let s3_path = item.path.clone().unwrap_or_else(|| item.name.clone());
                let local_path = format!("{}/{}", item.destination_dir, item.name);

                let job_id = transfer_manager
                    .enqueue_download(s3_path.clone(), local_path, None)
                    .await;

                item.job_id = Some(job_id);
                item.transfer_state = TransferState::Pending;

                // Store mapping for later lookup
                job_item_map.lock().await.insert(
                    job_id,
                    TransferJobInfo::Download {
                        item: item.clone(),
                    },
                );

                updated_items.push(item);
            }
        }
        updated_items
    }

    /// Enqueue uploads in the transfer manager and return updated items with job_ids
    async fn enqueue_uploads(
        transfer_manager: &TransferManager,
        local_selected_items: Vec<LocalSelectedItem>,
        job_item_map: &Arc<Mutex<HashMap<JobId, TransferJobInfo>>>,
    ) -> Vec<LocalSelectedItem> {
        let items_with_children = flatten_items(local_selected_items);
        let mut updated_items = Vec::new();

        for mut item in items_with_children {
            if !item.is_directory {
                let local_path = item.path.clone();
                let s3_path = if item.destination_path.is_empty() {
                    item.name.clone()
                } else {
                    format!("{}/{}", item.destination_path, item.name)
                };

                let job_id = transfer_manager
                    .enqueue_upload(local_path.clone(), s3_path, None)
                    .await;

                item.job_id = Some(job_id);
                item.transfer_state = TransferState::Pending;

                // Store mapping for later lookup
                job_item_map.lock().await.insert(
                    job_id,
                    TransferJobInfo::Upload { item: item.clone() },
                );

                updated_items.push(item);
            }
        }
        updated_items
    }

    /// Spawn the transfer worker that processes jobs from the TransferManager
    fn spawn_transfer_worker(
        transfer_manager: Arc<TransferManager>,
        s3_data_fetcher: S3DataFetcher,
        job_item_map: Arc<Mutex<HashMap<JobId, TransferJobInfo>>>,
        result_tx: UnboundedSender<TransferResult>,
        upload_progress_tx: UnboundedSender<UploadProgressItem>,
        download_progress_tx: UnboundedSender<DownloadProgressItem>,
    ) {
        tokio::spawn(async move {
            loop {
                // Try to get the next job
                if let Some(job) = transfer_manager.try_get_next().await {
                    let job_id = job.id;
                    let job_info = job_item_map.lock().await.get(&job_id).cloned();

                    match job_info {
                        Some(TransferJobInfo::Upload { item }) => {
                            let up_tx = upload_progress_tx.clone();
                            let fetcher = s3_data_fetcher.clone();
                            let tm = transfer_manager.clone();
                            let tx = result_tx.clone();

                            match fetcher.upload_item(item.clone(), up_tx).await {
                                Ok(_) => {
                                    tm.mark_completed(job_id).await;
                                    let mut completed_item = item;
                                    completed_item.transfer_state = TransferState::Completed;
                                    let _ = tx.send(TransferResult::UploadComplete(completed_item));
                                }
                                Err(e) => {
                                    let error_msg = e.to_string();
                                    tm.mark_failed(job_id, error_msg.clone()).await;
                                    let mut failed_item = item;
                                    failed_item.transfer_state =
                                        TransferState::Failed(error_msg.clone());
                                    let _ =
                                        tx.send(TransferResult::UploadFailed(failed_item, error_msg));
                                }
                            }
                        }
                        Some(TransferJobInfo::Download { item }) => {
                            let down_tx = download_progress_tx.clone();
                            let fetcher = s3_data_fetcher.clone();
                            let tm = transfer_manager.clone();
                            let tx = result_tx.clone();

                            match fetcher.download_item(item.clone(), down_tx).await {
                                Ok(_) => {
                                    tm.mark_completed(job_id).await;
                                    let mut completed_item = item;
                                    completed_item.transfer_state = TransferState::Completed;
                                    let _ =
                                        tx.send(TransferResult::DownloadComplete(completed_item));
                                }
                                Err(e) => {
                                    let error_msg = e.to_string();
                                    tm.mark_failed(job_id, error_msg.clone()).await;
                                    let mut failed_item = item;
                                    failed_item.transfer_state =
                                        TransferState::Failed(error_msg.clone());
                                    let _ = tx
                                        .send(TransferResult::DownloadFailed(failed_item, error_msg));
                                }
                            }
                        }
                        None => {
                            // Job info not found, skip
                            tracing::warn!("Job info not found for job_id: {}", job_id);
                        }
                    }
                } else {
                    // No jobs available, wait a bit before checking again
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        });
    }

    async fn fetch_s3_data(
        &self,
        bucket: Option<String>,
        prefix: Option<String>,
        s3_data_fetcher: S3DataFetcher,
        s3_tx: UnboundedSender<(Option<String>, Option<String>, Vec<S3DataItem>)>,
    ) {
        tokio::spawn(async move {
            match s3_data_fetcher
                .list_current_location(bucket.clone(), prefix.clone())
                .await
            {
                Ok(data) => {
                    let _ = s3_tx.send((bucket.clone(), prefix.clone(), data));
                }
                Err(e) => {
                    tracing::error!("Failed to fetch S3 data: {}", e);
                }
            }
        });
    }

    async fn list_s3_data_recursive(
        &self,
        item: S3SelectedItem,
        s3_data_fetcher: S3DataFetcher,
        s3_full_list_tx: UnboundedSender<(Option<String>, Option<String>, Vec<S3DataItem>)>,
    ) {
        tracing::info!("list_s3_Data_recursive");
        tokio::spawn(async move {
            let bucket_name = if item.is_bucket {
                item.name
            } else {
                item.bucket.unwrap_or(item.name)
            };
            let path = if item.is_bucket {
                None
            } else {
                item.path.clone()
            };
            match s3_data_fetcher
                .list_all_objects(&bucket_name, path.clone())
                .await
            {
                Ok(data) => {
                    tracing::info!("Downloaded items: {}", data.len());
                    let _ = s3_full_list_tx.send((Some(bucket_name), path.clone(), data));
                }
                Err(e) => {
                    tracing::error!("Failed to fetch S3 data: {}", e);
                }
            }
        });
    }

    async fn fetch_local_data(
        &self,
        dir_path: Option<String>,
        local_data_fetcher: LocalDataFetcher,
        local_tx: UnboundedSender<(String, Vec<LocalDataItem>)>,
    ) {
        let path = Self::get_directory_path(dir_path);
        tokio::spawn(async move {
            match local_data_fetcher.read_directory(path.clone()).await {
                Ok(data) => {
                    let _ = local_tx.send((path.clone().unwrap_or("/".to_string()), data));
                }
                Err(e) => {
                    tracing::error!("Failed to fetch local data: {}", e);
                    // Handle error, maybe retry or send error state
                }
            }
        });
    }

    fn get_directory_path(input_path: Option<String>) -> Option<String> {
        match input_path {
            Some(path) => {
                let path = Path::new(&path);
                if path.is_dir() {
                    // If the path itself is a directory, return it as is
                    path.to_str().map(String::from)
                } else {
                    // Otherwise, return the parent directory if available
                    path.parent().and_then(|p| p.to_str().map(String::from))
                }
            }
            None => None,
        }
    }

    async fn move_back_local_data(
        &self,
        current_path: String,
        local_data_fetcher: LocalDataFetcher,
        local_tx: UnboundedSender<(String, Vec<LocalDataItem>)>,
    ) {
        tokio::spawn(async move {
            let path = Path::new(&current_path);

            match local_data_fetcher.read_parent_directory().await {
                Ok(data) => {
                    let _ = match path.parent() {
                        Some(p_path) => local_tx.send((p_path.to_string_lossy().to_string(), data)),
                        None => local_tx.send((current_path, data)),
                    };
                }
                Err(e) => {
                    tracing::error!("Failed to fetch local data: {}", e);
                    // Handle error, maybe retry or send error state
                }
            }
        });
    }

    async fn delete_local_data(
        &self,
        item: LocalSelectedItem,
        local_data_fetcher: LocalDataFetcher,
        local_deleted_tx: UnboundedSender<Option<LocalError>>,
    ) {
        let path = item.path.clone();
        if item.is_directory {
            tokio::spawn(async move {
                match local_data_fetcher.delete_directory(path.clone()).await {
                    Ok(_) => {
                        let _ = local_deleted_tx.send(None);
                    }
                    Err(e) => {
                        tracing::error!("Failed to delete local directory: {}", e);
                        let _ =
                            local_deleted_tx.send(Some(LocalError::from_message(e.to_string())));
                    }
                }
            });
        } else {
            tokio::spawn(async move {
                match local_data_fetcher.delete_file(path.clone()).await {
                    Ok(_) => {
                        let _ = local_deleted_tx.send(None);
                    }
                    Err(e) => {
                        tracing::error!("Failed to delete local file: {}", e);
                        let _ =
                            local_deleted_tx.send(Some(LocalError::from_message(e.to_string())));
                    }
                }
            });
        }
    }

    async fn delete_s3_data(
        &self,
        item: S3SelectedItem,
        s3_data_fetcher: S3DataFetcher,
        s3_delete_tx: UnboundedSender<Option<S3Error>>,
    ) {
        let items_with_children = flatten_items(vec![item]);
        for item in items_with_children {
            if !item.is_directory {
                let delete_tx = s3_delete_tx.clone();
                let fetcher = s3_data_fetcher.clone();

                tokio::spawn(async move {
                    match fetcher
                        .delete_data(
                            item.is_bucket,
                            item.bucket.clone(),
                            item.path.clone().unwrap_or(item.name.clone()),
                            item.is_directory,
                        )
                        .await
                    {
                        Ok(_) => {
                            let _ = delete_tx.send(None);
                        }
                        Err(e) => {
                            tracing::error!("Failed to delete S3 data: {}", e);
                            let _ = delete_tx.send(Some(S3Error::from_message(e.to_string())));
                        }
                    }
                });
            }
        }
    }

    async fn create_bucket(
        &self,
        name: String,
        s3_data_fetcher: S3DataFetcher,
        create_bucket_tx: UnboundedSender<Option<S3Error>>,
    ) {
        tokio::spawn(async move {
            match s3_data_fetcher
                .create_bucket(name.clone(), s3_data_fetcher.default_region.clone())
                .await
            {
                Ok(_) => {
                    let _ = create_bucket_tx.send(None);
                }
                Err(e) => {
                    tracing::error!("Failed to create S3 bucket: {}", e);
                    let _ = create_bucket_tx.send(Some(S3Error::from_message(e.to_string())));
                }
            }
        });
    }

    fn get_current_s3_fetcher(state: &State) -> S3DataFetcher {
        S3DataFetcher::new(state.current_creds.clone())
    }

    pub async fn main_loop(
        self,
        mut terminator: Terminator,
        mut action_rx: UnboundedReceiver<Action>,
        mut interrupt_rx: broadcast::Receiver<Interrupted>,
        creds: Vec<FileCredential>,
    ) -> eyre::Result<Interrupted> {
        let local_data_fetcher = LocalDataFetcher::new();
        let transfer_manager = Arc::new(TransferManager::new(S3_OPERATIONS_CONCURRENCY_LEVEL));
        let job_item_map: Arc<Mutex<HashMap<JobId, TransferJobInfo>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let mut state = State::new(creds.clone());
        let s3_data_fetcher = Self::get_current_s3_fetcher(&state);
        state.set_s3_loading(true);
        state.set_current_local_path(
            dirs::home_dir()
                .unwrap()
                .as_path()
                .to_string_lossy()
                .to_string(),
        );

        let (s3_tx, mut s3_rx) =
            mpsc::unbounded_channel::<(Option<String>, Option<String>, Vec<S3DataItem>)>();
        let (s3_full_list_tx, mut s3_full_list_rx) =
            mpsc::unbounded_channel::<(Option<String>, Option<String>, Vec<S3DataItem>)>();
        let (s3_deleted_tx, mut s3_deleted_rx) = mpsc::unbounded_channel::<Option<S3Error>>();
        let (local_tx, mut local_rx) = mpsc::unbounded_channel::<(String, Vec<LocalDataItem>)>();
        let (local_deleted_tx, mut local_deleted_rx) =
            mpsc::unbounded_channel::<Option<LocalError>>();
        let (upload_tx, mut upload_rx) = mpsc::unbounded_channel::<UploadProgressItem>();
        let (download_tx, mut download_rx) = mpsc::unbounded_channel::<DownloadProgressItem>();
        let (create_bucket_tx, mut create_bucket_rx) = mpsc::unbounded_channel::<Option<S3Error>>();
        let (transfer_result_tx, mut transfer_result_rx) =
            mpsc::unbounded_channel::<TransferResult>();

        // Spawn the transfer worker
        Self::spawn_transfer_worker(
            transfer_manager.clone(),
            s3_data_fetcher.clone(),
            job_item_map.clone(),
            transfer_result_tx,
            upload_tx.clone(),
            download_tx.clone(),
        );

        self.fetch_s3_data(None, None, s3_data_fetcher.clone(), s3_tx.clone())
            .await;
        self.fetch_local_data(
            Some(
                dirs::home_dir()
                    .unwrap()
                    .as_path()
                    .to_string_lossy()
                    .to_string(),
            ),
            local_data_fetcher.clone(),
            local_tx.clone(),
        )
        .await;

        // the initial state once
        self.state_tx.send(state.clone())?;

        let _ticker = tokio::time::interval(Duration::from_secs(1));

        let result = loop {
            tokio::select! {
                        Some(action) = action_rx.recv() => match action {
                            Action::Exit => {
                                let _ = terminator.terminate(Interrupted::UserInt);
                                break Interrupted::UserInt;
                            },
                            Action::Navigate { page} => {
                                state.set_active_page(page);
                                let _ = self.state_tx.send(state.clone());
                            }
                            Action::FetchLocalData { path} =>
                                self.fetch_local_data(Some(path), local_data_fetcher.clone(), local_tx.clone()).await,
                            Action::FetchS3Data { bucket, prefix } => {
                                state.set_s3_loading(true);
                                let _ = self.state_tx.send(state.clone());
                                let s3_data_fetcher = Self::get_current_s3_fetcher(&state);
                                self.fetch_s3_data(bucket, prefix, s3_data_fetcher, s3_tx.clone()).await
                            }
                            Action::ListS3DataRecursiveForItem { item } => {
                                state.set_s3_list_recursive_loading(true);
                                let _ = self.state_tx.send(state.clone());
                                let s3_data_fetcher = Self::get_current_s3_fetcher(&state);
                                self.list_s3_data_recursive(item, s3_data_fetcher, s3_full_list_tx.clone()).await
                            }
                            Action::MoveBackLocal => self.move_back_local_data(state.current_local_path.clone(), local_data_fetcher.clone(), local_tx.clone()).await,
                            Action::SelectS3Item { item} => {
                                state.add_s3_selected_item(item);
                                let _ = self.state_tx.send(state.clone());
                            },
                            Action::UnselectS3Item { item} => {
                                state.remove_s3_selected_item(item);
                                let _ = self.state_tx.send(state.clone());
                            },
                            Action::SelectLocalItem { item} => {
                                state.add_local_selected_item(item);
                                let _ = self.state_tx.send(state.clone());
                            },
                            Action::UnselectLocalItem { item } => {
                                state.remove_local_selected_item(item);
                                let _ = self.state_tx.send(state.clone());
                            },
                            Action::RunTransfers => {
                                state.remove_already_transferred_items();

                                // Enqueue downloads and get updated items with job_ids
                                let updated_s3_items = Self::enqueue_downloads(
                                    &transfer_manager,
                                    state.s3_selected_items.clone(),
                                    &job_item_map,
                                ).await;

                                // Update state with job_ids
                                for updated_item in updated_s3_items {
                                    state.update_s3_item_job_id(&updated_item);
                                }

                                // Enqueue uploads and get updated items with job_ids
                                let updated_local_items = Self::enqueue_uploads(
                                    &transfer_manager,
                                    state.local_selected_items.clone(),
                                    &job_item_map,
                                ).await;

                                // Update state with job_ids
                                for updated_item in updated_local_items {
                                    state.update_local_item_job_id(&updated_item);
                                }

                                self.state_tx.send(state.clone())?;
                            },
                            Action::SelectCurrentS3Creds { item} => {
                                state.set_current_s3_creds(item);
                                let _ = self.state_tx.send(state.clone());
                                let s3_data_fetcher = Self::get_current_s3_fetcher(&state);
                                self.fetch_s3_data(None, None, s3_data_fetcher, s3_tx.clone()).await;
                            },
                            Action::DeleteS3Item { item} => {
                                let s3_data_fetcher = Self::get_current_s3_fetcher(&state);
                                tracing::info!("deleting s3 item...{:?}", item.clone());
                                self.delete_s3_data(item.clone(), s3_data_fetcher.clone(), s3_deleted_tx.clone()).await;
                                if item.is_bucket {
                                    self.fetch_s3_data(None, None, s3_data_fetcher, s3_tx.clone()).await;
                                } else {
                                    self.fetch_s3_data(item.bucket, None, s3_data_fetcher, s3_tx.clone()).await;
                                }
                            },
                            Action::DeleteLocalItem {item} => {
                                state.remove_local_selected_item(item.clone());
                                let _ = self.state_tx.send(state.clone());
                                self.delete_local_data(item.clone(), local_data_fetcher.clone(), local_deleted_tx.clone()).await;
                                self.fetch_local_data(Some(item.path.clone()), local_data_fetcher.clone(), local_tx.clone()).await;
                            },
                            Action::CreateBucket {name} => {
                                let s3_data_fetcher = Self::get_current_s3_fetcher(&state);
                                tracing::info!("creating s3 bucket...{:?}", name.clone());
                                self.create_bucket(name.clone(), s3_data_fetcher.clone(), create_bucket_tx.clone()).await;
                                self.fetch_s3_data(None, None, s3_data_fetcher, s3_tx.clone()).await;
                            },
                            Action::ClearDeletionErrors => {
                                state.s3_delete_error = None;
                                state.local_delete_error = None;
                                state.create_bucket_error = None;
                                self.state_tx.send(state.clone())?;
                            },
                            Action::SortS3 { column } => {
                                state.sort_s3_data(column);
                                self.state_tx.send(state.clone())?;
                            },
                            Action::SortLocal { column } => {
                                state.sort_local_data(column);
                                self.state_tx.send(state.clone())?;
                            },
                            Action::SetSearchMode { active } => {
                                state.set_search_mode(active);
                                self.state_tx.send(state.clone())?;
                            },
                            Action::SetSearchQuery { query } => {
                                state.set_search_query(query);
                                self.state_tx.send(state.clone())?;
                            },
                            Action::ClearSearch => {
                                state.clear_search();
                                self.state_tx.send(state.clone())?;
                            },
                            Action::PauseTransfer { job_id } => {
                                if let Err(e) = transfer_manager.pause(job_id).await {
                                    tracing::warn!("Failed to pause transfer {}: {}", job_id, e);
                                } else {
                                    // Update state to reflect paused status
                                    state.set_transfer_paused(job_id);
                                    self.state_tx.send(state.clone())?;
                                }
                            },
                            Action::ResumeTransfer { job_id } => {
                                if let Err(e) = transfer_manager.resume(job_id).await {
                                    tracing::warn!("Failed to resume transfer {}: {}", job_id, e);
                                } else {
                                    // Update state to reflect resumed (pending) status
                                    state.set_transfer_resumed(job_id);
                                    self.state_tx.send(state.clone())?;
                                }
                            },
                            Action::CancelTransfer { job_id } => {
                                if let Err(e) = transfer_manager.cancel(job_id).await {
                                    tracing::warn!("Failed to cancel transfer {}: {}", job_id, e);
                                } else {
                                    // Update state to mark item as cancelled
                                    state.set_transfer_cancelled(job_id);
                                    // Remove from job map
                                    job_item_map.lock().await.remove(&job_id);
                                    self.state_tx.send(state.clone())?;
                                }
                            },
                        },
                        Some(result) = transfer_result_rx.recv() => {
                            match result {
                                TransferResult::UploadComplete(item) => {
                                    let dest_bucket = item.destination_bucket.clone();
                                    state.update_selected_local_transfers(item);
                                    // Auto-refresh S3 view when all uploads to a bucket complete
                                    if state.all_uploads_complete_for_bucket(&dest_bucket) {
                                        let s3_data_fetcher = Self::get_current_s3_fetcher(&state);
                                        self.fetch_s3_data(Some(dest_bucket), None, s3_data_fetcher, s3_tx.clone()).await;
                                    }
                                    self.state_tx.send(state.clone())?;
                                },
                                TransferResult::UploadFailed(item, _error) => {
                                    state.update_selected_local_transfers(item);
                                    self.state_tx.send(state.clone())?;
                                },
                                TransferResult::DownloadComplete(item) => {
                                    let dest_dir = item.destination_dir.clone();
                                    state.update_selected_s3_transfers(item);
                                    // Auto-refresh local view when all downloads to a directory complete
                                    if state.all_downloads_complete_for_directory(&dest_dir) {
                                        self.fetch_local_data(Some(dest_dir), local_data_fetcher.clone(), local_tx.clone()).await;
                                    }
                                    self.state_tx.send(state.clone())?;
                                },
                                TransferResult::DownloadFailed(item, _error) => {
                                    state.update_selected_s3_transfers(item);
                                    self.state_tx.send(state.clone())?;
                                },
                            }
                        },
                        Some((bucket, prefix, data)) = s3_rx.recv() => {
                            state.update_buckets(bucket, prefix, data);
                            self.state_tx.send(state.clone())?;
                        },
                        Some((_bucket, _prefix, data)) = s3_full_list_rx.recv() => {
                            state.update_s3_recursive_list(data);
                            self.state_tx.send(state.clone())?;
                        },
                        Some((path, files)) = local_rx.recv() => {
                            state.update_files(path, files);
                            self.state_tx.send(state.clone())?;
                        },
                        Some(item) = upload_rx.recv() => {
                            if state.active_page == ActivePage::Transfers {
                                state.update_progress_on_selected_local_item(&item);
                                self.state_tx.send(state.clone())?;
                            }
                        },
                        Some(item) = download_rx.recv() => {
                            if state.active_page == ActivePage::Transfers {
                                state.update_progress_on_selected_s3_item(&item);
                                self.state_tx.send(state.clone())?;
                            }
                        },
                        Some(error) = local_deleted_rx.recv() => {
                            state.set_local_delete_error(error);
                            self.state_tx.send(state.clone())?;
                        },
                        Some(error) = s3_deleted_rx.recv() => {
                            state.set_s3_delete_error(error);
                            self.state_tx.send(state.clone())?;
                        },
                        Some(error) = create_bucket_rx.recv() => {
                            state.set_create_bucket_error(error);
                            self.state_tx.send(state.clone())?;
                        }

                // Catch and handle interrupt signal to gracefully shutdown
                Ok(interrupted) = interrupt_rx.recv() => {
                    break interrupted;
                }
            }
        };

        Ok(result)
    }
}

/// Info about a transfer job for mapping back to the original item
#[derive(Debug, Clone)]
enum TransferJobInfo {
    Upload { item: LocalSelectedItem },
    Download { item: S3SelectedItem },
}
