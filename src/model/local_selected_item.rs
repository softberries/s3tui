use crate::model::has_children::{HasChildren, HasProgress};
use crate::model::local_data_item::LocalDataItem;
use crate::model::transfer_state::TransferState;
use crate::settings::file_credentials::FileCredential;
use std::fs;
use std::path::{Path, PathBuf};

/// Keeps the information about the selected file which is later displayed on the transfers page
#[derive(Debug, Clone)]
pub struct LocalSelectedItem {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub destination_bucket: String,
    pub destination_path: String,
    pub s3_creds: FileCredential,
    pub children: Option<Vec<LocalSelectedItem>>,
    pub transfer_state: TransferState,
}

impl LocalSelectedItem {
    pub fn new(
        name: String,
        path: String,
        is_directory: bool,
        destination_bucket: String,
        destination_path: String,
        s3_creds: FileCredential,
        children: Option<Vec<LocalSelectedItem>>,
    ) -> LocalSelectedItem {
        LocalSelectedItem {
            name,
            path,
            is_directory,
            destination_bucket,
            destination_path,
            s3_creds,
            children,
            transfer_state: TransferState::default(),
        }
    }

    pub fn from_local_data_item(item: LocalDataItem, s3_creds: FileCredential) -> Self {
        LocalSelectedItem {
            name: item.name,
            path: item.path,
            is_directory: item.is_directory,
            destination_bucket: String::new(),
            destination_path: String::new(),
            s3_creds,
            children: None,
            transfer_state: TransferState::default(),
        }
    }

    pub fn list_directory_items(item: &LocalSelectedItem) -> Vec<LocalSelectedItem> {
        let path = Path::new(&item.path);

        if item.is_directory {
            let mut items = Vec::new();
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.is_dir() {
                        // Recursively process subdirectories
                        items.extend(Self::list_directory_items(&LocalSelectedItem {
                            name: path.file_name().unwrap().to_string_lossy().into_owned(),
                            path: path.to_string_lossy().into(),
                            is_directory: true,
                            destination_bucket: item.destination_bucket.clone(),
                            destination_path: PathBuf::from(&item.destination_path)
                                .join(path.file_name().unwrap().to_string_lossy().into_owned())
                                .to_string_lossy()
                                .into(),
                            s3_creds: item.s3_creds.clone(),
                            children: None,
                            transfer_state: TransferState::default(),
                        }));
                    } else {
                        // Process files
                        items.push(LocalSelectedItem {
                            name: path.file_name().unwrap().to_string_lossy().into_owned(),
                            path: path.to_string_lossy().into(),
                            is_directory: false,
                            destination_bucket: item.destination_bucket.clone(),
                            destination_path: PathBuf::from(&item.destination_path)
                                .join(path.file_name().unwrap().to_string_lossy().into_owned())
                                .to_string_lossy()
                                .into(),
                            s3_creds: item.s3_creds.clone(),
                            children: None,
                            transfer_state: TransferState::default(),
                        });
                    }
                }
            }
            items
        } else {
            vec![item.clone()]
        }
    }

    /// Returns true if the transfer has completed successfully
    pub fn is_transferred(&self) -> bool {
        self.transfer_state.is_completed()
    }
}

impl PartialEq for LocalSelectedItem {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.path == other.path
            && self.is_directory == other.is_directory
    }
}

impl HasChildren for LocalSelectedItem {
    fn children(&self) -> Option<&Vec<Self>> {
        self.children.as_ref()
    }

    fn take_children(self) -> Vec<Self> {
        self.children.unwrap_or_default()
    }
}

impl HasProgress for LocalSelectedItem {
    fn progress(&self) -> f64 {
        self.transfer_state.progress()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_local_selected_item_correctly() {
        let item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
        };
        let res = LocalSelectedItem::new(
            "file1.txt".into(),
            "path/to/file1.txt".into(),
            false,
            "test-bucket".into(),
            "".to_string(),
            Default::default(),
            None,
        );
        assert_eq!(item, res);
    }

    #[test]
    fn test_transfer_state_helpers() {
        let mut item = LocalSelectedItem {
            name: "file1.txt".into(),
            path: "/home/user/file1.txt".into(),
            is_directory: false,
            destination_bucket: "test-bucket".into(),
            destination_path: "uploads/file1.txt".into(),
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
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
        item.transfer_state = TransferState::Failed("Upload error".into());
        assert!(!item.is_transferred());
        assert_eq!(item.transfer_state.error(), Some("Upload error"));
    }
}
