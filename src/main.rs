mod termination;
mod state_store;
mod ui_manager;
mod model;
mod components;
mod fetchers;
mod settings;

use crate::settings::file_credentials;
use crate::state_store::StateStore;
use crate::termination::{create_termination, Interrupted};
use crate::ui_manager::UiManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
            eprintln!("No credentials file found, add credentials file into .s3tui/creds in your home directory.")
        }
    } else {
        eprintln!("Problem reading credential files, add at least one credentials file into .s3tui/creds in your home directory.")
    }


    if let Ok(reason) = interrupt_rx.recv().await {
        match reason {
            Interrupted::UserInt => println!("exited per user request"),
            Interrupted::OsSigInt => println!("exited because of an os sig int"),
        }
    } else {
        println!("exited because of an unexpected error");
    }

    Ok(())
}