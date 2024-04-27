use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::Frame;
use ratatui::prelude::Text;
use ratatui::widgets::Paragraph;
use tokio::sync::mpsc::UnboundedSender;
use crate::model::action::Action;
use crate::components::component::{Component, ComponentRender};
use crate::model::state::{ActivePage, State};

struct Props {
    i: String,
}

impl From<&State> for Props {
    fn from(_state: &State) -> Self {
        Props {
            i: "".to_string(),
        }
    }
}

pub struct HelpPage {
    pub action_tx: UnboundedSender<Action>,
    props: Props,
}

impl Component for HelpPage {
    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
        where
            Self: Sized,
    {
        HelpPage {
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
        HelpPage {
            props: Props::from(state),
            ..self
        }
    }

    fn name(&self) -> &str {
        "Help Page"
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Char('q') => {
                let _ = self.action_tx.send(Action::Exit);
            }
            KeyCode::Esc => {
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::FileManagerPage });
            }
            _ => {}
        }
    }
}

impl ComponentRender<()> for HelpPage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let user_info = Paragraph::new(Text::from(format!("Help Page: @{}", self.props.i)));
        frame.render_widget(user_info, frame.size());
    }
}