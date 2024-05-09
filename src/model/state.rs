use url::Url;
use crate::model::download_progress_item::DownloadProgressItem;
use crate::model::local_data_item::LocalDataItem;
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::upload_progress_item::UploadProgressItem;
use crate::model::s3_data_item::S3DataItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::settings::file_credentials::FileCredential;
use percent_encoding::percent_decode;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ActivePage {
    #[default]
    FileManager,
    Transfers,
    S3Creds,
    Help,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub active_page: ActivePage,
    pub local_data: Vec<LocalDataItem>,
    pub s3_data: Vec<S3DataItem>,
    pub s3_loading: bool,
    pub s3_selected_items: Vec<S3SelectedItem>,
    pub local_selected_items: Vec<LocalSelectedItem>,
    pub current_local_path: String,
    pub current_s3_bucket: Option<String>,
    pub current_s3_path: Option<String>,
    pub creds: Vec<FileCredential>,
    pub current_creds: FileCredential
}

impl State {
    pub fn new(creds: Vec<FileCredential>) -> State {
        let st = State::default();
        if let Some(current_creds) = creds.iter().find(|cred| cred.selected) {
            State {
                creds: creds.clone(),
                current_creds: current_creds.to_owned(),
                ..st
            }
        } else {
            State {
                creds: creds.clone(),
                ..st
            }
        }

    }
    pub fn set_active_page(&mut self, page: ActivePage) {
        self.active_page = page;
    }
    pub fn update_selected_s3_transfers(&mut self, item: S3SelectedItem) {
        for it in self.s3_selected_items.iter_mut() {
            if it.name == item.name && item.error.is_none() {
                it.transferred = true;
                it.progress = 100f64;
            } else if it.name == item.name && item.error.is_some() {
                it.transferred = false;
                it.progress = 0f64;
                it.error = item.error.clone();
            }
        }
    }

    pub fn update_selected_local_transfers(&mut self, item: LocalSelectedItem) {
        for it in self.local_selected_items.iter_mut() {
            if it.name == item.name {
                it.transferred = true;
                it.progress = 100f64;
            }
        }
    }

    pub fn remove_already_transferred_items(&mut self) {
        self.s3_selected_items.retain(|it|
            !it.transferred
        );
        self.local_selected_items.retain(|it|
            !it.transferred
        );
    }

    pub fn update_buckets(&mut self, bucket: Option<String>, prefix: Option<String>, bucket_list: Vec<S3DataItem>) {
        self.s3_data = bucket_list;
        self.s3_loading = false;
        self.current_s3_bucket = bucket;
        self.current_s3_path = prefix;
    }

    pub fn update_files(&mut self, path: String, files: Vec<LocalDataItem>) {
        self.local_data = files;
        self.current_local_path = path;
    }

    pub fn set_current_local_path(&mut self, path: String) {
        self.current_local_path = path;
    }

    pub fn set_s3_loading(&mut self, loading: bool) {
        self.s3_loading = loading;
    }

    pub fn add_s3_selected_item(&mut self, item: S3SelectedItem) {
        self.s3_selected_items.push(item);
    }

    pub fn add_local_selected_item(&mut self, item: LocalSelectedItem) {
        self.local_selected_items.push(item);
    }

    pub fn remove_s3_selected_item(&mut self, item: S3SelectedItem) {
        self.s3_selected_items.retain(|it|
            it.bucket != item.bucket ||
                it.name != item.name ||
                it.path != item.path
        );
    }

    pub fn remove_local_selected_item(&mut self, item: LocalSelectedItem) {
        self.local_selected_items.retain(|it|
            it.name != item.name ||
                it.path != item.path
        );
    }

    pub fn set_current_s3_creds(&mut self, item: FileCredential) {
        for cred in self.creds.iter_mut() {
            if cred.name == item.name {
                cred.selected = true;
                self.current_creds = cred.clone();
            } else {
                cred.selected = false;
            }
        }
    }
    
