use crate::model::state::ActivePage;

#[derive(Debug, Clone)]
pub enum Action {
    Navigate { page: ActivePage },
    FetchLocalData { path: String },
    FetchS3Data { bucket: Option<String>, prefix: Option<String> },
    MoveBackLocal,
    Exit,
}