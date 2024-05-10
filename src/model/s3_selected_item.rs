use crate::model::s3_data_item::S3DataItem;
use crate::settings::file_credentials::FileCredential;

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

    pub fn to_columns(&self) -> Vec<String> {
        let progress = format!("{:.2}%", self.progress);
        if self.is_bucket {
            vec![self.name.clone(), "/".to_string(), self.destination_dir.clone(), self.s3_creds.name.clone(), progress, self.error.clone().unwrap_or("".to_string())]
        } else {
            vec![self.bucket.clone().unwrap_or("".to_string()), self.name.clone(), self.destination_dir.clone(), self.s3_creds.name.clone(), progress, self.error.clone().unwrap_or("".to_string())]
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

    #[test]
    fn to_columns_get_correct_vector() {
        let item = S3SelectedItem::new(
            "file1.txt".into(),
            Some("test-bucket".into()),
            Some("path/to/file1.txt".into()),
            false,
            false,
            "".into(),
            Default::default()
        );
        let res = item.to_columns();
        assert_eq!(res.len(), 6);
        assert_eq!(res[0], item.bucket.unwrap());
        assert_eq!(res[1], item.name);
        assert_eq!(res[2], item.destination_dir);
        assert_eq!(res[3], item.s3_creds.name);
        assert_eq!(res[4], "0.00%".to_string());
        assert_eq!(res[5], "".to_string());
    }

    #[test]
    fn to_columns_with_error_get_correct_vector() {
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
            error: Some("Error".into())
        };
        let res = item.to_columns();
        assert_eq!(res.len(), 6);
        assert_eq!(res[0], item.bucket.unwrap());
        assert_eq!(res[1], item.name);
        assert_eq!(res[2], item.destination_dir);
        assert_eq!(res[3], item.s3_creds.name);
        assert_eq!(res[4], "0.00%".to_string());
        assert_eq!(res[5], "Error".to_string());
    }

    #[test]
    fn to_columns_for_bucket_get_correct_vector() {
        let item = S3SelectedItem::new(
            "file1.txt".into(),
            Some("test-bucket".into()),
            Some("path/to/file1.txt".into()),
            false,
            true,
            "".into(),
            Default::default()
        );
        let res = item.to_columns();

        assert_eq!(res.len(), 6);
        assert_eq!(res[0], item.name);
        assert_eq!(res[1], "/");
        assert_eq!(res[2], item.destination_dir);
        assert_eq!(res[3], item.s3_creds.name);
        assert_eq!(res[4], "0.00%".to_string());
        assert_eq!(res[5], "".to_string());
    }

    #[test]
    fn to_columns_for_bucket_with_error_get_correct_vector() {
        let item = S3SelectedItem {
            bucket: Some("test-bucket".into()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: true,
            destination_dir: "".to_string(),
            transferred: false,
            s3_creds: Default::default(),
            progress: 0f64,
            error: Some("Error".into())
        };
        let res = item.to_columns();

        assert_eq!(res.len(), 6);
        assert_eq!(res[0], item.name);
        assert_eq!(res[1], "/");
        assert_eq!(res[2], item.destination_dir);
        assert_eq!(res[3], item.s3_creds.name);
        assert_eq!(res[4], "0.00%".to_string());
        assert_eq!(res[5], "Error".to_string());
    }
}