//! This module provides list of all possible actions which can be executed on the UI
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::sorting::SortColumn;
use crate::model::state::ActivePage;
use crate::settings::file_credentials::FileCredential;

/// List of all possible actions a user can execute
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Navigate {
        page: ActivePage,
    },
    FetchLocalData {
        path: String,
    },
    FetchS3Data {
        bucket: Option<String>,
        prefix: Option<String>,
    },
    ListS3DataRecursiveForItem {
        item: S3SelectedItem,
    },
    MoveBackLocal,
    SelectS3Item {
        item: S3SelectedItem,
    },
    UnselectS3Item {
        item: S3SelectedItem,
    },
    SelectLocalItem {
        item: LocalSelectedItem,
    },
    UnselectLocalItem {
        item: LocalSelectedItem,
    },
    SelectCurrentS3Creds {
        item: FileCredential,
    },
    DeleteS3Item {
        item: S3SelectedItem,
    },
    DeleteLocalItem {
        item: LocalSelectedItem,
    },
    CreateBucket {
        name: String,
    },
    ClearDeletionErrors,
    RunTransfers,
    SortS3 {
        column: SortColumn,
    },
    SortLocal {
        column: SortColumn,
    },
    SetSearchMode {
        active: bool,
    },
    SetSearchQuery {
        query: String,
    },
    ClearSearch,
    Exit,
}
