use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::Frame;
use ratatui::prelude::Text;
use ratatui::widgets::Paragraph;
use tokio::sync::mpsc::UnboundedSender;
use crate::action::Action;
use crate::components::component::{Component, ComponentRender};
use crate::state::{ActivePage, State};

struct Props {
    /// The logged in user
    i: String,
}

impl From<&State> for Props {
    fn from(_state: &State) -> Self {
        Props {
            i: "".to_string(),
        }
    }
}

pub struct FileManagerPage {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,
    /// State Mapped ChatPage Props
    props: Props,
}

impl Component for FileManagerPage {
    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
        where
            Self: Sized,
    {
        FileManagerPage {
            action_tx: action_tx.clone(),
            // set the props
            props: Props::from(state),
        }
            .move_with_state(state)
    }

    fn move_with_state(self, state: &State) -> Self
        where
            Self: Sized,
    {
        FileManagerPage {
            props: Props::from(state),
            ..self
        }
    }

    fn name(&self) -> &str {
        "File Manager"
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Char('?') => {
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::HelpPage });
            }
            KeyCode::Char('q') => {
                let _ = self.action_tx.send(Action::Exit);
            }
            _ => {}
        }
    }
}

impl ComponentRender<()> for FileManagerPage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let user_info = Paragraph::new(Text::from(format!("File Manger: @{}", self.props.i)));
        frame.render_widget(user_info, frame.size());
    }
}