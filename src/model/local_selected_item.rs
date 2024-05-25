use std::fs;
use std::path::{Path, PathBuf};
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
    pub children: Option<Vec<LocalSelectedItem>>,
    pub error: Option<String>,
}

impl LocalSelectedItem {
    pub fn new(name: String, path: String, is_directory: bool, destination_bucket: String, destination_path: String, s3_creds: FileCredential, children: Option<Vec<LocalSelectedItem>>) -> LocalSelectedItem {
        LocalSelectedItem {
            name,
            path,
            is_directory,
            destination_bucket,
            destination_path,
            transferred: false,
            s3_creds,
            progress: 0f64,
            children,
            error: None,
        }
    }
    /*
    let selected_item = LocalSelectedItem::new(
                    sr.name.clone(),
                    sr.path,
                    sr.is_directory,
                    selected_bucket,
                    destination_path,
                    self.props.current_s3_creds.clone(),
                    None,
                );
    let selected_item = LocalSelectedItem::new(
                sr.name,
                sr.path,
                sr.is_directory,
                "".to_string(),
                self.props.current_s3_path.clone(),
                self.props.current_s3_creds.clone(),
                None,
            );
     */

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
            children: None,
            error: None,
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
                                .to_string_lossy().into(),
                            transferred: false,
                            s3_creds: item.s3_creds.clone(),
                            progress: 0.0,
                            children: None,
                            error: None,
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
                                .to_string_lossy().into(),
                            transferred: false,
                            s3_creds: item.s3_creds.clone(),
                            progress: 0.0,
                            children: None,
                            error: None,
                        });
                    }
                }
            }
            items
        } else {
            vec![item.clone()]
        }
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
            children: None,
            error: None,
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
}