use std::path::PathBuf;
use clap::Parser;

use crate::utils::version;

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    /// Path to the credentials file
    #[arg(long)]
    pub creds_file: Option<PathBuf>,
}
