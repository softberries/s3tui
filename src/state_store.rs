use std::path::Path;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use crate::fetchers::local_data_fetcher::LocalDataFetcher;
use crate::fetchers::s3_data_fetcher::S3DataFetcher;
use crate::model::action::Action;
use crate::model::local_data_item::LocalDataItem;
use crate::model::s3_data_item::S3DataItem;
use crate::model::state::{ActivePage, State};
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

    pub async fn main_loop(
        self,
        mut terminator: Terminator,
        mut action_rx: UnboundedReceiver<Action>,
        mut interrupt_rx: broadcast::Receiver<Interrupted>,
    ) -> anyhow::Result<Interrupted> {
        let s3_data_fetcher = S3DataFetcher::new();
        let local_data_fetcher = LocalDataFetcher::new();
        let mut state = State::default();
        state.set_s3_loading(true);
        state.set_current_local_path(dirs::home_dir().unwrap().as_path().to_string_lossy().to_string());

        let (s3_tx, mut s3_rx) = mpsc::unbounded_channel::<(Option<String>, Option<String>, Vec<S3DataItem>)>();
        let (local_tx, mut local_rx) = mpsc::unbounded_channel::<(String, Vec<LocalDataItem>)>();
        self.fetch_s3_data(None, None, s3_data_fetcher.clone(), s3_tx.clone()).await;
        self.fetch_local_data(Some(dirs::home_dir().unwrap().as_path().to_string_lossy().to_string()), local_data_fetcher.clone(), local_tx.clone()).await;

        // the initial state once
        self.state_tx.send(state.clone())?;

        let mut _ticker = tokio::time::interval(Duration::from_secs(1));

        let result = loop {
            tokio::select! {
                    Some((_bucket, _prefix, data)) = s3_rx.recv() => {
                        state.update_buckets(data);
                        self.state_tx.send(state.clone())?;
                    },
                    Some((path, files)) = local_rx.recv() => {
                        state.update_files(path, files);
                        self.state_tx.send(state.clone())?;
                    },
                    Some(action) = action_rx.recv() => match action {
                        Action::Exit => {
                            let _ = terminator.terminate(Interrupted::UserInt);

                            break Interrupted::UserInt;
                        },
                        Action::Navigate { page} =>
                            match page {
                                ActivePage::HelpPage => self.state_tx.send(State{active_page: ActivePage::HelpPage, ..state.clone()})?,
                                ActivePage::FileManagerPage => self.state_tx.send(State{active_page: ActivePage::FileManagerPage, ..state.clone()})?,
                                ActivePage::TransfersPage => self.state_tx.send(State{active_page: ActivePage::TransfersPage, ..state.clone()})?,
                        },
                        Action::FetchLocalData { path} =>
                            self.fetch_local_data(Some(path), local_data_fetcher.clone(), local_tx.clone()).await,
                        Action::FetchS3Data { bucket, prefix } => {
                            state.set_s3_loading(true);
                            let _ = self.state_tx.send(state.clone());
                            self.fetch_s3_data(bucket, prefix, s3_data_fetcher.clone(), s3_tx.clone()).await},
                        Action::MoveBackLocal => self.move_back_local_data(state.current_local_path.clone(), local_data_fetcher.clone(), local_tx.clone()).await,
                        Action::SelectS3Item { item} => {
                            state.add_s3_selected_item(item);
                            let _ = self.state_tx.send(state.clone());
                        },
                        Action::UnselectS3Item { item} => {
                            state.remove_s3_selected_item(item);
                            let _ = self.state_tx.send(state.clone());
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
