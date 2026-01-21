//! # S3 TUI Application
//!
//! This crate provides a simple `S3` client for managing your resources in s3 buckets.
//! `s3tui` is a powerful terminal-based application that enables seamless file transfers between
//! your local machine and multiple AWS S3 accounts.
//! Crafted with the `ratatui` Rust TUI framework, `s3tui` provides a robust user interface for managing
//! uploads and downloads simultaneously in both directions,
//! enhancing your productivity with `S3` services.

#![forbid(unsafe_code)]
mod cli;
mod components;
mod state_store;
mod termination;
mod ui_manager;

// Re-use library modules
use s3tui::model;
use s3tui::services;
use s3tui::settings;
use s3tui::utils;

use settings::file_credentials;
use crate::state_store::StateStore;
use crate::termination::{create_termination, Interrupted};
use crate::ui_manager::UiManager;
use utils::{initialize_logging, initialize_panic_handler};
use clap::Parser;
use cli::Cli;
use color_eyre::eyre;

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
            eprintln!("No minio file found, add minio file into your $S3TUI_DATA/creds directory in your home directory.");
        }
    } else {
        eprintln!("Problem reading credential files, add at least one minio file into $S3TUI_DATA/creds in your home directory.");
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
