use std::path::Path;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use crate::services::local_data_fetcher::LocalDataFetcher;
use crate::services::s3_data_fetcher::S3DataFetcher;
use crate::model::action::Action;
use crate::model::download_progress_item::DownloadProgressItem;
use crate::model::local_data_item::LocalDataItem;
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::upload_progress_item::UploadProgressItem;
use crate::model::s3_data_item::S3DataItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::state::{ActivePage, State};
use crate::settings::file_credentials::FileCredential;
use crate::termination::{Interrupted, Terminator};

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
    async fn download_data(&self, s3_data_fetcher: &S3DataFetcher, s3_selected_items: Vec<S3SelectedItem>, selected_s3_transfers_tx: UnboundedSender<S3SelectedItem>, download_tx: UnboundedSender<DownloadProgressItem>) {
        for item in s3_selected_items {
            let tx = selected_s3_transfers_tx.clone();
            let down_tx = download_tx.clone();
            let fetcher = s3_data_fetcher.clone();
            tokio::spawn(async move {
                match fetcher.download_item(item.clone(), down_tx).await {
                    Ok(_) => {
                        if tx.send(item.clone()).is_err() {
                            eprintln!("Failed to send downloaded item");
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to download data: {}", e);
                    }
                }
            });
        }
    }
    async fn upload_data(&self, s3_data_fetcher: &S3DataFetcher, local_selected_items: Vec<LocalSelectedItem>, selected_local_transfers_tx: UnboundedSender<LocalSelectedItem>, upload_tx: UnboundedSender<UploadProgressItem>) {
        for item in local_selected_items {
            let local_tx = selected_local_transfers_tx.clone();
            let up_tx = upload_tx.clone();
            let fetcher = s3_data_fetcher.clone();
            tokio::spawn(async move {
                match fetcher.upload_item(item.clone(), up_tx).await {
                    Ok(_) => {
                        if local_tx.send(item.clone()).is_err() {
                            eprintln!("Failed to send uploaded item");
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to upload data: {}", e);
                    }
                }
            });
        }
    }
    async fn fetch_s3_data(&self, bucket: Option<String>, prefix: Option<String>, s3_data_fetcher: S3DataFetcher, s3_tx: UnboundedSender<(Option<String>, Option<String>, Vec<S3DataItem>)>) {
        tokio::spawn(async move {
            match s3_data_fetcher.list_current_location(bucket.clone(), prefix.clone()).await {
                Ok(data) => {
                    let _ = s3_tx.send((bucket.clone(), prefix.clone(), data));
                }
                Err(e) => {
                    eprintln!("Failed to fetch S3 data: {}", e);
                }
            }
        });
    }
    async fn fetch_local_data(&self, path: Option<String>, local_data_fetcher: LocalDataFetcher, local_tx: UnboundedSender<(String, Vec<LocalDataItem>)>) {
        tokio::spawn(async move {
            match local_data_fetcher.read_directory(path.clone()).await {
                Ok(data) => {
                    let _ = local_tx.send((path.clone().unwrap_or("/".to_string()), data));
                }
                Err(e) => {
                    eprintln!("Failed to fetch local data: {}", e);
                    // Handle error, maybe retry or send error state
                }
            }
        });
    }
    async fn move_back_local_data(&self, current_path: String, local_data_fetcher: LocalDataFetcher, local_tx: UnboundedSender<(String, Vec<LocalDataItem>)>) {
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
                    eprintln!("Failed to fetch local data: {}", e);
                    // Handle error, maybe retry or send error state
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
    ) -> anyhow::Result<Interrupted> {
        let local_data_fetcher = LocalDataFetcher::new();
        let mut state = State::new(creds.clone());
        let s3_data_fetcher = Self::get_current_s3_fetcher(&state);
        state.set_s3_loading(true);
        state.set_current_local_path(dirs::home_dir().unwrap().as_path().to_string_lossy().to_string());

        let (s3_tx, mut s3_rx) = mpsc::unbounded_channel::<(Option<String>, Option<String>, Vec<S3DataItem>)>();
        let (local_tx, mut local_rx) = mpsc::unbounded_channel::<(String, Vec<LocalDataItem>)>();
        let (selected_s3_transfers_tx, mut selected_s3_transfers_rx) = mpsc::unbounded_channel::<S3SelectedItem>();
        let (selected_local_transfers_tx, mut selected_local_transfers_rx) = mpsc::unbounded_channel::<LocalSelectedItem>();
        let (upload_tx, mut upload_rx) = mpsc::unbounded_channel::<UploadProgressItem>();
        let (download_tx, mut download_rx) = mpsc::unbounded_channel::<DownloadProgressItem>();

        self.fetch_s3_data(None, None, s3_data_fetcher.clone(), s3_tx.clone()).await;
        self.fetch_local_data(Some(dirs::home_dir().unwrap().as_path().to_string_lossy().to_string()), local_data_fetcher.clone(), local_tx.clone()).await;

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
                            self.fetch_s3_data(bucket, prefix, s3_data_fetcher, s3_tx.clone()).await},
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
                        }
                    },
                    Some(item) = selected_s3_transfers_rx.recv() => {
                        state.update_selected_s3_transfers(item);
                        self.state_tx.send(state.clone())?;
                    },
                    Some(item) = selected_local_transfers_rx.recv() => {
                        state.update_selected_local_transfers(item);
                        self.state_tx.send(state.clone())?;
                    },
                    Some((bucket, prefix, data)) = s3_rx.recv() => {
                        state.update_buckets(bucket, prefix, data);
                        self.state_tx.send(state.clone())?;
                    },
                    Some((path, files)) = local_rx.recv() => {
                        state.update_files(path, files);
                        self.state_tx.send(state.clone())?;
                    },
                    Some(item) = upload_rx.recv() => {
                        if state.active_page == ActivePage::Transfers {
                            state.update_progress_on_selected_local_item(item);
                            self.state_tx.send(state.clone())?;
                        }
                    },
                    Some(item) = download_rx.recv() => {
                        if state.active_page == ActivePage::Transfers {
                            state.update_progress_on_selected_s3_item(item);
                            self.state_tx.send(state.clone())?;
                        }
                    },

            // Catch and handle interrupt signal to gracefully shutdown
            Ok(interrupted) = interrupt_rx.recv() => {
                break interrupted;
            }
        }
        };

        Ok(result)
    }
}
