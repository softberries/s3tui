use crate::model::local_data_item::LocalDataItem;
use crate::settings::file_credentials::FileCredential;

#[derive(Debug, Clone)]
pub struct LocalSelectedItem {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub destination_bucket: String,
    pub destination_path: String,
    pub transferred: bool,
    pub s3_creds: FileCredential,
    pub progress: f64
}

impl LocalSelectedItem {
    pub fn new(name: String, path: String, is_directory: bool, destination_bucket: String, destination_path: String, s3_creds: FileCredential) -> LocalSelectedItem {
        LocalSelectedItem {
            name,
            path,
            is_directory,
            destination_bucket,
            destination_path,
            transferred: false,
            s3_creds,
            progress: 0f64,
        }
    }

    pub fn from_local_data_item(item: LocalDataItem, s3_creds: FileCredential) -> Self {
        LocalSelectedItem {
            name: item.name,
            path: item.path,
            is_directory: item.is_directory,
            destination_bucket: String::new(),
            destination_path: String::new(),
            transferred: false,
            s3_creds: s3_creds,
            progress: 0f64,
        }
    }

    pub fn to_columns(&self) -> Vec<String> {
        let progress = format!("{:.2}%", self.progress);
        vec![self.name.clone(), self.path.clone(), self.destination_bucket.clone(), self.destination_path.to_string(), self.s3_creds.name.clone(), progress]
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