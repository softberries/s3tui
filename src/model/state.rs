use crate::model::local_data_item::LocalDataItem;
use crate::model::s3_data_item::S3DataItem;
use crate::model::s3_selected_item::S3SelectedItem;

#[derive(Debug, Clone)]
pub enum ActivePage {
    FileManagerPage,
    TransfersPage,
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
    pub current_local_path: String,
    pub current_s3_bucket: String,
    pub current_s3_path: String
}

impl State {
    pub fn update_buckets(&mut self, bucket_list: Vec<S3DataItem>) {
        self.s3_data = bucket_list;
        self.s3_loading = false;
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
}