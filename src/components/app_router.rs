use crossterm::event::KeyEvent;
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;
use crate::model::action::Action;
use crate::components::file_manager_page::FileManagerPage;
use crate::components::component::{Component, ComponentRender};
use crate::components::help_page::HelpPage;
use crate::components::s3_creds_page::S3CredsPage;
use crate::components::transfers_page::TransfersPage;
use crate::model::state::State;
use crate::model::state::ActivePage;


struct Props {
    active_page: ActivePage,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        Props {
            active_page: state.clone().active_page
        }
    }
}

/// Handles transitions between different TUI pages and passes on the state transitions
pub struct AppRouter {
    props: Props,
    file_manager_page: FileManagerPage,
    help_page: HelpPage,
    transfers_page: TransfersPage,
    s3_creds_page: S3CredsPage
}

impl AppRouter {
    fn get_active_page_component(&self) -> &dyn Component {
        match self.props.active_page {
            ActivePage::FileManager => &self.file_manager_page,
            ActivePage::Help => &self.help_page,
            ActivePage::Transfers => &self.transfers_page,
            ActivePage::S3Creds => &self.s3_creds_page,
        }
    }

    fn get_active_page_component_mut(&mut self) -> &mut dyn Component {
        match self.props.active_page {
            ActivePage::FileManager => &mut self.file_manager_page,
            ActivePage::Help => &mut self.help_page,
            ActivePage::Transfers => &mut self.transfers_page,
            ActivePage::S3Creds => &mut self.s3_creds_page,
        }
    }
}

impl Component for AppRouter {
    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
        where
            Self: Sized,
    {
        AppRouter {
            props: Props::from(state),
            //
            file_manager_page: FileManagerPage::new(state, action_tx.clone()),
            help_page: HelpPage::new(state, action_tx.clone()),
            transfers_page: TransfersPage::new(state, action_tx.clone()),
            s3_creds_page: S3CredsPage::new(state, action_tx.clone()),
        }
            .move_with_state(state)
    }

    fn move_with_state(self, state: &State) -> Self
        where
            Self: Sized,
    {
        AppRouter {
            props: Props::from(state),
            //
            file_manager_page: self.file_manager_page.move_with_state(state),
            help_page: self.help_page.move_with_state(state),
            transfers_page: self.transfers_page.move_with_state(state),
            s3_creds_page: self.s3_creds_page.move_with_state(state),
        }
    }

    // route all functions to the active page
    fn name(&self) -> &str {
        self.get_active_page_component().name()
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        self.get_active_page_component_mut().handle_key_event(key)
    }
}

impl ComponentRender<()> for AppRouter {
    fn render(&self, frame: &mut Frame, props: ()) {
        match self.props.active_page {
            ActivePage::FileManager => self.file_manager_page.render(frame, props),
            ActivePage::Help => self.help_page.render(frame, props),
            ActivePage::Transfers => self.transfers_page.render(frame, props),
            ActivePage::S3Creds => self.s3_creds_page.render(frame, props),
        }
    }
}
