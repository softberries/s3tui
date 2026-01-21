//! Transfer state persistence for resumable uploads and downloads
//!
//! This module provides functionality to persist transfer state to disk,
//! allowing uploads and downloads to be resumed after app restarts or failures.

use crate::settings::file_credentials::FileCredential;
use color_eyre::eyre::{self, Report};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::RwLock;

/// Unique identifier for a resumable transfer
pub type TransferId = String;

/// State of a multipart upload that can be resumed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumableUpload {
    /// Unique transfer ID
    pub id: TransferId,
    /// S3 multipart upload ID
    pub upload_id: String,
    /// Source file path on local disk
    pub source_path: String,
    /// Destination bucket
    pub bucket: String,
    /// Destination key in S3
    pub key: String,
    /// Total file size in bytes
    pub file_size: u64,
    /// Part size used for this upload
    pub part_size: u64,
    /// Completed parts with their ETags (part_number -> e_tag)
    pub completed_parts: HashMap<i32, String>,
    /// Credentials used for this upload
    pub credentials: CredentialInfo,
    /// Timestamp when upload was started
    pub started_at: u64,
    /// Timestamp of last activity
    pub last_updated: u64,
}

/// State of a download that can be resumed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumableDownload {
    /// Unique transfer ID
    pub id: TransferId,
    /// Source bucket
    pub bucket: String,
    /// Source key in S3
    pub key: String,
    /// Destination file path on local disk
    pub destination_path: String,
    /// Total file size in bytes
    pub total_size: u64,
    /// Bytes already downloaded
    pub bytes_downloaded: u64,
    /// Credentials used for this download
    pub credentials: CredentialInfo,
    /// Timestamp when download was started
    pub started_at: u64,
    /// Timestamp of last activity
    pub last_updated: u64,
}

/// Minimal credential info for persistence (without secrets in plain text)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialInfo {
    /// Credential profile name
    pub name: String,
    /// Default region
    pub region: String,
}

impl From<&FileCredential> for CredentialInfo {
    fn from(cred: &FileCredential) -> Self {
        CredentialInfo {
            name: cred.name.clone(),
            region: cred.default_region.clone(),
        }
    }
}

/// All persisted transfer state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransferState {
    /// Resumable uploads indexed by transfer ID
    pub uploads: HashMap<TransferId, ResumableUpload>,
    /// Resumable downloads indexed by transfer ID
    pub downloads: HashMap<TransferId, ResumableDownload>,
}

/// Manages persistence of transfer state to disk
pub struct TransferStateStore {
    /// Path to the state file
    state_file: PathBuf,
    /// In-memory state with RwLock for concurrent access
    state: RwLock<TransferState>,
}

impl TransferStateStore {
    /// Create a new transfer state store
    ///
    /// The state file will be created in the data directory if it doesn't exist.
    pub async fn new(data_dir: PathBuf) -> eyre::Result<Self> {
        let state_file = data_dir.join("transfer_state.json");

        // Load existing state or create empty
        let state = if state_file.exists() {
            match Self::load_from_file(&state_file).await {
                Ok(state) => {
                    tracing::info!(
                        "Loaded transfer state: {} uploads, {} downloads",
                        state.uploads.len(),
                        state.downloads.len()
                    );
                    state
                }
                Err(e) => {
                    tracing::warn!("Failed to load transfer state, starting fresh: {}", e);
                    TransferState::default()
                }
            }
        } else {
            TransferState::default()
        };

        Ok(TransferStateStore {
            state_file,
            state: RwLock::new(state),
        })
    }

    /// Load state from file
    async fn load_from_file(path: &PathBuf) -> eyre::Result<TransferState> {
        let content = fs::read_to_string(path).await?;
        let state: TransferState = serde_json::from_str(&content)?;
        Ok(state)
    }

