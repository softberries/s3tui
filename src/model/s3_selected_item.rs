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
    pub children: Option<Vec<S3SelectedItem>>,
    pub error: Option<String>,
}

impl S3SelectedItem {
    pub fn from_s3_data_item(
        item: S3DataItem,
        creds: FileCredential,
        destination_dir: String,
    ) -> S3SelectedItem {
        S3SelectedItem {
            bucket: item.bucket,
            name: item.name,
            path: Some(item.path),
            is_directory: item.is_directory,
            is_bucket: item.is_bucket,
            destination_dir: destination_dir.clone(),
            transferred: false,
            s3_creds: creds,
            progress: 0f64,
            children: None,
            error: None,
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
            destination_dir: destination_dir.clone(),
            transferred: false,
            s3_creds: creds,
            progress: 0f64,
            children: Some(children),
            error: None,
        }
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
            children: None,
            error: None,
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
            transferred: false,
            s3_creds: Default::default(),
            progress: 0f64,
            children: None,
            error: None,
        };
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
            children: Some(vec![child.clone()]),
            error: None,
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
}
