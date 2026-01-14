use color_eyre::eyre;
use crossterm::cursor;
use crossterm::event::{DisableBracketedPaste, DisableMouseCapture};
use crossterm::terminal::LeaveAlternateScreen;
use std::io::LineWriter;
use std::path::PathBuf;

use directories::ProjectDirs;
use lazy_static::lazy_static;
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    self, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, Layer,
};

const VERSION_MESSAGE: &str = env!("CARGO_PKG_VERSION");

lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
    pub static ref DATA_FOLDER: Option<PathBuf> =
        std::env::var(format!("{}_DATA", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
    pub static ref CONFIG_FOLDER: Option<PathBuf> =
        std::env::var(format!("{}_CONFIG", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
    pub static ref LOG_ENV: String = format!("{}_LOGLEVEL", PROJECT_NAME.clone());
    pub static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
}
pub type IO = std::io::Stdout;
pub fn io() -> IO {
    std::io::stdout()
}
fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "softberries", env!("CARGO_PKG_NAME"))
}

fn stop() -> eyre::Result<()> {
    if crossterm::terminal::is_raw_mode_enabled()? {
        crossterm::execute!(io(), DisableBracketedPaste)?;
        crossterm::execute!(io(), DisableMouseCapture)?;
        crossterm::execute!(io(), LeaveAlternateScreen, cursor::Show)?;
        crossterm::terminal::disable_raw_mode()?;
    }
    Ok(())
}

/// Eyre hook to display a bit more user-friendly messages in case of panic
pub fn initialize_panic_handler() -> eyre::Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();
    eyre_hook.install()?;
    std::panic::set_hook(Box::new(move |panic_info| {
        // let (ui_manager, action_rx) = UiManager::new();
        let _ = stop();
        //
        // if let Ok(mut t) = crate::tui::Tui::new() {
        //     if let Err(r) = t.exit() {
        //         error!("Unable to exit Terminal: {:?}", r);
        //     }
        // }
        #[cfg(not(debug_assertions))]
        {
            use human_panic::{handle_dump, print_msg, Metadata};
            let meta = Metadata {
                version: env!("CARGO_PKG_VERSION").into(),
                name: env!("CARGO_PKG_NAME").into(),
                authors: env!("CARGO_PKG_AUTHORS").replace(':', ", ").into(),
                homepage: env!("CARGO_PKG_HOMEPAGE").into(),
            };

            let file_path = handle_dump(&meta, panic_info);
            // prints human-panic message
            print_msg(file_path, &meta)
                .expect("human-panic: printing error message to console failed");
            eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color-eyre stack trace to stderr
        }
        let msg = format!("{}", panic_hook.panic_report(panic_info));
        log::error!("Error: {}", strip_ansi_escapes::strip_str(msg));

        #[cfg(debug_assertions)]
        {
            // Better Panic stacktrace that is only enabled when debugging.
            better_panic::Settings::auto()
                .most_recent_first(false)
                .lineno_suffix(true)
                .verbosity(better_panic::Verbosity::Full)
                .create_panic_handler()(panic_info);
        }

        std::process::exit(libc::EXIT_FAILURE);
    }));
    Ok(())
}

/// Gets the user specified data directory
/// Eventually takes the system default location
pub fn get_data_dir() -> PathBuf {
    let directory = if let Some(s) = DATA_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".data")
    };
    directory
}

/// Gets the user specified configuration directory
/// Eventually takes the system default location
pub fn get_config_dir() -> PathBuf {
    let directory = if let Some(s) = CONFIG_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".config")
    };
    directory
}

/// Sets up logging capabilities for the application
/// The logs are stored in the data directory
pub fn initialize_logging() -> eyre::Result<()> {
    let directory = get_data_dir();
    std::fs::create_dir_all(directory.clone())?;
    let log_path = directory.join(LOG_FILE.clone());
    let log_file = std::fs::File::create(log_path)?;
    // Wrap in LineWriter to ensure logs are flushed after each line,
    // then in Mutex for thread-safe access required by tracing-subscriber
    let log_file = std::sync::Mutex::new(LineWriter::new(log_file));
    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG")
            .or_else(|_| std::env::var(LOG_ENV.clone()))
            .unwrap_or_else(|_| format!("{}=info", env!("CARGO_CRATE_NAME"))),
    );
    // std::env::set_var("RUST_LOG", "error");
    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env());
    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .init();
    Ok(())
}

/// Similar to the `std::dbg!` macro, but generates `tracing` events rather
/// than printing to stdout.
///
/// By default, the verbosity level for the generated events is `DEBUG`, but
/// this can be customized.
#[macro_export]
macro_rules! trace_dbg {
    (target: $target:expr, level: $level:expr, $ex:expr) => {{
        match $ex {
            value => {
                tracing::event!(target: $target, $level, ?value, stringify!($ex));
                value
            }
        }
    }};
    (level: $level:expr, $ex:expr) => {
        trace_dbg!(target: module_path!(), level: $level, $ex)
    };
    (target: $target:expr, $ex:expr) => {
        trace_dbg!(target: $target, level: tracing::Level::DEBUG, $ex)
    };
    ($ex:expr) => {
        trace_dbg!(level: tracing::Level::DEBUG, $ex)
    };
}

