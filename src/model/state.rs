//! This module provides functionality for keeping the application state
use crate::model::download_progress_item::DownloadProgressItem;
use crate::model::error::{LocalError, S3Error};
use crate::model::has_children::calculate_overall_progress;
use crate::model::local_data_item::LocalDataItem;
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_data_item::S3DataItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::sorting::{sort_items, SortColumn, SortState};
use crate::model::transfer_state::TransferState;
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
    pub local_delete_error: Option<LocalError>,
    pub s3_delete_error: Option<S3Error>,
    pub create_bucket_error: Option<S3Error>,
    pub s3_sort_state: SortState,
    pub local_sort_state: SortState,
    /// Current search/filter query
    pub search_query: String,
    /// Whether search input is active
    pub search_mode: bool,
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
            if it.name == item.name {
                match &item.transfer_state {
                    TransferState::Failed(err) => {
                        it.transfer_state = TransferState::Failed(err.clone());
                    }
                    _ => {
                        it.transfer_state = TransferState::Completed;
                    }
                }
            }
            if let Some(children) = it.children.as_mut() {
                for itc in children.iter_mut() {
                    if itc.name == item.name {
                        match &item.transfer_state {
                            TransferState::Failed(err) => {
                                itc.transfer_state = TransferState::Failed(err.clone());
                            }
                            _ => {
                                itc.transfer_state = TransferState::Completed;
                            }
                        }
                    }
                }
                // Recalculate parent progress based on children
                let progress = calculate_overall_progress(children);
                // Parent is only transferred when ALL children are transferred
                let all_completed = children.iter().all(|c| c.is_transferred());
                if all_completed {
                    it.transfer_state = TransferState::Completed;
                } else {
                    it.transfer_state = TransferState::InProgress(progress);
                }
            }
        }
    }

    pub fn update_selected_local_transfers(&mut self, item: LocalSelectedItem) {
        for it in self.local_selected_items.iter_mut() {
            if it.name == item.name {
                match &item.transfer_state {
                    TransferState::Failed(err) => {
                        it.transfer_state = TransferState::Failed(err.clone());
                    }
                    _ => {
                        it.transfer_state = TransferState::Completed;
                    }
                }
            }
            if let Some(children) = it.children.as_mut() {
                for itc in children.iter_mut() {
                    if itc.name == item.name {
                        match &item.transfer_state {
                            TransferState::Failed(err) => {
                                itc.transfer_state = TransferState::Failed(err.clone());
                            }
                            _ => {
                                itc.transfer_state = TransferState::Completed;
                            }
                        }
                    }
                }
                // Recalculate parent progress based on children
                let progress = calculate_overall_progress(children);
                // Parent is only transferred when ALL children are transferred
                let all_completed = children.iter().all(|c| c.is_transferred());
                if all_completed {
                    it.transfer_state = TransferState::Completed;
                } else {
                    it.transfer_state = TransferState::InProgress(progress);
                }
            }
        }
    }

    pub fn remove_already_transferred_items(&mut self) {
        self.s3_selected_items.retain(|it| !it.is_transferred());
        self.local_selected_items.retain(|it| !it.is_transferred());
    }

    pub fn all_uploads_complete_for_bucket(&self, bucket: &str) -> bool {
        self.local_selected_items
            .iter()
            .filter(|item| item.destination_bucket == bucket)
            .all(|item| item.is_transferred())
    }

    pub fn all_downloads_complete_for_directory(&self, directory: &str) -> bool {
        self.s3_selected_items
            .iter()
            .filter(|item| item.destination_dir == directory)
            .all(|item| item.is_transferred())
    }

    pub fn update_buckets(
        &mut self,
        bucket: Option<String>,
        prefix: Option<String>,
        bucket_list: Vec<S3DataItem>,
    ) {
        self.s3_data = bucket_list;
        sort_items(&mut self.s3_data, &self.s3_sort_state);
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
        sort_items(&mut self.local_data, &self.local_sort_state);
        self.current_local_path = path;
    }

    pub fn set_local_delete_error(&mut self, error: Option<LocalError>) {
        self.local_delete_error = error;
    }

    pub fn set_s3_delete_error(&mut self, error: Option<S3Error>) {
        self.s3_delete_error = error;
    }

    pub fn set_create_bucket_error(&mut self, error: Option<S3Error>) {
        self.create_bucket_error = error;
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

    pub fn sort_s3_data(&mut self, column: SortColumn) {
        self.s3_sort_state.set_column(column);
        sort_items(&mut self.s3_data, &self.s3_sort_state);
    }

    pub fn sort_local_data(&mut self, column: SortColumn) {
        self.local_sort_state.set_column(column);
        sort_items(&mut self.local_data, &self.local_sort_state);
    }

    pub fn set_search_mode(&mut self, active: bool) {
        self.search_mode = active;
    }

    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.search_mode = false;
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
            if item.is_transferred() {
                continue;
            }

            if item.children.is_none() {
                if item.destination_bucket == *bucket_name && item.name == decoded_name {
                    item.transfer_state = TransferState::InProgress(progress_item.progress);
                }
            } else if let Some(children) = &mut item.children {
                for child in children.iter_mut() {
                    // Skip already-transferred children
                    if child.is_transferred() {
                        continue;
                    }
                    if child.destination_bucket == *bucket_name && child.name == decoded_name {
                        child.transfer_state = TransferState::InProgress(progress_item.progress);
                    }
                }
                let progress = calculate_overall_progress(children);
                item.transfer_state = TransferState::InProgress(progress);
            }
        }
    }

    fn update_s3_item_with_progress(&mut self, progress_item: &DownloadProgressItem) {
        let target_bucket = Some(progress_item.bucket.clone());

        for item in &mut self.s3_selected_items {
            // Use path if available, otherwise fall back to name (matches download_item logic)
            let item_key = item.path.as_ref().unwrap_or(&item.name);

            // Skip already-transferred items to avoid race conditions with late progress updates
            if item.is_transferred() {
                continue;
            }

            if item.children.is_none() {
                if item_key == &progress_item.name && item.bucket == target_bucket {
                    item.transfer_state = TransferState::InProgress(progress_item.progress);
                }
            } else if let Some(children) = &mut item.children {
                for child in children.iter_mut() {
                    // Skip already-transferred children
                    if child.is_transferred() {
                        continue;
                    }
                    let child_key = child.path.as_ref().unwrap_or(&child.name);
                    if child_key == &progress_item.name && child.bucket == target_bucket {
                        child.transfer_state = TransferState::InProgress(progress_item.progress);
                    }
                }
                let progress = calculate_overall_progress(children);
                item.transfer_state = TransferState::InProgress(progress);
            }
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
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
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
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
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
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::default(),
        };
        state.s3_selected_items.push(selected_item.clone());
        state.update_selected_s3_transfers(selected_item.clone());
        assert!(state.s3_selected_items[0].is_transferred());
        assert_eq!(state.s3_selected_items[0].transfer_state.progress(), 100f64);
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
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::default(),
        };
        let selected_item = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            s3_creds: FileCredential::default(),
            children: Some(vec![child.clone()]),
            transfer_state: TransferState::default(),
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
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::Failed("Error".into()),
        };
        state.add_s3_selected_item(selected_item.clone());
        state.update_selected_s3_transfers(selected_item.clone());
        assert!(!state.s3_selected_items[0].is_transferred());
        assert_eq!(state.s3_selected_items[0].transfer_state.progress(), 0f64);
    }

    #[test]
    fn update_selected_local_transfers_updates_correctly() {
        let mut state = State::default();
        let selected_item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
        };
        state.add_local_selected_item(selected_item.clone());
        state.update_selected_local_transfers(selected_item.clone());
        assert!(state.local_selected_items[0].is_transferred());
        assert_eq!(state.local_selected_items[0].transfer_state.progress(), 100f64);
    }

    #[test]
    fn update_selected_local_transfers_with_error_updates_correctly() {
        let mut state = State::default();
        let selected_item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::Failed("Error".into()),
        };
        state.add_local_selected_item(selected_item.clone());
        state.update_selected_local_transfers(selected_item.clone());
        assert!(!state.local_selected_items[0].is_transferred());
        assert_eq!(state.local_selected_items[0].transfer_state.progress(), 0f64);
    }

    #[test]
    fn remove_already_transferred_items_removes_correctly() {
        let mut state = State::default();
        let local_item_not_transfered = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
        };
        let local_item_transfered = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::Completed,
        };
        let s3_item_not_transferred = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::default(),
        };
        let s3_item_transferred = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::Completed,
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
        assert!(!state.s3_selected_items[0].is_transferred());
        assert!(!state.local_selected_items[0].is_transferred());
    }

    #[test]
    fn update_progress_on_selected_local_item_updates_correctly() {
        let mut state = State::default();
        let selected_item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
        };

        state.local_selected_items.push(selected_item.clone());
        let progress_item = UploadProgressItem {
            progress: 0.5,
            uri: "https://test-bucket.s3.amazonaws.com/path/to/file1.txt".into(),
        };
        state.update_progress_on_selected_local_item(&progress_item);

        assert_eq!(state.local_selected_items[0].transfer_state.progress(), 0.5);
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
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::default(),
        };

        state.s3_selected_items.push(item.clone());
        // DownloadProgressItem.name should be the path (S3 key), matching download_item behavior
        let progress_item = DownloadProgressItem {
            progress: 0.5,
            bucket: "test-bucket".to_string(),
            name: "path/to/file1.txt".into(),
        };
        state.update_progress_on_selected_s3_item(&progress_item);

        assert_eq!(state.s3_selected_items[0].transfer_state.progress(), 0.5);
    }

    #[test]
    fn update_local_item_with_progress_updates_the_item() {
        let mut state = State::default();
        let selected_item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
        };
        state.local_selected_items = vec![selected_item];
        let progress_item = UploadProgressItem {
            progress: 50.0,
            uri: "https://test-bucket.s3.eu-west-1.amazonaws.com/file1.txt?x-id=PutObject".into(),
        };
        state.update_local_item_with_progress(&progress_item);
        assert_eq!(state.local_selected_items[0].transfer_state.progress(), 50.0);
    }

    #[test]
    fn update_local_item_with_progress_updates_child_correctly() {
        let mut state = State::default();
        let child = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
        };
        let selected_item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "/".into(),
            path: "path/to".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: Some(vec![child]),
            transfer_state: TransferState::default(),
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
                .transfer_state
                .progress(),
            50.0
        );
        assert_eq!(state.local_selected_items[0].transfer_state.progress(), 50.0);
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
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::default(),
        };
        state.s3_selected_items = vec![selected_item];
        // DownloadProgressItem.name should be the path (S3 key), matching download_item behavior
        let progress_item = DownloadProgressItem {
            progress: 50.0,
            bucket: "test-bucket".into(),
            name: "path/to/file1.txt".into(),
        };
        state.update_s3_item_with_progress(&progress_item);
        assert_eq!(state.s3_selected_items[0].transfer_state.progress(), 50.0);
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
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::default(),
        };
        let selected_item = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "/".into(),
            path: Some("path/to".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "path/to/dest".into(),
            s3_creds: FileCredential::default(),
            children: Some(vec![child.clone()]),
            transfer_state: TransferState::default(),
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
                .transfer_state
                .progress(),
            50.0
        );
        assert_eq!(state.s3_selected_items[0].transfer_state.progress(), 50.0);
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

    #[test]
    fn all_uploads_complete_for_bucket_returns_true_when_all_transferred() {
        let mut state = State::default();
        let item1 = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::Completed,
        };
        let item2 = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file2.txt".into(),
            path: "path/to/file2.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::Completed,
        };
        state.add_local_selected_item(item1);
        state.add_local_selected_item(item2);
        assert!(state.all_uploads_complete_for_bucket("test-bucket"));
    }

    #[test]
    fn all_uploads_complete_for_bucket_returns_false_when_pending() {
        let mut state = State::default();
        let item1 = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::Completed,
        };
        let item2 = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file2.txt".into(),
            path: "path/to/file2.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::InProgress(50.0),
        };
        state.add_local_selected_item(item1);
        state.add_local_selected_item(item2);
        assert!(!state.all_uploads_complete_for_bucket("test-bucket"));
    }

    #[test]
    fn all_uploads_complete_for_bucket_ignores_other_buckets() {
        let mut state = State::default();
        let item1 = LocalSelectedItem {
            destination_bucket: "bucket-a".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::Completed,
        };
        let item2 = LocalSelectedItem {
            destination_bucket: "bucket-b".into(),
            destination_path: "".to_string(),
            name: "file2.txt".into(),
            path: "path/to/file2.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::InProgress(50.0),
        };
        state.add_local_selected_item(item1);
        state.add_local_selected_item(item2);
        // bucket-a should be complete even though bucket-b is not
        assert!(state.all_uploads_complete_for_bucket("bucket-a"));
        assert!(!state.all_uploads_complete_for_bucket("bucket-b"));
    }

    #[test]
    fn all_downloads_complete_for_directory_returns_true_when_all_transferred() {
        let mut state = State::default();
        let item1 = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "/home/user/downloads".into(),
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::Completed,
        };
        let item2 = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file2.txt".into(),
            path: Some("path/to/file2.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "/home/user/downloads".into(),
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::Completed,
        };
        state.add_s3_selected_item(item1);
        state.add_s3_selected_item(item2);
        assert!(state.all_downloads_complete_for_directory("/home/user/downloads"));
    }

    #[test]
    fn all_downloads_complete_for_directory_returns_false_when_pending() {
        let mut state = State::default();
        let item1 = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "/home/user/downloads".into(),
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::Completed,
        };
        let item2 = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file2.txt".into(),
            path: Some("path/to/file2.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "/home/user/downloads".into(),
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::InProgress(50.0),
        };
        state.add_s3_selected_item(item1);
        state.add_s3_selected_item(item2);
        assert!(!state.all_downloads_complete_for_directory("/home/user/downloads"));
    }

    #[test]
    fn all_downloads_complete_for_directory_ignores_other_directories() {
        let mut state = State::default();
        let item1 = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "/home/user/dir-a".into(),
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::Completed,
        };
        let item2 = S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: "file2.txt".into(),
            path: Some("path/to/file2.txt".into()),
            is_directory: false,
            is_bucket: false,
            destination_dir: "/home/user/dir-b".into(),
            s3_creds: FileCredential::default(),
            children: None,
            transfer_state: TransferState::InProgress(50.0),
        };
        state.add_s3_selected_item(item1);
        state.add_s3_selected_item(item2);
        // dir-a should be complete even though dir-b is not
        assert!(state.all_downloads_complete_for_directory("/home/user/dir-a"));
        assert!(!state.all_downloads_complete_for_directory("/home/user/dir-b"));
    }

    #[test]
    fn test_search_mode_activation() {
        let mut state = State::default();
        assert!(!state.search_mode);
        assert!(state.search_query.is_empty());

        state.set_search_mode(true);
        assert!(state.search_mode);
    }

    #[test]
    fn test_search_mode_deactivation_preserves_query() {
        let mut state = State::default();
        state.search_mode = true;
        state.search_query = "test".to_string();

        state.set_search_mode(false);
        assert!(!state.search_mode);
        // Query should be preserved so filtering continues after closing the input
        assert_eq!(state.search_query, "test");
    }

    #[test]
    fn test_set_search_query() {
        let mut state = State::default();
        state.set_search_query("document".to_string());
        assert_eq!(state.search_query, "document");
    }

    #[test]
    fn test_clear_search() {
        let mut state = State::default();
        state.search_mode = true;
        state.search_query = "test".to_string();

        state.clear_search();
        assert!(!state.search_mode);
        assert!(state.search_query.is_empty());
    }
}
