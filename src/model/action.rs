use crate::model::state::ActivePage;

#[derive(Debug, Clone)]
pub enum Action {
    Navigate { page: ActivePage },
    Exit,
}