use crate::model::local_data_item::LocalDataItem;
use crate::settings::file_credentials::FileCredential;

/// Keeps the information about the selected file which is later displayed on the transfers page
#[derive(Debug, Clone)]
pub struct LocalSelectedItem {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub destination_bucket: String,
    pub destination_path: String,
    pub transferred: bool,
    pub s3_creds: FileCredential,
    pub progress: f64,
    pub error: Option<String>
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
            error: None
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
            s3_creds,
            progress: 0f64,
            error: None,
        }
    }

    pub fn to_columns(&self) -> Vec<String> {
        let progress = format!("{:.2}%", self.progress);
        vec![self.name.clone(), self.path.clone(), self.destination_bucket.clone(), self.destination_path.to_string(), self.s3_creds.name.clone(), progress, self.error.clone().unwrap_or("".to_string())]
    }
}


impl PartialEq for LocalSelectedItem {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name &&
            self.path == other.path &&
            self.is_directory == other.is_directory
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
            transferred: false,
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            progress: 0.0,
            is_directory: false,
            s3_creds: Default::default(),
            error: None
        };
        let res = LocalSelectedItem::new(
            "file1.txt".into(),
            "path/to/file1.txt".into(),
            false,
            "test-bucket".into(),
            "".to_string(),
            Default::default()
        );
        assert_eq!(item, res);
    }

    #[test]
    fn to_columns_get_correct_vector() {
        let item = LocalSelectedItem::new(
            "file1.txt".into(),
            "path/to/file1.txt".into(),
            false,
            "test-bucket".into(),
            "".to_string(),
            Default::default()
        );
        let res = item.to_columns();
        assert_eq!(res.len(), 7);
        assert_eq!(res[0], item.name);
        assert_eq!(res[1], item.path);
        assert_eq!(res[2], item.destination_bucket);
        assert_eq!(res[3], item.destination_path);
        assert_eq!(res[4], item.s3_creds.name);
        assert_eq!(res[5], "0.00%".to_string());
        assert_eq!(res[6], "".to_string());
    }

    #[test]
    fn to_columns_with_error_get_correct_vector() {
        let item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            transferred: false,
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            progress: 0.0,
            is_directory: false,
            s3_creds: Default::default(),
            error: Some("Error".into())
        };
        let res = item.to_columns();
        assert_eq!(res.len(), 7);
        assert_eq!(res[0], item.name);
        assert_eq!(res[1], item.path);
        assert_eq!(res[2], item.destination_bucket);
        assert_eq!(res[3], item.destination_path);
        assert_eq!(res[4], item.s3_creds.name);
        assert_eq!(res[5], "0.00%".to_string());
        assert_eq!(res[6], "Error".to_string());
    }
}