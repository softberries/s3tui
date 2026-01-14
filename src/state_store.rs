//! This module provides functionality for interactions between UI and state
use crate::model::action::Action;
use crate::model::download_progress_item::DownloadProgressItem;
use crate::model::has_children::flatten_items;
use crate::model::local_data_item::LocalDataItem;
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_data_item::S3DataItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::state::{ActivePage, State};
use crate::model::upload_progress_item::UploadProgressItem;
use crate::services::local_data_fetcher::LocalDataFetcher;
use crate::services::s3_data_fetcher::S3DataFetcher;
use crate::settings::file_credentials::FileCredential;
use crate::termination::{Interrupted, Terminator};
use color_eyre::eyre;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{broadcast, mpsc};
use tokio::sync::Semaphore;

/// Maximum simultaneous uploads/downloads
static S3_OPERATIONS_CONCURRENCY_LEVEL: usize = 8;

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
    async fn download_data(
        &self,
        s3_data_fetcher: &S3DataFetcher,
        s3_selected_items: Vec<S3SelectedItem>,
        selected_s3_transfers_tx: UnboundedSender<S3SelectedItem>,
        download_tx: UnboundedSender<DownloadProgressItem>,
    ) {
        let items_with_children = flatten_items(s3_selected_items);
        let semaphore = Arc::new(Semaphore::new(S3_OPERATIONS_CONCURRENCY_LEVEL)); // Adjust the number based on system capabilities
        for item in items_with_children {
            if !item.is_bucket && !item.is_directory {
                let tx = selected_s3_transfers_tx.clone();
                let down_tx = download_tx.clone();
                let fetcher = s3_data_fetcher.clone();
                let semaphore = semaphore.clone();
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    match fetcher.download_item(item.clone(), down_tx).await {
                        Ok(_) => {
                            if tx.send(item.clone()).is_err() {
                                tracing::error!("Failed to send downloaded item");
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to download data: {}",  e);
                            let orig_item = item.clone();
                            let errored_item = S3SelectedItem {
                                error: Some(e.to_string()),
                                transferred: false,
                                progress: 0f64,
                                ..orig_item
                            };
                            if tx.send(errored_item).is_err() {
                                tracing::error!("Failed to send item in error");
                            }
                        }
                    }
                });
            }
        }
    }

    async fn upload_data(
        &self,
        s3_data_fetcher: &S3DataFetcher,
        local_selected_items: Vec<LocalSelectedItem>,
        selected_local_transfers_tx: UnboundedSender<LocalSelectedItem>,
        upload_tx: UnboundedSender<UploadProgressItem>,
    ) {
        let items_with_children = flatten_items(local_selected_items);
        let semaphore = Arc::new(Semaphore::new(S3_OPERATIONS_CONCURRENCY_LEVEL)); // Adjust the number based on system capabilities
        for item in items_with_children {
            if !item.is_directory {
                let local_tx = selected_local_transfers_tx.clone();
                let up_tx = upload_tx.clone();
                let fetcher = s3_data_fetcher.clone();
                let semaphore = semaphore.clone();
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    match fetcher.upload_item(item.clone(), up_tx).await {
                        Ok(_) => {
                            if local_tx.send(item.clone()).is_err() {
                                tracing::error!("Failed to send uploaded item");
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to upload data: {}", e);
                            let orig_item = item.clone();
                            let errored_item = LocalSelectedItem {
                                error: Some(e.to_string()),
                                transferred: false,
                                progress: 0f64,
                                ..orig_item
                            };
                            if local_tx.send(errored_item).is_err() {
                                tracing::error!("Failed to send item in error");
                            }
                        }
                    }
                });
            }
        }
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
        local_deleted_tx: UnboundedSender<Option<String>>,
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
                        let _ = local_deleted_tx.send(Some(e.to_string()));
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
                        let _ = local_deleted_tx.send(Some(e.to_string()));
                    }
                }
            });
        }
    }

    async fn delete_s3_data(
        &self,
        item: S3SelectedItem,
        s3_data_fetcher: S3DataFetcher,
        s3_delete_tx: UnboundedSender<Option<String>>,
    ) {
        let items_with_children = flatten_items(vec![item]);
        let semaphore = Arc::new(Semaphore::new(S3_OPERATIONS_CONCURRENCY_LEVEL)); // Adjust the number based on system capabilities
        for item in items_with_children {
            if !item.is_directory {
                let delete_tx = s3_delete_tx.clone();
                let fetcher = s3_data_fetcher.clone();
                let semaphore = semaphore.clone();
                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    match fetcher
                        .delete_data(
                            item.is_bucket,
                            item.bucket.clone(),
                            item.path.clone().unwrap_or(item.name.clone()),
                            item.is_directory,
                        )
                        .await
                    {
                        Ok(data) => {
                            let _ = delete_tx.send(data);
                        }
                        Err(e) => {
                            tracing::error!("Failed to delete S3 data: {}", e);
                            let _ =
                                delete_tx.send(Some(format!("Failed to delete S3 data: {}", e)));
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
        create_bucket_tx: UnboundedSender<Option<String>>,
    ) {
        tokio::spawn(async move {
            match s3_data_fetcher
                .create_bucket(name.clone(), s3_data_fetcher.default_region.clone())
                .await
            {
                Ok(data) => {
                    let _ = create_bucket_tx.send(data);
                }
                Err(e) => {
                    tracing::error!("Failed to create S3 bucket: {}", e);
                    let _ = create_bucket_tx.send(Some(format!("Failed to create bucket: {}", e)));
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
        let (s3_deleted_tx, mut s3_deleted_rx) = mpsc::unbounded_channel::<Option<String>>();
        let (local_tx, mut local_rx) = mpsc::unbounded_channel::<(String, Vec<LocalDataItem>)>();
        let (local_deleted_tx, mut local_deleted_rx) = mpsc::unbounded_channel::<Option<String>>();
        let (selected_s3_transfers_tx, mut selected_s3_transfers_rx) =
            mpsc::unbounded_channel::<S3SelectedItem>();
        let (selected_local_transfers_tx, mut selected_local_transfers_rx) =
            mpsc::unbounded_channel::<LocalSelectedItem>();
        let (upload_tx, mut upload_rx) = mpsc::unbounded_channel::<UploadProgressItem>();
        let (download_tx, mut download_rx) = mpsc::unbounded_channel::<DownloadProgressItem>();
        let (create_bucket_tx, mut create_bucket_rx) = mpsc::unbounded_channel::<Option<String>>();

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
                                let st = state.clone();
                                let s3_data_fetcher = Self::get_current_s3_fetcher(&st);
                                self.download_data(&s3_data_fetcher, st.s3_selected_items, selected_s3_transfers_tx.clone(), download_tx.clone()).await;
                                self.upload_data(&s3_data_fetcher, st.local_selected_items, selected_local_transfers_tx.clone(), upload_tx.clone()).await;
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
                                state.s3_delete_state = None;
                                state.local_delete_state = None;
                                state.create_bucket_state = None;
                                self.state_tx.send(state.clone())?;
                            }
                        },
                        Some(item) = selected_s3_transfers_rx.recv() => {
                            let dest_dir = item.destination_dir.clone();
                            state.update_selected_s3_transfers(item);
                            // Auto-refresh local view when all downloads to a directory complete
                            if state.all_downloads_complete_for_directory(&dest_dir) {
                                self.fetch_local_data(Some(dest_dir), local_data_fetcher.clone(), local_tx.clone()).await;
                            }
                            self.state_tx.send(state.clone())?;
                        },
                        Some(item) = selected_local_transfers_rx.recv() => {
                            let dest_bucket = item.destination_bucket.clone();
                            state.update_selected_local_transfers(item);
                            // Auto-refresh S3 view when all uploads to a bucket complete
                            if state.all_uploads_complete_for_bucket(&dest_bucket) {
                                let s3_data_fetcher = Self::get_current_s3_fetcher(&state);
                                self.fetch_s3_data(Some(dest_bucket), None, s3_data_fetcher, s3_tx.clone()).await;
                            }
                            self.state_tx.send(state.clone())?;
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
                        Some(error_str) = local_deleted_rx.recv() => {
                            state.set_local_delete_error(error_str);
                            self.state_tx.send(state.clone())?;
                        },
                        Some(error_str) = s3_deleted_rx.recv() => {
                            state.set_s3_delete_error(error_str);
                            self.state_tx.send(state.clone())?;
                        },
                        Some(error_str) = create_bucket_rx.recv() => {
                            state.set_create_bucket_error(error_str);
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