    /*
    The url can look smth like this:
    "https://maluchyplywaja.s3.eu-west-1.amazonaws.com/IMG_8123.HEIC?x-id=PutObject"
     */
    fn update_local_item_with_progress(selected_items: &mut [LocalSelectedItem], progress_item: UploadProgressItem) {
        let url = match Url::parse(progress_item.uri.as_str()) {
            Ok(url) => url,
            Err(_) => return, // Exit the function if URL parsing fails
        };

        let host = url.host_str().unwrap_or_default();
        let path_segments = url.path_segments().map(|c| c.collect::<Vec<_>>()).unwrap_or_default();
        let name = path_segments.last().unwrap_or(&"");

        // Assume bucket name is the first segment of the host
        let bucket_parts = host.split('.').collect::<Vec<_>>();
        let bucket_name = bucket_parts.first().unwrap_or(&"");

        for item in selected_items.iter_mut() {
            let encoded_name = percent_decode(name.as_bytes())
                .decode_utf8() // This returns a `Result<Cow<str>, Utf8Error>`
                .unwrap_or_else(|e| panic!("Decoding error: {}", e)) // Handle the error case
                .to_string(); // Convert `Cow<str>` to `String`
            let name = String::from(&item.name);
            if &item.destination_bucket == bucket_name && name == encoded_name {
                item.progress = progress_item.progress;
            }
        }
    }

    fn update_s3_item_with_progress(selected_items: &mut [S3SelectedItem], progress_item: DownloadProgressItem) {
        for item in selected_items.iter_mut() {
            if item.name == progress_item.name && item.bucket == Some(progress_item.bucket.clone()) {
                item.progress = progress_item.progress;
            }
        }
    }

    pub fn update_progress_on_selected_local_item(&mut self, item: UploadProgressItem) {
        Self::update_local_item_with_progress(&mut self.local_selected_items, item.clone());
    }

    pub fn update_progress_on_selected_s3_item(&mut self, item: DownloadProgressItem) {
        Self::update_s3_item_with_progress(&mut self.s3_selected_items, item.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_active_page_is_file_manager_page() {
        let state = State::default();
        assert_eq!(state.active_page, ActivePage::FileManager);
    }

    #[test]
    fn set_active_page_changes_page_correctly() {
        let mut state = State::default();
        state.set_active_page(ActivePage::Help);
        assert_eq!(state.active_page, ActivePage::Help);
    }

    #[test]
    fn new_state_with_selected_credential_sets_current_creds_correctly() {
        let creds = vec![
            FileCredential { name: "AWS".into(), access_key: "".to_string(), secret_key: "".to_string(), default_region: "".to_string(), selected: false },
            FileCredential { name: "Azure".into(), access_key: "".to_string(), secret_key: "".to_string(), default_region: "".to_string(), selected: true }
        ];
        let state = State::new(creds.clone());
        assert_eq!(state.current_creds, creds[1]);
    }

    #[test]
    fn add_and_remove_s3_selected_item_works() {
        let mut state = State::default();
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

        state.add_s3_selected_item(item.clone());
        assert_eq!(state.s3_selected_items.len(), 1);
        assert_eq!(state.s3_selected_items[0], item);

        state.remove_s3_selected_item(item);
        assert!(state.s3_selected_items.is_empty());
    }

    #[test]
    fn update_progress_on_selected_local_item_updates_correctly() {
        let mut state = State::default();
        let selected_item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            transferred: false,
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            progress: 0.0,
            is_directory: false,
            s3_creds: Default::default(),
        };

        state.local_selected_items.push(selected_item.clone());
        let progress_item = UploadProgressItem {
            progress: 0.5,
            uri: "https://test-bucket.s3.amazonaws.com/path/to/file1.txt".into()
        };
        state.update_progress_on_selected_local_item(progress_item);
        
        assert_eq!(state.local_selected_items[0].progress, 0.5);
    }
}
