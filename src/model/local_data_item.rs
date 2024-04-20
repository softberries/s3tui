#[derive(Debug, Clone)]
pub struct LocalDataItem {
    pub name: String,
    pub size: String,
    pub file_type: String,
    pub path: String,
    pub is_directory: bool,
}

impl LocalDataItem {
    pub fn init(file_name: String,
            size: String,
            file_type: &str,
            path: &str,
            is_directory: bool) -> LocalDataItem {
        LocalDataItem {
            name: file_name,
            size,
            file_type: String::from(file_type),
            path: String::from(path),
            is_directory,
        }
    }
    pub fn to_columns(&self) -> Vec<String> {
        vec![self.name.clone(), self.size.clone(), self.file_type.clone()]
    }
}