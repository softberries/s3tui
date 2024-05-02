use crate::model::local_data_item::LocalDataItem;
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_data_item::S3DataItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::settings::file_credentials::FileCredential;

#[derive(Debug, Clone, PartialEq)]
pub enum ActivePage {
    FileManagerPage,
    TransfersPage,
    S3CredsPage,
    HelpPage,
}

impl Default for ActivePage {
    fn default() -> Self {
        ActivePage::FileManagerPage
    }
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
        //todo: update state of selected item instead of removing it
        self.s3_selected_items.retain(|it|
            it.bucket != item.bucket ||
                it.name != item.name ||
                it.path != item.path
        );
    }

    pub fn update_selected_local_transfers(&mut self, item: LocalSelectedItem) {
        //todo: update state of selected item instead of removing it
        self.local_selected_items.retain(|it|
            it.name != item.name ||
                it.path != item.path
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
}