use crate::model::has_children::{HasChildren, HasProgress};
use crate::model::s3_data_item::S3DataItem;
use crate::model::transfer_state::TransferState;
use crate::services::transfer_manager::JobId;
use crate::settings::file_credentials::FileCredential;
use serde::{Deserialize, Serialize};

/// Represents an item (file/directory/bucket) on your s3 account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3SelectedItem {
    pub bucket: Option<String>,
    pub name: String,
    pub path: Option<String>,
    pub is_directory: bool,
    pub is_bucket: bool,
    pub destination_dir: String,
    pub s3_creds: FileCredential,
    pub children: Option<Vec<S3SelectedItem>>,
    pub transfer_state: TransferState,
    /// Job ID assigned by TransferManager when queued
    pub job_id: Option<JobId>,
}

impl S3SelectedItem {
    pub fn from_s3_data_item(
        item: S3DataItem,
        creds: FileCredential,
        destination_dir: String,
    ) -> S3SelectedItem {
        S3SelectedItem {
            bucket: item.bucket,
            name: item.name.clone(),
            path: Some(item.path),
            is_directory: item.is_directory,
            is_bucket: item.is_bucket,
            destination_dir,
            s3_creds: creds,
            children: None,
            transfer_state: TransferState::default(),
            job_id: None,
        }
    }

    pub fn from_s3_data_item_with_children(
        item: S3DataItem,
        creds: FileCredential,
        destination_dir: String,
        children: Vec<S3SelectedItem>,
    ) -> S3SelectedItem {
        S3SelectedItem {
            bucket: item.bucket,
            name: item.name,
            path: Some(item.path),
            is_directory: item.is_directory,
            is_bucket: item.is_bucket,
            destination_dir,
            s3_creds: creds,
            children: Some(children),
            transfer_state: TransferState::default(),
            job_id: None,
        }
    }

    /// Returns true if the transfer has completed successfully
    pub fn is_transferred(&self) -> bool {
        self.transfer_state.is_completed()
    }
}

impl PartialEq for S3SelectedItem {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.bucket == other.bucket
            && self.path == other.path
            && self.is_directory == other.is_directory
            && self.is_bucket == other.is_bucket
    }
}

impl HasChildren for S3SelectedItem {
    fn children(&self) -> Option<&Vec<Self>> {
        self.children.as_ref()
    }

    fn take_children(self) -> Vec<Self> {
        self.children.unwrap_or_default()
    }
}

impl HasProgress for S3SelectedItem {
    fn progress(&self) -> f64 {
        self.transfer_state.progress()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_s3_selected_item_correctly() {
        let item = S3SelectedItem {
            bucket: Some("test-bucket".into()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "".to_string(),
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
            job_id: None,
        };
        let s3_data_item = S3DataItem {
            bucket: Some("test-bucket".into()),
            name: "file1.txt".into(),
            size: "100kB".into(),
            file_type: "txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            is_bucket: false,
            region: Some("eu-west-1".into()),
        };
        let creds = FileCredential {
            name: "personal".into(),
            access_key: "abc".into(),
            secret_key: "abc".into(),
            default_region: "abc".into(),
            endpoint_url: None,
            force_path_style: false,
            selected: true,
        };
        let destination_dir = "/".into();
        let res = S3SelectedItem::from_s3_data_item(s3_data_item, creds, destination_dir);
        assert_eq!(item, res);
    }

    #[test]
    fn create_s3_selected_item_with_children_correctly() {
        let child = S3SelectedItem {
            bucket: Some("test-bucket".into()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "".to_string(),
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
            job_id: None,
        };
        let item = S3SelectedItem {
            bucket: Some("test-bucket".into()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "".to_string(),
            s3_creds: Default::default(),
            children: Some(vec![child.clone()]),
            transfer_state: TransferState::default(),
            job_id: None,
        };
        let s3_data_item = S3DataItem {
            bucket: Some("test-bucket".into()),
            name: "file1.txt".into(),
            size: "100kB".into(),
            file_type: "txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            is_bucket: false,
            region: Some("eu-west-1".into()),
        };
        let creds = FileCredential {
            name: "personal".into(),
            access_key: "abc".into(),
            secret_key: "abc".into(),
            default_region: "abc".into(),
            endpoint_url: None,
            force_path_style: false,
            selected: true,
        };
        let destination_dir = "/".into();
        let res = S3SelectedItem::from_s3_data_item_with_children(
            s3_data_item,
            creds,
            destination_dir,
            vec![child],
        );
        assert_eq!(item, res);
        assert_eq!(item.children.unwrap(), res.children.unwrap());
    }

    #[test]
    fn test_transfer_state_helpers() {
        let mut item = S3SelectedItem {
            bucket: Some("test-bucket".into()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "/downloads".to_string(),
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
            job_id: None,
        };

        // Initially pending
        assert!(!item.is_transferred());
        assert_eq!(item.transfer_state.progress(), 0.0);
        assert!(item.transfer_state.error().is_none());

        // Set in progress
        item.transfer_state = TransferState::InProgress(50.0);
        assert!(!item.is_transferred());
        assert_eq!(item.transfer_state.progress(), 50.0);

        // Complete
        item.transfer_state = TransferState::Completed;
        assert!(item.is_transferred());
        assert_eq!(item.transfer_state.progress(), 100.0);

        // Failed
        item.transfer_state = TransferState::Failed("Network error".into());
        assert!(!item.is_transferred());
        assert_eq!(item.transfer_state.error(), Some("Network error"));
    }
}
