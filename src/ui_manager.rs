//! This module provides functionality for passing state to displayed components
use std::{
    io::{self, Stdout},
    time::Duration,
};

use color_eyre::eyre;
use color_eyre::eyre::Context;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::{
    broadcast,
    mpsc::{self, Receiver, UnboundedReceiver},
};
use tokio_stream::StreamExt;

use super::components::app_router::AppRouter;
use crate::components::component::{Component, ComponentRender};
use crate::model::action::Action;
use crate::model::state::State;
use crate::termination::Interrupted;

const RENDERING_TICK_RATE: Duration = Duration::from_millis(250);

/// Handles cross-term events together with state and interrupt actions
pub struct UiManager {
    action_tx: mpsc::UnboundedSender<Action>,
}

impl UiManager {
    pub fn new() -> (Self, UnboundedReceiver<Action>) {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        (Self { action_tx }, action_rx)
    }
    pub async fn main_loop(
        self,
        mut state_rx: Receiver<State>,
        mut interrupt_rx: broadcast::Receiver<Interrupted>,
    ) -> eyre::Result<Interrupted> {
        // consume the first state to initialize the ui app
        let mut app_router = {
            let state = state_rx.recv().await.unwrap();

            AppRouter::new(&state, self.action_tx.clone())
        };

        let mut terminal = setup_terminal()?;
        let mut ticker = tokio::time::interval(RENDERING_TICK_RATE);
        let mut crossterm_events = EventStream::new();

        let result: eyre::Result<Interrupted> = loop {
            tokio::select! {
                // Tick to terminate the select every N milliseconds
                _ = ticker.tick() => (),
                // Catch and handle crossterm events
               maybe_event = crossterm_events.next() => match maybe_event {
                    Some(Ok(Event::Key(key)))  => {
                        app_router.handle_key_event(key);
                    },
                    None => break Ok(Interrupted::UserInt),
                    _ => (),
                },
                // Handle state updates
                Some(state) = state_rx.recv() => {
                    app_router = app_router.move_with_state(&state);
                },
                // Catch and handle interrupt signal to gracefully shutdown
                Ok(interrupted) = interrupt_rx.recv() => {
                    break Ok(interrupted);
                }
            }
            //render(frame, ()))
            if let Err(err) = terminal
                .draw(|frame| app_router.render(frame, ()))
                .context("could not render to the terminal")
            {
                break Err(err);
            }
        };

        restore_terminal(&mut terminal)?;

        result
    }
}

fn setup_terminal() -> eyre::Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = io::stdout();

    enable_raw_mode()?;

    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> eyre::Result<()> {
    disable_raw_mode()?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(terminal.show_cursor()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ui_manager_new() {
        let (_ui_manager, mut action_rx) = UiManager::new();
        assert!(
            action_rx.try_recv().is_err(),
            "Should start with no pending actions"
        );
    }
}
