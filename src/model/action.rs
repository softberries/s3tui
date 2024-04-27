use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::state::ActivePage;

#[derive(Debug, Clone)]
pub enum Action {
    Navigate { page: ActivePage },
    FetchLocalData { path: String },
    FetchS3Data { bucket: Option<String>, prefix: Option<String> },
    MoveBackLocal,
    SelectS3Item { item: S3SelectedItem },
    UnselectS3Item { item: S3SelectedItem },
    Exit,
}