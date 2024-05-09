use std::path::Path;
use color_eyre::Result;
use tokio::fs;
use humansize::{FileSize, file_size_opts as options};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::model::local_data_item::LocalDataItem;

#[derive(Clone, Default)]
pub struct LocalDataFetcher {
    current_dir: Arc<Mutex<String>>,
}

impl LocalDataFetcher {
    pub fn new() -> Self {
        LocalDataFetcher {
            current_dir: Arc::new(Mutex::new(String::new()))
        }
    }

    async fn get_current_dir(&self) -> String {
        // Lock the mutex and clone the string to return it
        let current_dir = self.current_dir.lock().await;
        current_dir.clone()
    }

    pub async fn read_parent_directory(&self) -> Result<Vec<LocalDataItem>> {
        let current_dir = self.get_current_dir().await;
        let path = Path::new(&current_dir);
        let parent_path = match path.parent() {
            Some(p_path) => p_path.to_path_buf(),
            None => path.to_path_buf()
        };
        let parent_path_cow = parent_path.to_string_lossy();
        self.read_directory(Some(String::from(parent_path_cow.as_ref()))).await
    }

    pub async fn read_directory(&self, absolute_path_str: Option<String>) -> Result<Vec<LocalDataItem>> {
        let mut files_info = Vec::new();
        let home_dir = dirs::home_dir().unwrap();
        if let Some(path) = absolute_path_str {
            let mut current_dir = self.current_dir.lock().await;
            *current_dir = path;
        } else {
            let mut current_dir = self.current_dir.lock().await;
            let path_cow = home_dir.to_string_lossy();
            let path_str = path_cow.as_ref();
            *current_dir = String::from(path_str);
        }
        let mut entries = fs::read_dir(self.get_current_dir().await).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;

            let file_name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let extension_cow = path.extension()
                .unwrap_or_default()
                .to_string_lossy();
            let extension = extension_cow.as_ref();
            let path_cow = path.to_string_lossy();
            let path_str = path_cow.as_ref();
            let is_directory = metadata.is_dir();
            let size = metadata.len().file_size(options::CONVENTIONAL).unwrap_or_else(|_| "0 B".to_string());
            let file_type = if is_directory { "Dir" } else { extension };

            files_info.push(LocalDataItem::init(file_name, size, file_type, path_str, is_directory));
        }

        Ok(files_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs::{self, File};
    use tokio::io::AsyncWriteExt;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_new() {
        let fetcher = LocalDataFetcher::new();
        assert!(fetcher.current_dir.lock().await.is_empty(), "Initial directory should be empty");
    }

    #[tokio::test]
    async fn test_get_current_dir() {
        let fetcher = LocalDataFetcher::new();
        assert_eq!(fetcher.get_current_dir().await, "", "Should return the initial empty directory");
    }

    #[tokio::test]
    async fn test_read_parent_directory() -> color_eyre::Result<()> {
        let dir = tempdir()?;
        let sub_dir = dir.path().join("subdir");
        fs::create_dir(&sub_dir).await?;

        let fetcher = LocalDataFetcher::new();
        {
            let mut current_dir = fetcher.current_dir.lock().await;
            *current_dir = sub_dir.to_str().unwrap().to_string();
        }

        let parent_dir_files = fetcher.read_parent_directory().await?;
        assert_eq!(parent_dir_files.len(), 1, "Should contain one directory entry");
        assert!(parent_dir_files.iter().any(|f| f.name == "subdir"), "Should include the subdir");
        Ok(())
    }

    #[tokio::test]
    async fn test_read_directory() -> color_eyre::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("file.txt");
        let mut file = File::create(&file_path).await?;
        file.write_all(b"Hello, world!").await?;

        let fetcher = LocalDataFetcher::new();
        {
            let mut current_dir = fetcher.current_dir.lock().await;
            *current_dir = dir.path().to_str().unwrap().to_string();
        }

        let files = fetcher.read_directory(Some(fetcher.get_current_dir().await)).await?;
        assert_eq!(files.len(), 1, "Should contain one file entry");
        assert!(files.iter().any(|f| f.name == "file.txt" && f.is_directory == false), "Should correctly identify the file");

        Ok(())
    }

    #[tokio::test]
    async fn test_read_home_directory() -> color_eyre::Result<()> {

        let fetcher = LocalDataFetcher::new();
       
        let files = fetcher.read_directory(None).await?;
        assert!(!files.is_empty(), "Should contain multiple files");
        Ok(())
    }
}
