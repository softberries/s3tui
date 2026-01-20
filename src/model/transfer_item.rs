use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::transfer_state::TransferState;
use crate::services::transfer_manager::JobId;
use crate::settings::file_credentials::FileCredential;
use crate::utils::format_progress_bar;
use std::time::Instant;

/// Represents an item (file/directory/bucket) on your transfers list
#[derive(Debug, Clone)]
pub struct TransferItem {
    pub direction: String,
    pub bucket: String,
    pub name: String,
    pub path: Option<String>,
    pub destination_dir: String,
    pub s3_creds: FileCredential,
    pub transfer_state: TransferState,
    /// Job ID assigned by TransferManager when queued
    pub job_id: Option<JobId>,
    /// Total size of the file in bytes (0 if unknown)
    /// Note: Currently unused - infrastructure for future byte-level tracking
    #[allow(dead_code)]
    pub total_bytes: u64,
    /// Bytes transferred so far
    /// Note: Currently unused - infrastructure for future byte-level tracking
    #[allow(dead_code)]
    pub bytes_transferred: u64,
    /// When the transfer started (for speed calculation)
    /// Note: Currently unused - infrastructure for future byte-level tracking
    #[allow(dead_code)]
    pub started_at: Option<Instant>,
}

impl TransferItem {
    /// Progress bar width in characters
    const PROGRESS_BAR_WIDTH: usize = 8;

    pub fn to_columns(&self) -> Vec<String> {
        let progress = if self.transfer_state.is_cancelled() {
            "Cancelled".to_string()
        } else if self.transfer_state.is_paused() {
            let progress_pct = self.transfer_state.progress();
            format!("Paused {:>5.1}%", progress_pct)
        } else if self.transfer_state.is_completed() {
            "Completed".to_string()
        } else {
            let progress_pct = self.transfer_state.progress();
            let progress_bar = format_progress_bar(progress_pct, Self::PROGRESS_BAR_WIDTH);
            format!("{} {:>5.1}%", progress_bar, progress_pct)
        };
        let error = self
            .transfer_state
            .error()
            .map(|s| s.to_string())
            .unwrap_or_default();
        vec![
            self.direction.clone(),
            self.bucket.clone(),
            self.name.clone(),
            self.destination_dir.clone(),
            self.s3_creds.name.clone(),
            progress,
            error,
        ]
    }

    pub fn from_s3_selected_item(item: S3SelectedItem) -> TransferItem {
        TransferItem {
            direction: "↓".into(),
            bucket: item.bucket.unwrap_or_default(),
            name: item.name,
            path: item.path,
            destination_dir: item.destination_dir,
            s3_creds: item.s3_creds,
            transfer_state: item.transfer_state,
            job_id: item.job_id,
            total_bytes: 0,
            bytes_transferred: 0,
            started_at: None,
        }
    }

    pub fn from_local_selected_item(item: LocalSelectedItem) -> TransferItem {
        TransferItem {
            direction: "↑".into(),
            bucket: item.destination_bucket,
            name: item.name,
            path: Some(item.path),
            destination_dir: item.destination_path,
            s3_creds: item.s3_creds,
            transfer_state: item.transfer_state,
            job_id: item.job_id,
            total_bytes: 0,
            bytes_transferred: 0,
            started_at: None,
        }
    }

    /// Calculate the current transfer speed in bytes per second
    /// Note: Currently unused - infrastructure for future byte-level tracking
    #[allow(dead_code)]
    pub fn speed(&self) -> f64 {
        if let Some(started_at) = self.started_at {
            let elapsed = started_at.elapsed().as_secs_f64();
            crate::utils::calculate_transfer_speed(self.bytes_transferred, elapsed)
        } else {
            0.0
        }
    }

    /// Calculate estimated time remaining in seconds
    /// Note: Currently unused - infrastructure for future byte-level tracking
    #[allow(dead_code)]
    pub fn eta(&self) -> Option<u64> {
        let speed = self.speed();
        if speed > 0.0 && self.total_bytes > self.bytes_transferred {
            let remaining = self.total_bytes - self.bytes_transferred;
            crate::utils::calculate_eta(remaining, speed)
        } else {
            None
        }
    }

    /// Returns true if the transfer has completed successfully
    pub fn is_transferred(&self) -> bool {
        self.transfer_state.is_completed()
    }

    /// Returns true if the transfer is paused
    pub fn is_paused(&self) -> bool {
        self.transfer_state.is_paused()
    }

    /// Returns true if the transfer was cancelled
    pub fn is_cancelled(&self) -> bool {
        self.transfer_state.is_cancelled()
    }

    /// Returns the error message if the transfer failed
    pub fn error(&self) -> Option<&str> {
        self.transfer_state.error()
    }
}

impl PartialEq for TransferItem {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.bucket == other.bucket
            && self.path == other.path
            && self.destination_dir == other.destination_dir
    }
}
