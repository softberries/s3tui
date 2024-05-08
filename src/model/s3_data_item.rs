pub struct FileInfo {
    pub file_name: String,
    pub size: String,
    pub file_type: String,
    pub path: String,
    pub is_directory: bool,
}

pub struct BucketInfo {
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub is_bucket: bool,
}

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
    pub fn init(
        bucket_info: BucketInfo,
        file_info: FileInfo) -> S3DataItem {
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