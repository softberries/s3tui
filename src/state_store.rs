use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use crate::action::Action;
use crate::state::{ActivePage, State};
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
    pub async fn main_loop(
        self,
        mut terminator: Terminator,
        mut action_rx: UnboundedReceiver<Action>,
        mut interrupt_rx: broadcast::Receiver<Interrupted>,
    ) -> anyhow::Result<Interrupted> {
        // let mut state = State::default();
        let state = State::default();

        // the initial state once
        self.state_tx.send(state.clone())?;

        let mut _ticker = tokio::time::interval(Duration::from_secs(1));

        let result = loop {
            tokio::select! {
                    Some(action) = action_rx.recv() => match action {
                        Action::Exit => {
                            let _ = terminator.terminate(Interrupted::UserInt);

                            break Interrupted::UserInt;
                        },
                        Action::Navigate { page} =>
                            match page {
                                ActivePage::HelpPage => self.state_tx.send(State{active_page: ActivePage::HelpPage})?,
                                ActivePage::FileManagerPage => self.state_tx.send(State{active_page: ActivePage::FileManagerPage})?,
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
