//! This module provides functionality for representing s3 data

/// Represents a file in s3 bucket
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub file_name: String,
    pub size: String,
    pub file_type: String,
    pub path: String,
    pub is_directory: bool,
}
/// Represents a bucket on s3
#[derive(Debug, Clone)]
pub struct BucketInfo {
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub is_bucket: bool,
}
/// Keeps the information about fetched data from s3
#[derive(Debug, Clone)]
pub struct S3DataItem {
    pub bucket: Option<String>,
    pub name: String,
    pub size: String,
    pub file_type: String,
    pub path: String,
    pub is_directory: bool,
    pub is_bucket: bool,
    pub region: Option<String>,
}

impl S3DataItem {
    pub fn init(bucket_info: BucketInfo, file_info: FileInfo) -> S3DataItem {
        S3DataItem {
            bucket: bucket_info.bucket,
            name: file_info.file_name,
            size: file_info.size,
            file_type: file_info.file_type,
            path: file_info.path,
            is_directory: file_info.is_directory,
            is_bucket: bucket_info.is_bucket,
            region: bucket_info.region,
        }
    }
    pub fn to_columns(&self) -> Vec<String> {
        vec![self.name.clone(), self.size.clone(), self.file_type.clone()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_s3_data_item_correctly() {
        let bucket_info = BucketInfo {
            bucket: Some("bucket".to_string()),
            region: Some("region".to_string()),
            is_bucket: true,
        };
        let file_info = FileInfo {
            file_name: "file_name.txt".into(),
            size: "23 MB".into(),
            file_type: "txt".into(),
            path: "/some/path".into(),
            is_directory: false,
        };
        let s3_data_item = S3DataItem::init(bucket_info.clone(), file_info.clone());
        assert_eq!(s3_data_item.bucket, bucket_info.bucket);
        assert_eq!(s3_data_item.region, bucket_info.region);
        assert_eq!(s3_data_item.is_bucket, bucket_info.is_bucket);
        assert_eq!(s3_data_item.name, file_info.file_name);
        assert_eq!(s3_data_item.file_type, file_info.file_type);
        assert_eq!(s3_data_item.path, file_info.path);
        assert_eq!(s3_data_item.is_directory, file_info.is_directory);
    }

    #[test]
    fn to_columns_get_correct_vector() {
        let bucket_info = BucketInfo {
            bucket: Some("bucket".to_string()),
            region: Some("region".to_string()),
            is_bucket: true,
        };
        let file_info = FileInfo {
            file_name: "file_name.txt".into(),
            size: "23 MB".into(),
            file_type: "txt".into(),
            path: "/some/path".into(),
            is_directory: false,
        };
        let s3_data_item = S3DataItem::init(bucket_info.clone(), file_info.clone());
        let res = s3_data_item.to_columns();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0], file_info.file_name);
        assert_eq!(res[1], file_info.size);
        assert_eq!(res[2], file_info.file_type);
    }
}
