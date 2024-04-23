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
    async fn fetch_s3_data(&self, bucket: Option<String>, prefix: Option<String>, s3_data_fetcher: S3DataFetcher, s3_tx: UnboundedSender<Vec<S3DataItem>>) {
        tokio::spawn(async move {
            match s3_data_fetcher.list_current_location(bucket, prefix).await {
                Ok(data) => {
                    let _ = s3_tx.send(data);
                }
                Err(e) => {
                    eprintln!("Failed to fetch S3 data: {}", e);
                }
            }
        });
    }
    async fn fetch_local_data(&self, path: Option<String>, local_data_fetcher: LocalDataFetcher, local_tx: UnboundedSender<Vec<LocalDataItem>>) {
        tokio::spawn(async move {
            match local_data_fetcher.read_directory(path).await {
                Ok(data) => {
                    let _ = local_tx.send(data);
                }
                Err(e) => {
                    eprintln!("Failed to fetch local data: {}", e);
                    // Handle error, maybe retry or send error state
                }
            }
        });
    }
    async fn move_back_local_data(&self, local_data_fetcher: LocalDataFetcher, local_tx: UnboundedSender<Vec<LocalDataItem>>) {
        tokio::spawn(async move {
            match local_data_fetcher.read_parent_directory().await {
                Ok(data) => {
                    let _ = local_tx.send(data);
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

        let (s3_tx, mut s3_rx) = mpsc::unbounded_channel::<Vec<S3DataItem>>();
        let (local_tx, mut local_rx) = mpsc::unbounded_channel::<Vec<LocalDataItem>>();
        self.fetch_s3_data(None, None, s3_data_fetcher.clone(), s3_tx.clone()).await;
        self.fetch_local_data(None, local_data_fetcher.clone(), local_tx.clone()).await;

        // the initial state once
        self.state_tx.send(state.clone())?;

        let mut _ticker = tokio::time::interval(Duration::from_secs(1));

        let result = loop {
            tokio::select! {
                    Some(bucket_list) = s3_rx.recv() => {
                        // Process the list of buckets
                        state.update_buckets(bucket_list);
                        self.state_tx.send(state.clone())?;
                    },
                    Some(files) = local_rx.recv() => {
                        // Process the list of buckets
                        state.update_files(files);
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
                        },
                        Action::FetchLocalData { path} =>
                            self.fetch_local_data(Some(path), local_data_fetcher.clone(), local_tx.clone()).await,
                        Action::FetchS3Data { bucket, prefix } => {
                            println!("Bucket: {:?}, prefix: {:?}", bucket.clone(), prefix.clone());
                            self.fetch_s3_data(bucket, prefix, s3_data_fetcher.clone(), s3_tx.clone()).await},
                        Action::MoveBackLocal => self.move_back_local_data(local_data_fetcher.clone(), local_tx.clone()).await,
                        // Action::MoveBackS3 => self.move_back_s3_data(s3_data_fetcher.clone(), s3_tx.clone()).await
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