    /// Save state to file
    async fn save_to_file(&self) -> eyre::Result<()> {
        let state = self.state.read().await;
        let content = serde_json::to_string_pretty(&*state)?;

        // Ensure parent directory exists
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&self.state_file, content).await?;
        Ok(())
    }

    /// Generate a unique transfer ID
    fn generate_id() -> TransferId {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("transfer_{}", timestamp)
    }

    /// Get current timestamp
    fn current_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    // ==================== Upload Operations ====================

    /// Register a new multipart upload for tracking
    #[allow(clippy::too_many_arguments)]
    pub async fn add_upload(
        &self,
        upload_id: String,
        source_path: String,
        bucket: String,
        key: String,
        file_size: u64,
        part_size: u64,
        credentials: &FileCredential,
    ) -> eyre::Result<TransferId> {
        let transfer_id = Self::generate_id();
        let now = Self::current_timestamp();

        let upload = ResumableUpload {
            id: transfer_id.clone(),
            upload_id,
            source_path,
            bucket,
            key,
            file_size,
            part_size,
            completed_parts: HashMap::new(),
            credentials: CredentialInfo::from(credentials),
            started_at: now,
            last_updated: now,
        };

        {
            let mut state = self.state.write().await;
            state.uploads.insert(transfer_id.clone(), upload);
        }

        self.save_to_file().await?;
        Ok(transfer_id)
    }

    /// Update completed parts for an upload
    pub async fn update_upload_part(
        &self,
        transfer_id: &TransferId,
        part_number: i32,
        e_tag: String,
    ) -> eyre::Result<()> {
        {
            let mut state = self.state.write().await;
            if let Some(upload) = state.uploads.get_mut(transfer_id) {
                upload.completed_parts.insert(part_number, e_tag);
                upload.last_updated = Self::current_timestamp();
            } else {
                return Err(Report::msg(format!(
                    "Upload not found: {}",
                    transfer_id
                )));
            }
        }

        self.save_to_file().await?;
        Ok(())
    }

    /// Mark an upload as complete and remove from state
    pub async fn complete_upload(&self, transfer_id: &TransferId) -> eyre::Result<()> {
        {
            let mut state = self.state.write().await;
            state.uploads.remove(transfer_id);
        }

        self.save_to_file().await?;
        tracing::debug!("Upload completed and removed from state: {}", transfer_id);
        Ok(())
    }

    /// Remove a failed/cancelled upload from state
    pub async fn remove_upload(&self, transfer_id: &TransferId) -> eyre::Result<()> {
        {
            let mut state = self.state.write().await;
            state.uploads.remove(transfer_id);
        }

        self.save_to_file().await?;
        Ok(())
    }

    // ==================== Download Operations ====================

    /// Register a new download for tracking
    pub async fn add_download(
        &self,
        bucket: String,
        key: String,
        destination_path: String,
        total_size: u64,
        credentials: &FileCredential,
    ) -> eyre::Result<TransferId> {
        let transfer_id = Self::generate_id();
        let now = Self::current_timestamp();

        let download = ResumableDownload {
            id: transfer_id.clone(),
            bucket,
            key,
            destination_path,
            total_size,
            bytes_downloaded: 0,
            credentials: CredentialInfo::from(credentials),
            started_at: now,
            last_updated: now,
        };

        {
            let mut state = self.state.write().await;
            state.downloads.insert(transfer_id.clone(), download);
        }

        self.save_to_file().await?;
        Ok(transfer_id)
    }

    /// Update bytes downloaded for a download
    pub async fn update_download_progress(
        &self,
        transfer_id: &TransferId,
        bytes_downloaded: u64,
    ) -> eyre::Result<()> {
        {
            let mut state = self.state.write().await;
            if let Some(download) = state.downloads.get_mut(transfer_id) {
                download.bytes_downloaded = bytes_downloaded;
                download.last_updated = Self::current_timestamp();
            } else {
                return Err(Report::msg(format!(
                    "Download not found: {}",
                    transfer_id
                )));
            }
        }

        self.save_to_file().await?;
        Ok(())
    }

    /// Mark a download as complete and remove from state
    pub async fn complete_download(&self, transfer_id: &TransferId) -> eyre::Result<()> {
        {
            let mut state = self.state.write().await;
            state.downloads.remove(transfer_id);
        }

        self.save_to_file().await?;
        tracing::debug!("Download completed and removed from state: {}", transfer_id);
        Ok(())
    }

    // ==================== Utility Operations ====================

    /// Get counts of pending transfers
    pub async fn get_pending_counts(&self) -> (usize, usize) {
        let state = self.state.read().await;
        (state.uploads.len(), state.downloads.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_credential() -> FileCredential {
        FileCredential {
            name: "test".to_string(),
            access_key: "test_key".to_string(),
            secret_key: "test_secret".to_string(),
            default_region: "us-east-1".to_string(),
            selected: true,
        }
    }

    #[tokio::test]
    async fn test_persistence_across_reload() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Create store and add data
        {
            let store = TransferStateStore::new(path.clone()).await.unwrap();
            let creds = create_test_credential();

            store
                .add_upload(
                    "upload123".to_string(),
                    "/path/file.txt".to_string(),
                    "bucket".to_string(),
                    "key".to_string(),
                    1024,
                    1024,
                    &creds,
                )
                .await
                .unwrap();

            store
                .add_download(
                    "bucket".to_string(),
                    "key".to_string(),
                    "/local/file.txt".to_string(),
                    2048,
                    &creds,
                )
                .await
                .unwrap();
        }

        // Reload and verify data persisted
        {
            let store = TransferStateStore::new(path).await.unwrap();
            let (uploads, downloads) = store.get_pending_counts().await;
            assert_eq!(uploads, 1);
            assert_eq!(downloads, 1);
        }
    }
}
