use crate::model::s3_data_item::S3DataItem;
use crate::settings::file_credentials::FileCredential;

/// Represents an item (file/directory/bucket) on your s3 account
#[derive(Debug, Clone)]
pub struct S3SelectedItem {
    pub bucket: Option<String>,
    pub name: String,
    pub path: Option<String>,
    pub is_directory: bool,
    pub is_bucket: bool,
    pub destination_dir: String,
    pub transferred: bool,
    pub s3_creds: FileCredential,
    pub progress: f64,
    pub error: Option<String>,
}

impl S3SelectedItem {
    pub fn new(name: String, bucket: Option<String>, path: Option<String>, is_directory: bool, is_bucket: bool, destination_dir: String, s3_creds: FileCredential) -> S3SelectedItem {
        S3SelectedItem {
            bucket,
            name,
            path,
            is_directory,
            is_bucket,
            destination_dir,
            transferred: false,
            s3_creds,
            progress: 0f64,
            error: None,
        }
    }

    pub fn from_s3_data_item(item: S3DataItem, creds: FileCredential) -> S3SelectedItem {
        S3SelectedItem {
            bucket: item.bucket,
            name: item.name,
            path: Some(item.path),
            is_directory: item.is_directory,
            is_bucket: item.is_bucket,
            destination_dir: String::new(), // Or provide a default value or additional context if needed
            transferred: false, // Default value since it's not part of S3DataItem
            s3_creds: creds,
            progress: 0f64,
            error: None,
        }
    }
}


impl PartialEq for S3SelectedItem {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name &&
            self.bucket == other.bucket &&
            self.path == other.path &&
            self.is_directory == other.is_directory &&
            self.is_bucket == other.is_bucket
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
            transferred: false,
            s3_creds: Default::default(),
            progress: 0f64,
            error: None
        };
        let res = S3SelectedItem::new(
            "file1.txt".into(),
            Some("test-bucket".into()),
            Some("path/to/file1.txt".into()),
            false,
            false,
            "".into(),
            Default::default()
        );
        assert_eq!(item, res);
    }
}