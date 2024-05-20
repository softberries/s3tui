/// Represents an item (file/directory) on your local machine
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_local_data_item_correctly() {
        let item = LocalDataItem {
            name: "name".into(),
            size: "1 MB".into(),
            file_type: "txt".into(),
            path: "/some/path".into(),
            is_directory: false,
        };
        let local_data_item = LocalDataItem::init("name".into(), "1 MB".into(), "txt", "/some/path", false);
        assert_eq!(local_data_item.name, item.name);
        assert_eq!(local_data_item.size, item.size);
        assert_eq!(local_data_item.is_directory, item.is_directory);
        assert_eq!(local_data_item.path, item.path);
        assert_eq!(local_data_item.file_type, item.file_type);
    }

    #[test]
    fn to_columns_get_correct_vector() {
        let item = LocalDataItem {
            name: "name".into(),
            size: "1 MB".into(),
            file_type: "txt".into(),
            path: "/some/path".into(),
            is_directory: false,
        };
        let res = item.to_columns();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0], item.name);
        assert_eq!(res[1], item.size);
        assert_eq!(res[2], item.file_type);
    }
}