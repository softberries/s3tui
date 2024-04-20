use crate::model::local_data_item::LocalDataItem;
use crate::model::s3_data_item::S3DataItem;

#[derive(Debug, Clone)]
pub enum ActivePage {
    FileManagerPage,
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
}

impl State {
    pub fn update_buckets(&mut self, bucket_list: Vec<S3DataItem>) {
        self.s3_data = bucket_list;
    }

    pub fn update_files(&mut self, files: Vec<LocalDataItem>) {
        self.local_data = files;
    }
}