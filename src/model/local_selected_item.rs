use crate::model::local_data_item::LocalDataItem;

#[derive(Debug, Clone)]
pub struct LocalSelectedItem {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub destination_bucket: String,
    pub destination_path: String,
    pub transferred: bool,
}

impl LocalSelectedItem {
    pub fn new(name: String, path: String, is_directory: bool, destination_bucket: String, destination_path: String) -> LocalSelectedItem {
        LocalSelectedItem {
            name,
            path,
            is_directory,
            destination_bucket,
            destination_path,
            transferred: false,
        }
    }

    pub fn to_columns(&self) -> Vec<String> {
        vec![self.name.clone(), self.path.clone(), self.destination_bucket.clone(), self.destination_path.to_string(), self.is_directory.to_string()]
    }
}

impl From<LocalDataItem> for LocalSelectedItem {
    fn from(item: LocalDataItem) -> Self {
        LocalSelectedItem {
            name: item.name,
            path: item.path,
            is_directory: item.is_directory,
            destination_bucket: String::new(),
            destination_path: String::new(),
            transferred: false,
        }
    }
}

impl PartialEq for LocalSelectedItem {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name &&
            self.path == other.path &&
            self.is_directory == other.is_directory
            // self.destination_bucket == other.destination_bucket
            // self.destination_path == other.destination_path
    }
}
// impl Eq for S3SelectedItem {}