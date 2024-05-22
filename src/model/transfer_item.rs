use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::settings::file_credentials::FileCredential;

/// Represents an item (file/directory/bucket) on your transfers list
#[derive(Debug, Clone)]
pub struct TransferItem {
    pub direction: String,
    pub bucket: String,
    pub name: String,
    pub path: Option<String>,
    pub destination_dir: String,
    pub transferred: bool,
    pub s3_creds: FileCredential,
    pub progress: f64,
    pub error: Option<String>,
}

impl TransferItem {
    pub fn to_columns(&self) -> Vec<String> {
        let progress = format!("{:.2}%", self.progress);
        vec![self.direction.clone(), self.bucket.clone(), self.name.clone(), self.destination_dir.clone(), self.s3_creds.name.clone(), progress, self.error.clone().unwrap_or("".to_string())]
    }

    pub fn from_s3_selected_item(item: S3SelectedItem) -> TransferItem {
        TransferItem {
            direction: "↓".into(),
            bucket: item.bucket.unwrap_or("".into()),
            name: item.name,
            path: item.path,
            destination_dir: item.destination_dir,
            transferred: item.transferred,
            s3_creds: item.s3_creds,
            progress: item.progress,
            error: item.error,
        }
    }

    pub fn from_local_selected_item(item: LocalSelectedItem) -> TransferItem {
        TransferItem {
            direction: "↑".into(),
            bucket: item.destination_bucket,
            name: item.name,
            path: Some(item.path),
            destination_dir: item.destination_path,
            transferred: item.transferred,
            s3_creds: item.s3_creds,
            progress: item.progress,
            error: item.error,
        }
    }
}


impl PartialEq for TransferItem {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name &&
            self.bucket == other.bucket &&
            self.path == other.path &&
            self.destination_dir == other.destination_dir
    }
}