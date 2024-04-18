use crossterm::event::KeyEvent;
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;
use crate::action::Action;
use crate::file_manager_page::FileManagerPage;
use crate::component::{Component, ComponentRender};
use crate::help_page::HelpPage;
use crate::state::State;
use crate::state::ActivePage;


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

pub struct AppRouter {
    props: Props,
    file_manager_page: FileManagerPage,
    help_page: HelpPage,
}

impl AppRouter {
    fn get_active_page_component(&self) -> &dyn Component {
        match self.props.active_page {
            ActivePage::FileManagerPage => &self.file_manager_page,
            ActivePage::HelpPage => &self.help_page,
        }
    }

    fn get_active_page_component_mut(&mut self) -> &mut dyn Component {
        match self.props.active_page {
            ActivePage::FileManagerPage => &mut self.file_manager_page,
            ActivePage::HelpPage => &mut self.help_page,
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
            ActivePage::FileManagerPage => self.file_manager_page.render(frame, props),
            ActivePage::HelpPage => self.help_page.render(frame, props),
        }
    }
}