/// Creates a visual progress bar using Unicode block characters
///
/// # Arguments
/// * `progress` - Progress percentage (0.0 to 100.0)
/// * `width` - Width of the progress bar in characters
///
/// # Example
/// ```
/// let bar = format_progress_bar(50.0, 10);
/// // Returns: "█████░░░░░"
/// ```
pub fn format_progress_bar(progress: f64, width: usize) -> String {
    let progress = progress.clamp(0.0, 100.0);
    let filled = ((progress / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Formats bytes into a human-readable string
///
/// # Arguments
/// * `bytes` - Number of bytes
///
/// # Example
/// ```
/// assert_eq!(format_bytes(1024), "1.00 KB");
/// assert_eq!(format_bytes(1048576), "1.00 MB");
/// ```
/// Note: Currently unused - infrastructure for future byte-level tracking
#[allow(dead_code)]
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Formats a duration in seconds into a human-readable string
///
/// # Arguments
/// * `seconds` - Duration in seconds
///
/// # Example
/// ```
/// assert_eq!(format_duration(65), "1m 5s");
/// assert_eq!(format_duration(3661), "1h 1m 1s");
/// ```
/// Note: Currently unused - infrastructure for future byte-level tracking
#[allow(dead_code)]
pub fn format_duration(seconds: u64) -> String {
    if seconds == 0 {
        return "0s".to_string();
    }

    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    if secs > 0 || parts.is_empty() {
        parts.push(format!("{}s", secs));
    }

    parts.join(" ")
}

/// Formats transfer speed in bytes per second to human-readable format
///
/// # Arguments
/// * `bytes_per_sec` - Speed in bytes per second
///
/// Note: Currently unused - infrastructure for future byte-level tracking
#[allow(dead_code)]
pub fn format_speed(bytes_per_sec: f64) -> String {
    if bytes_per_sec < 1.0 {
        return "0 B/s".to_string();
    }
    format!("{}/s", format_bytes(bytes_per_sec as u64))
}

/// Calculates transfer speed given bytes transferred and duration
///
/// # Arguments
/// * `bytes` - Bytes transferred
/// * `duration_secs` - Duration in seconds (as f64 for sub-second precision)
///
/// # Returns
/// Speed in bytes per second
/// Note: Currently unused - infrastructure for future byte-level tracking
#[allow(dead_code)]
pub fn calculate_transfer_speed(bytes: u64, duration_secs: f64) -> f64 {
    if duration_secs <= 0.0 {
        return 0.0;
    }
    bytes as f64 / duration_secs
}

/// Calculates estimated time remaining
///
/// # Arguments
/// * `remaining_bytes` - Bytes left to transfer
/// * `speed` - Current speed in bytes per second
///
/// # Returns
/// ETA in seconds, or None if speed is zero
/// Note: Currently unused - infrastructure for future byte-level tracking
#[allow(dead_code)]
pub fn calculate_eta(remaining_bytes: u64, speed: f64) -> Option<u64> {
    if speed <= 0.0 {
        return None;
    }
    Some((remaining_bytes as f64 / speed).ceil() as u64)
}

pub fn version() -> String {
    let author = clap::crate_authors!();

    // let current_exe_path = PathBuf::from(clap::crate_name!()).display().to_string();
    let config_dir_path = get_config_dir().display().to_string();
    let data_dir_path = get_data_dir().display().to_string();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}

Config directory: {config_dir_path}
Data directory: {data_dir_path}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_progress_bar_empty() {
        assert_eq!(format_progress_bar(0.0, 10), "░░░░░░░░░░");
    }

    #[test]
    fn test_format_progress_bar_half() {
        assert_eq!(format_progress_bar(50.0, 10), "█████░░░░░");
    }

    #[test]
    fn test_format_progress_bar_full() {
        assert_eq!(format_progress_bar(100.0, 10), "██████████");
    }

    #[test]
    fn test_format_progress_bar_clamps_over_100() {
        assert_eq!(format_progress_bar(150.0, 10), "██████████");
    }

    #[test]
    fn test_format_progress_bar_clamps_negative() {
        assert_eq!(format_progress_bar(-10.0, 10), "░░░░░░░░░░");
    }

    #[test]
    fn test_format_bytes_small() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn test_format_bytes_kilobytes() {
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
    }

    #[test]
    fn test_format_bytes_megabytes() {
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1572864), "1.50 MB");
    }

    #[test]
    fn test_format_bytes_gigabytes() {
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_bytes_terabytes() {
        assert_eq!(format_bytes(1099511627776), "1.00 TB");
    }

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(format_duration(0), "0s");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(45), "45s");
    }

    #[test]
    fn test_format_duration_minutes_seconds() {
        assert_eq!(format_duration(65), "1m 5s");
        assert_eq!(format_duration(120), "2m");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3661), "1h 1m 1s");
        assert_eq!(format_duration(3600), "1h");
    }

    #[test]
    fn test_format_speed() {
        assert_eq!(format_speed(0.0), "0 B/s");
        assert_eq!(format_speed(1024.0), "1.00 KB/s");
        assert_eq!(format_speed(1048576.0), "1.00 MB/s");
    }

    #[test]
    fn test_calculate_transfer_speed() {
        assert_eq!(calculate_transfer_speed(1000, 1.0), 1000.0);
        assert_eq!(calculate_transfer_speed(1000, 2.0), 500.0);
        assert_eq!(calculate_transfer_speed(1000, 0.0), 0.0);
        assert_eq!(calculate_transfer_speed(1000, -1.0), 0.0);
    }

    #[test]
    fn test_calculate_eta() {
        assert_eq!(calculate_eta(1000, 100.0), Some(10));
        assert_eq!(calculate_eta(1000, 0.0), None);
        assert_eq!(calculate_eta(0, 100.0), Some(0));
    }
}
