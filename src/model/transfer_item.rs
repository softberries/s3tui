use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::transfer_state::TransferState;
use crate::settings::file_credentials::FileCredential;

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
}

impl TransferItem {
    pub fn to_columns(&self) -> Vec<String> {
        let progress = format!("{:.2}%", self.transfer_state.progress());
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
        }
    }

    /// Returns true if the transfer has completed successfully
    pub fn is_transferred(&self) -> bool {
        self.transfer_state.is_completed()
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
