#![forbid(unsafe_code)]
mod termination;
mod state_store;
mod ui_manager;
mod model;
mod components;
mod services;
mod settings;
mod utils;
mod cli;

use crate::settings::file_credentials;
use crate::state_store::StateStore;
use crate::termination::{create_termination, Interrupted};
use crate::ui_manager::UiManager;
use crate::{
    utils::{initialize_logging, initialize_panic_handler},
};
use clap::Parser;
use color_eyre::eyre;
use cli::Cli;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    initialize_logging()?;
    initialize_panic_handler()?;
    let _args = Cli::parse();
    let (terminator, mut interrupt_rx) = create_termination();
    let (state_store, state_rx) = StateStore::new();
    let (ui_manager, action_rx) = UiManager::new();

    if let Ok(creds) = file_credentials::load_credentials() {
        if !creds.is_empty() {
            tokio::try_join!(
                state_store.main_loop(terminator, action_rx, interrupt_rx.resubscribe(), creds),
                ui_manager.main_loop(state_rx, interrupt_rx.resubscribe()),
             )?;
        } else {
            eprintln!("No credentials file found, add credentials file into your $S3TUI_DATA/creds directory in your home directory.");
        }
    } else {
        eprintln!("Problem reading credential files, add at least one credentials file into $S3TUI_DATA/creds in your home directory.");
    }


    if let Ok(reason) = interrupt_rx.recv().await {
        match reason {
            Interrupted::UserInt => tracing::info!("exited per user request"),
            Interrupted::OsSigInt => tracing::info!("exited because of an os sig int"),
        }
    } else {
        tracing::error!("exited because of an unexpected error");
    }

    Ok(())
}