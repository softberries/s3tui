//! Transfer items persistence for resuming transfers across app restarts
//!
//! This module persists the selected transfer items (uploads and downloads)
//! so they can be restored when the app restarts.

use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::transfer_state::TransferState;
use color_eyre::eyre;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// Persisted transfer items state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistedTransfers {
    /// Downloads (S3 -> Local)
    pub s3_selected_items: Vec<S3SelectedItem>,
    /// Uploads (Local -> S3)
    pub local_selected_items: Vec<LocalSelectedItem>,
}

/// Manages persistence of transfer items to disk
pub struct TransferPersistence {
    /// Path to the persistence file
    state_file: PathBuf,
}

impl TransferPersistence {
    /// Create a new transfer persistence manager
    pub fn new(data_dir: PathBuf) -> Self {
        TransferPersistence {
            state_file: data_dir.join("pending_transfers.json"),
        }
    }

    /// Load persisted transfers from disk
    pub async fn load(&self) -> eyre::Result<PersistedTransfers> {
        if !self.state_file.exists() {
            return Ok(PersistedTransfers::default());
        }

        match fs::read_to_string(&self.state_file).await {
            Ok(content) => {
                let mut transfers: PersistedTransfers = serde_json::from_str(&content)?;

                // Reset job_ids since they're not valid across restarts
                // Also convert InProgress to Paused since the transfer was interrupted
                for item in &mut transfers.s3_selected_items {
                    item.job_id = None;
                    Self::reset_transfer_state(&mut item.transfer_state);
                    // Reset children too
                    if let Some(children) = &mut item.children {
                        for child in children {
                            child.job_id = None;
                            Self::reset_transfer_state(&mut child.transfer_state);
                        }
                    }
                }
                for item in &mut transfers.local_selected_items {
                    item.job_id = None;
                    Self::reset_transfer_state(&mut item.transfer_state);
                    // Reset children too
                    if let Some(children) = &mut item.children {
                        for child in children {
                            child.job_id = None;
                            Self::reset_transfer_state(&mut child.transfer_state);
                        }
                    }
                }

                tracing::info!(
                    "Loaded {} pending downloads and {} pending uploads from previous session",
                    transfers.s3_selected_items.len(),
                    transfers.local_selected_items.len()
                );
                Ok(transfers)
            }
            Err(e) => {
                tracing::warn!("Failed to load persisted transfers: {}", e);
                Ok(PersistedTransfers::default())
            }
        }
    }

    /// Reset transfer state for items loaded from disk
    /// InProgress becomes Paused (since it was interrupted)
    /// Pending stays Pending
    /// Terminal states are filtered out before saving
    fn reset_transfer_state(state: &mut TransferState) {
        match state {
            TransferState::InProgress(progress) => {
                *state = TransferState::Paused(*progress);
            }
            TransferState::Pending => {
                // Keep as pending
            }
            TransferState::Paused(_) => {
                // Keep as paused
            }
            _ => {
                // For terminal states (shouldn't be persisted), reset to pending
                *state = TransferState::Pending;
            }
        }
    }

    /// Save transfers to disk
    /// Only saves non-terminal (not completed, not failed, not cancelled) transfers
    pub async fn save(
        &self,
        s3_items: &[S3SelectedItem],
        local_items: &[LocalSelectedItem],
    ) -> eyre::Result<()> {
        // Filter out completed/failed/cancelled transfers
        let s3_to_save: Vec<S3SelectedItem> = s3_items
            .iter()
            .filter(|item| !item.transfer_state.is_terminal())
            .cloned()
            .collect();

        let local_to_save: Vec<LocalSelectedItem> = local_items
            .iter()
            .filter(|item| !item.transfer_state.is_terminal())
            .cloned()
            .collect();

        let transfers = PersistedTransfers {
            s3_selected_items: s3_to_save,
            local_selected_items: local_to_save,
        };

        // Ensure parent directory exists
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&transfers)?;
        fs::write(&self.state_file, content).await?;

        tracing::debug!(
            "Saved {} downloads and {} uploads to disk",
            transfers.s3_selected_items.len(),
            transfers.local_selected_items.len()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::file_credentials::FileCredential;
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
    async fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TransferPersistence::new(temp_dir.path().to_path_buf());
        let creds = create_test_credential();

        let s3_items = vec![S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file.txt".to_string(),
            path: Some("path/file.txt".to_string()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "/downloads".to_string(),
            s3_creds: creds.clone(),
            children: None,
            transfer_state: TransferState::InProgress(50.0),
            job_id: Some(crate::services::transfer_manager::JobId::from(1)),
        }];

        let local_items = vec![LocalSelectedItem {
            name: "upload.txt".to_string(),
            path: "/home/user/upload.txt".to_string(),
            is_directory: false,
            destination_bucket: "test-bucket".to_string(),
            destination_path: "uploads/".to_string(),
            s3_creds: creds,
            children: None,
            transfer_state: TransferState::Paused(25.0),
            job_id: Some(crate::services::transfer_manager::JobId::from(2)),
        }];

        // Save
        persistence.save(&s3_items, &local_items).await.unwrap();

        // Load
        let loaded = persistence.load().await.unwrap();

        assert_eq!(loaded.s3_selected_items.len(), 1);
        assert_eq!(loaded.local_selected_items.len(), 1);

        // Check that job_ids are reset
        assert!(loaded.s3_selected_items[0].job_id.is_none());
        assert!(loaded.local_selected_items[0].job_id.is_none());

        // Check that InProgress became Paused
        assert!(matches!(
            loaded.s3_selected_items[0].transfer_state,
            TransferState::Paused(p) if (p - 50.0).abs() < 0.01
        ));

        // Paused should stay Paused
        assert!(matches!(
            loaded.local_selected_items[0].transfer_state,
            TransferState::Paused(p) if (p - 25.0).abs() < 0.01
        ));
    }

    #[tokio::test]
    async fn test_completed_items_not_saved() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TransferPersistence::new(temp_dir.path().to_path_buf());
        let creds = create_test_credential();

        let s3_items = vec![
            S3SelectedItem {
                bucket: Some("test-bucket".to_string()),
                name: "completed.txt".to_string(),
                path: Some("completed.txt".to_string()),
                is_directory: false,
                is_bucket: false,
                destination_dir: "/downloads".to_string(),
                s3_creds: creds.clone(),
                children: None,
                transfer_state: TransferState::Completed,
                job_id: None,
            },
            S3SelectedItem {
                bucket: Some("test-bucket".to_string()),
                name: "pending.txt".to_string(),
                path: Some("pending.txt".to_string()),
                is_directory: false,
                is_bucket: false,
                destination_dir: "/downloads".to_string(),
                s3_creds: creds,
                children: None,
                transfer_state: TransferState::Pending,
                job_id: None,
            },
        ];

        persistence.save(&s3_items, &[]).await.unwrap();
        let loaded = persistence.load().await.unwrap();

        // Only the pending item should be saved
        assert_eq!(loaded.s3_selected_items.len(), 1);
        assert_eq!(loaded.s3_selected_items[0].name, "pending.txt");
    }

    #[tokio::test]
    async fn test_load_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let persistence = TransferPersistence::new(temp_dir.path().to_path_buf());

        let loaded = persistence.load().await.unwrap();
        assert!(loaded.s3_selected_items.is_empty());
        assert!(loaded.local_selected_items.is_empty());
    }
}
