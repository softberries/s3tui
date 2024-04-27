#[derive(Debug, Clone)]
pub struct S3DataItem {
    pub bucket: Option<String>,
    pub name: String,
    pub size: String,
    pub file_type: String,
    pub path: String,
    pub is_directory: bool,
    pub is_bucket: bool,
}

impl S3DataItem {
    pub fn init(
        bucket: Option<String>,
        file_name: String,
        size: String,
        file_type: &str,
        path: &str,
        is_directory: bool,
        is_bucket: bool) -> S3DataItem {
        S3DataItem {
            bucket,
            name: file_name,
            size,
            file_type: String::from(file_type),
            path: String::from(path),
            is_directory,
            is_bucket,
        }
    }
    pub fn to_columns(&self) -> Vec<String> {
        vec![self.name.clone(), self.size.clone(), self.file_type.clone()]
    }
}