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
    pub active_page: ActivePage
}