use crate::model::s3_data_item::S3DataItem;

#[derive(Debug, Clone)]
pub struct S3SelectedItem {
    pub bucket: Option<String>,
    pub name: String,
    pub path: Option<String>,
    pub is_directory: bool,
    pub is_bucket: bool,
    pub destination_dir: String,
    pub transferred: bool,
}

impl S3SelectedItem {
    pub fn new(name: String, bucket: Option<String>, path: Option<String>, is_directory: bool, is_bucket: bool, destination_dir: String) -> S3SelectedItem {
        S3SelectedItem {
            bucket,
            name,
            path,
            is_directory,
            is_bucket,
            destination_dir,
            transferred: false,
        }
    }

    pub fn to_columns(&self) -> Vec<String> {
        if self.is_bucket {
            vec![self.name.clone(), "/".to_string(), self.destination_dir.clone(), self.is_bucket.to_string(), self.is_directory.to_string()]
        } else {
            vec![self.bucket.clone().unwrap_or("".to_string()), self.name.clone(), self.destination_dir.clone(), self.is_bucket.to_string(), self.is_directory.to_string()]
        }
    }
}

impl From<S3DataItem> for S3SelectedItem {
    fn from(item: S3DataItem) -> Self {
        S3SelectedItem {
            bucket: item.bucket,
            name: item.name,
            path: Some(item.path),
            is_directory: item.is_directory,
            is_bucket: item.is_bucket,
            destination_dir: String::new(), // Or provide a default value or additional context if needed
            transferred: false, // Default value since it's not part of S3DataItem
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
        // self.destination_dir == other.destination_dir
    }
}
// impl Eq for S3SelectedItem {}