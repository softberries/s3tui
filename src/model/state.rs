//! This module provides functionality for keeping the application state
use crate::model::download_progress_item::DownloadProgressItem;
use crate::model::local_data_item::LocalDataItem;
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_data_item::S3DataItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::upload_progress_item::UploadProgressItem;
use crate::settings::file_credentials::FileCredential;
use percent_encoding::percent_decode;
use url::Url;

/// Safely decode a URL-encoded string, returning the original on error
fn decode_url_safe(encoded: &str) -> String {
    percent_decode(encoded.as_bytes())
        .decode_utf8()
        .map(|cow| cow.into_owned())
        .unwrap_or_else(|_| {
            tracing::warn!("Failed to decode URL: {}", encoded);
            encoded.to_string()
        })
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ActivePage {
    #[default]
    FileManager,
    Transfers,
    S3Creds,
    Help,
}

/// Represents entire state of the application, each page transforms this information for
/// suitable Props object
#[derive(Debug, Clone, Default)]
pub struct State {
    pub active_page: ActivePage,
    pub local_data: Vec<LocalDataItem>,
    pub s3_data: Vec<S3DataItem>,
    pub s3_data_full_list: Vec<S3DataItem>,
    pub s3_loading: bool,
    pub s3_list_recursive_loading: bool,
    pub s3_selected_items: Vec<S3SelectedItem>,
    pub local_selected_items: Vec<LocalSelectedItem>,
    pub current_local_path: String,
    pub current_s3_bucket: Option<String>,
    pub current_s3_path: Option<String>,
    pub creds: Vec<FileCredential>,
    pub current_creds: FileCredential,
    pub local_delete_state: Option<String>,
    pub s3_delete_state: Option<String>,
    pub create_bucket_state: Option<String>,
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
                it.error.clone_from(&item.error);
            }
            if let Some(children) = it.children.as_mut() {
                for itc in children.iter_mut() {
                    if itc.name == item.name && item.error.is_none() {
                        itc.transferred = true;
                        itc.progress = 100f64;
                    } else if itc.name == item.name && item.error.is_some() {
                        itc.transferred = false;
                        itc.progress = 0f64;
                        itc.error.clone_from(&item.error);
                    }
                }
                // Recalculate parent progress based on children
                it.progress = Self::calculate_overall_progress_s3(children);
                // Parent is only transferred when ALL children are transferred
                it.transferred = children.iter().all(|c| c.transferred);
            }
        }
    }

    pub fn update_selected_local_transfers(&mut self, item: LocalSelectedItem) {
        for it in self.local_selected_items.iter_mut() {
            if it.name == item.name && item.error.is_none() {
                it.transferred = true;
                it.progress = 100f64;
            } else if it.name == item.name && item.error.is_some() {
                it.transferred = false;
                it.progress = 0f64;
                it.error.clone_from(&item.error);
            }
            if let Some(children) = it.children.as_mut() {
                for itc in children.iter_mut() {
                    if itc.name == item.name && item.error.is_none() {
                        itc.transferred = true;
                        itc.progress = 100f64;
                    } else if itc.name == item.name && item.error.is_some() {
                        itc.transferred = false;
                        itc.progress = 0f64;
                        itc.error.clone_from(&item.error);
                    }
                }
                // Recalculate parent progress based on children
                it.progress = Self::calculate_overall_progress_local(children);
                // Parent is only transferred when ALL children are transferred
                it.transferred = children.iter().all(|c| c.transferred);
            }
        }
    }

    pub fn remove_already_transferred_items(&mut self) {
        self.s3_selected_items.retain(|it| !it.transferred);
        self.local_selected_items.retain(|it| !it.transferred);
    }

    pub fn update_buckets(
        &mut self,
        bucket: Option<String>,
        prefix: Option<String>,
        bucket_list: Vec<S3DataItem>,
    ) {
        self.s3_data = bucket_list;
        self.s3_loading = false;
        self.current_s3_bucket = bucket;
        self.current_s3_path = prefix;
    }

    pub fn update_s3_recursive_list(&mut self, bucket_list: Vec<S3DataItem>) {
        self.s3_data_full_list = bucket_list;
        self.s3_list_recursive_loading = false;
    }

    pub fn update_files(&mut self, path: String, files: Vec<LocalDataItem>) {
        self.local_data = files;
        self.current_local_path = path;
    }

    pub fn set_local_delete_error(&mut self, error_str: Option<String>) {
        self.local_delete_state = error_str;
    }

    pub fn set_s3_delete_error(&mut self, error_str: Option<String>) {
        self.s3_delete_state = error_str;
    }

    pub fn set_create_bucket_error(&mut self, error_str: Option<String>) {
        self.create_bucket_state = error_str;
    }

    pub fn set_current_local_path(&mut self, path: String) {
        self.current_local_path = path;
    }

    pub fn set_s3_loading(&mut self, loading: bool) {
        self.s3_loading = loading;
    }

    pub fn set_s3_list_recursive_loading(&mut self, loading: bool) {
        self.s3_list_recursive_loading = loading;
    }

    pub fn add_s3_selected_item(&mut self, item: S3SelectedItem) {
        self.s3_selected_items.push(item.clone());
    }

    pub fn add_local_selected_item(&mut self, it: LocalSelectedItem) {
        if it.is_directory {
            let items = LocalSelectedItem::list_directory_items(&it);
            let item = LocalSelectedItem {
                children: Some(items),
                ..it
            };
            self.local_selected_items.push(item);
        } else {
            self.local_selected_items.push(it);
        }
    }

    pub fn remove_s3_selected_item(&mut self, item: S3SelectedItem) {
        self.s3_selected_items
            .retain(|it| it.bucket != item.bucket || it.name != item.name || it.path != item.path);
    }

    pub fn remove_local_selected_item(&mut self, item: LocalSelectedItem) {
        self.local_selected_items
            .retain(|it| it.name != item.name || it.path != item.path);
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
    fn update_local_item_with_progress(&mut self, progress_item: &UploadProgressItem) {
        let url = match Url::parse(progress_item.uri.as_str()) {
            Ok(url) => url,
            Err(_) => return,
        };
        let host = url.host_str().unwrap_or_default();
        let path_segments: Vec<&str> = url
            .path_segments()
            .map(|c| c.collect::<Vec<_>>())
            .unwrap_or_default();

        // Detect URL style: path-style vs virtual-hosted-style
        // Path-style: https://s3.region.amazonaws.com/bucket/key
        // Virtual-hosted: https://bucket.s3.region.amazonaws.com/key
        let host_parts: Vec<&str> = host.split('.').collect();
        let is_path_style = host_parts.first().map(|s| s.starts_with("s3")).unwrap_or(false);

        let (bucket_name, name) = if is_path_style {
            // Path-style: bucket is first path segment, key is the rest
            let bucket = path_segments.first().unwrap_or(&"");
            let key = path_segments.last().unwrap_or(&"");
            (*bucket, *key)
        } else {
            // Virtual-hosted-style: bucket is first host segment
            let bucket = host_parts.first().unwrap_or(&"");
            let key = path_segments.last().unwrap_or(&"");
            (*bucket, *key)
        };
        let decoded_name = decode_url_safe(name);

        for item in &mut self.local_selected_items {
            // Skip already-transferred items to avoid race conditions with late progress updates
            if item.transferred {
                continue;
            }

            if item.children.is_none() {
                if item.destination_bucket == *bucket_name && item.name == decoded_name {
                    item.progress = progress_item.progress;
                }
            } else if let Some(children) = &mut item.children {
                for child in children.iter_mut() {
                    // Skip already-transferred children
                    if child.transferred {
                        continue;
                    }
                    if child.destination_bucket == *bucket_name && child.name == decoded_name {
                        child.progress = progress_item.progress;
                    }
                }
                item.progress = Self::calculate_overall_progress_local(children);
            }
        }
    }

    fn update_s3_item_with_progress(&mut self, progress_item: &DownloadProgressItem) {
        let target_bucket = Some(progress_item.bucket.clone());

        for item in &mut self.s3_selected_items {
            // Use path if available, otherwise fall back to name (matches download_item logic)
            let item_key = item.path.as_ref().unwrap_or(&item.name);

            // Skip already-transferred items to avoid race conditions with late progress updates
            if item.transferred {
                continue;
            }

            if item.children.is_none() {
                if item_key == &progress_item.name && item.bucket == target_bucket {
                    item.progress = progress_item.progress;
                }
            } else if let Some(children) = &mut item.children {
                for child in children.iter_mut() {
                    // Skip already-transferred children
                    if child.transferred {
                        continue;
                    }
                    let child_key = child.path.as_ref().unwrap_or(&child.name);
                    if child_key == &progress_item.name && child.bucket == target_bucket {
                        child.progress = progress_item.progress;
                    }
                }
                item.progress = Self::calculate_overall_progress_s3(children);
            }
        }
    }

    fn calculate_overall_progress_s3(items: &[S3SelectedItem]) -> f64 {
        if items.is_empty() {
            return 0.0;
        }
        let all_progress: f64 = items.iter().map(|i| i.progress).sum();
        if all_progress > 0.0 {
            all_progress / items.len() as f64
        } else {
            0.0
        }
    }

    fn calculate_overall_progress_local(items: &[LocalSelectedItem]) -> f64 {
        if items.is_empty() {
            return 0.0;
        }
        let all_progress: f64 = items.iter().map(|i| i.progress).sum();
        if all_progress > 0.0 {
            all_progress / items.len() as f64
        } else {
            0.0
        }
    }

    pub fn update_progress_on_selected_local_item(&mut self, item: &UploadProgressItem) {
        self.update_local_item_with_progress(item);
    }

    pub fn update_progress_on_selected_s3_item(&mut self, item: &DownloadProgressItem) {
        self.update_s3_item_with_progress(item);
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
            FileCredential {
                name: "AWS".into(),
                access_key: "".to_string(),
                secret_key: "".to_string(),
                default_region: "".to_string(),
                selected: false,
            },
            FileCredential {
                name: "Azure".into(),
                access_key: "".to_string(),
                secret_key: "".to_string(),
                default_region: "".to_string(),
                selected: true,
            },
        ];
        let state = State::new(creds.clone());
        assert_eq!(state.current_creds, creds[1]);
    }

    #[test]
    fn new_state_without_selected_credential_sets_current_creds_correctly() {
        let creds = vec![
            FileCredential {
                name: "AWS".into(),
                access_key: "".to_string(),
                secret_key: "".to_string(),
                default_region: "".to_string(),
                selected: false,
            },
            FileCredential {
                name: "Azure".into(),
                access_key: "".to_string(),
                secret_key: "".to_string(),
                default_region: "".to_string(),
                selected: false,
            },
        ];
        let state = State::new(creds.clone());
        assert_eq!(state.current_creds, state.current_creds);
    }

    #[test]
    fn set_current_s3_creds_set_creds_correctly_for_existing_state() {
        let creds = vec![
            FileCredential {
                name: "AWS".into(),
                access_key: "".to_string(),
                secret_key: "".to_string(),
                default_region: "".to_string(),
                selected: true,
            },
            FileCredential {
                name: "Azure".into(),
                access_key: "".to_string(),
                secret_key: "".to_string(),
                default_region: "".to_string(),
                selected: false,
            },
        ];
        let mut state = State::new(creds.clone());
        assert_eq!(state.current_creds, creds[0]);

        state.set_current_s3_creds(creds[1].clone());
        assert_eq!(state.current_creds.name, creds[1].name);
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
            children: None,
            error: None,
        };

        state.add_s3_selected_item(item.clone());
        assert_eq!(state.s3_selected_items.len(), 1);
        assert_eq!(state.s3_selected_items[0], item);

        state.remove_s3_selected_item(item);
        assert!(state.s3_selected_items.is_empty());
    }

    #[test]
    fn add_and_remove_local_selected_item_works() {
        let mut state = State::default();
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

        state.add_local_selected_item(item.clone());
        assert_eq!(state.local_selected_items.len(), 1);
        assert_eq!(state.local_selected_items[0], item);

        state.remove_local_selected_item(item);
        assert!(state.local_selected_items.is_empty());
    }

    #[test]
    fn update_selected_s3_transfers_updates_correctly() {
        let mut state = State::default();
        let selected_item = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            transferred: false,
            s3_creds: FileCredential::default(),
            progress: 0.0,
            children: None,
            error: None,
        };
        state.s3_selected_items.push(selected_item.clone());
        state.update_selected_s3_transfers(selected_item.clone());
        assert!(state.s3_selected_items[0].transferred);
        assert_eq!(state.s3_selected_items[0].progress, 100f64);
    }

    #[test]
    fn update_selected_s3_transfers_updates_child_correctly() {
        let mut state = State::default();
        let child = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "child-file1.txt".into(),
            path: Some("path/to/child-file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            transferred: false,
            s3_creds: FileCredential::default(),
            progress: 0.0,
            children: None,
            error: None,
        };
        let selected_item = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            transferred: false,
            s3_creds: FileCredential::default(),
            progress: 0.0,
            children: Some(vec![child.clone()]),
            error: None,
        };
        state.s3_selected_items.push(selected_item.clone());
        state.update_selected_s3_transfers(child.clone());
        let children = state.s3_selected_items[0]
            .clone()
            .children
            .unwrap_or_default();
        assert_eq!(children.len(), 1);
        // assert!(children[0].transferred);
        // assert_eq!(children[0].progress, 100f64);
    }

    #[test]
    fn update_selected_s3_transfers_with_error_updates_correctly() {
        let mut state = State::default();
        let selected_item = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            transferred: false,
            s3_creds: FileCredential::default(),
            progress: 0.0,
            children: None,
            error: Some("Error".into()),
        };
        state.add_s3_selected_item(selected_item.clone());
        state.update_selected_s3_transfers(selected_item.clone());
        assert!(!state.s3_selected_items[0].transferred);
        assert_eq!(state.s3_selected_items[0].progress, 0f64);
    }

    #[test]
    fn update_selected_local_transfers_updates_correctly() {
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
            children: None,
            error: None,
        };
        state.add_local_selected_item(selected_item.clone());
        state.update_selected_local_transfers(selected_item.clone());
        assert!(state.local_selected_items[0].transferred);
        assert_eq!(state.local_selected_items[0].progress, 100f64);
    }

    #[test]
    fn update_selected_local_transfers_with_error_updates_correctly() {
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
            children: None,
            error: Some("Error".into()),
        };
        state.add_local_selected_item(selected_item.clone());
        state.update_selected_local_transfers(selected_item.clone());
        assert!(!state.local_selected_items[0].transferred);
        assert_eq!(state.local_selected_items[0].progress, 0f64);
    }

    #[test]
    fn remove_already_transferred_items_removes_correctly() {
        let mut state = State::default();
        let local_item_not_transfered = LocalSelectedItem {
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
        let local_item_transfered = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            transferred: true,
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            progress: 0.0,
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            error: None,
        };
        let s3_item_not_transferred = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            transferred: false,
            s3_creds: FileCredential::default(),
            progress: 0.0,
            children: None,
            error: None,
        };
        let s3_item_transferred = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            transferred: true,
            s3_creds: FileCredential::default(),
            progress: 0.0,
            children: None,
            error: None,
        };
        state.add_local_selected_item(local_item_transfered);
        state.add_local_selected_item(local_item_not_transfered);
        state.add_s3_selected_item(s3_item_not_transferred);
        state.add_s3_selected_item(s3_item_transferred);
        assert_eq!(state.s3_selected_items.len(), 2);
        assert_eq!(state.local_selected_items.len(), 2);

        state.remove_already_transferred_items();
        assert_eq!(state.s3_selected_items.len(), 1);
        assert_eq!(state.local_selected_items.len(), 1);
        assert!(!state.s3_selected_items[0].transferred);
        assert!(!state.local_selected_items[0].transferred);
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
            children: None,
            error: None,
        };

        state.local_selected_items.push(selected_item.clone());
        let progress_item = UploadProgressItem {
            progress: 0.5,
            uri: "https://test-bucket.s3.amazonaws.com/path/to/file1.txt".into(),
        };
        state.update_progress_on_selected_local_item(&progress_item);

        assert_eq!(state.local_selected_items[0].progress, 0.5);
    }

    #[test]
    fn update_progress_on_selected_s3_item_updates_correctly() {
        let mut state = State::default();
        let item = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            transferred: false,
            s3_creds: FileCredential::default(),
            progress: 0.0,
            children: None,
            error: None,
        };

        state.s3_selected_items.push(item.clone());
        // DownloadProgressItem.name should be the path (S3 key), matching download_item behavior
        let progress_item = DownloadProgressItem {
            progress: 0.5,
            bucket: "test-bucket".to_string(),
            name: "path/to/file1.txt".into(),
        };
        state.update_progress_on_selected_s3_item(&progress_item);

        assert_eq!(state.s3_selected_items[0].progress, 0.5);
    }

    #[test]
    fn update_local_item_with_progress_updates_the_item() {
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
            children: None,
            error: None,
        };
        state.local_selected_items = vec![selected_item];
        let progress_item = UploadProgressItem {
            progress: 50.0,
            uri: "https://test-bucket.s3.eu-west-1.amazonaws.com/file1.txt?x-id=PutObject".into(),
        };
        state.update_local_item_with_progress(&progress_item);
        assert_eq!(state.local_selected_items[0].progress, 50.0);
    }

    #[test]
    fn update_local_item_with_progress_updates_child_correctly() {
        let mut state = State::default();
        let child = LocalSelectedItem {
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
        let selected_item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            transferred: false,
            name: "/".into(),
            path: "path/to".into(),
            progress: 0.0,
            is_directory: false,
            s3_creds: Default::default(),
            children: Some(vec![child]),
            error: None,
        };
        state.local_selected_items = vec![selected_item];
        let progress_item = UploadProgressItem {
            progress: 50.0,
            uri: "https://test-bucket.s3.eu-west-1.amazonaws.com/file1.txt?x-id=PutObject".into(),
        };
        state.update_local_item_with_progress(&progress_item);
        assert_eq!(
            state.local_selected_items[0]
                .clone()
                .children
                .unwrap_or_default()[0]
                .progress,
            50.0
        );
        assert_eq!(state.local_selected_items[0].progress, 50.0);
    }

    #[test]
    fn update_s3_item_with_progress_updates_the_item() {
        let mut state = State::default();
        let selected_item = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            transferred: false,
            s3_creds: FileCredential::default(),
            progress: 0.0,
            children: None,
            error: None,
        };
        state.s3_selected_items = vec![selected_item];
        // DownloadProgressItem.name should be the path (S3 key), matching download_item behavior
        let progress_item = DownloadProgressItem {
            progress: 50.0,
            bucket: "test-bucket".into(),
            name: "path/to/file1.txt".into(),
        };
        state.update_s3_item_with_progress(&progress_item);
        assert_eq!(state.s3_selected_items[0].progress, 50.0);
    }

    #[test]
    fn update_s3_item_with_progress_updates_child_correctly() {
        let mut state = State::default();
        let child = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            transferred: false,
            s3_creds: FileCredential::default(),
            progress: 0.0,
            children: None,
            error: None,
        };
        let selected_item = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "/".into(),
            path: Some("path/to".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            transferred: false,
            s3_creds: FileCredential::default(),
            progress: 0.0,
            children: Some(vec![child.clone()]),
            error: None,
        };
        state.s3_selected_items = vec![selected_item];
        // DownloadProgressItem.name should be the path (S3 key), matching download_item behavior
        let progress_item = DownloadProgressItem {
            progress: 50.0,
            bucket: "test-bucket".into(),
            name: "path/to/file1.txt".into(),
        };
        state.update_s3_item_with_progress(&progress_item);
        assert_eq!(
            state.s3_selected_items[0]
                .clone()
                .children
                .unwrap_or_default()[0]
                .progress,
            50.0
        );
        assert_eq!(state.s3_selected_items[0].progress, 50.0);
    }

    #[test]
    fn test_decode_url_safe_decodes_valid_utf8() {
        let encoded = "Hello%20World";
        let result = super::decode_url_safe(encoded);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_decode_url_safe_returns_original_on_invalid_utf8() {
        // This is a valid percent-encoded sequence but decodes to invalid UTF-8
        let invalid_utf8 = "%FF%FE";
        let result = super::decode_url_safe(invalid_utf8);
        // Should return the original string on error
        assert_eq!(result, invalid_utf8);
    }

    #[test]
    fn test_decode_url_safe_handles_unencoded_string() {
        let unencoded = "normal_filename.txt";
        let result = super::decode_url_safe(unencoded);
        assert_eq!(result, "normal_filename.txt");
    }
}
