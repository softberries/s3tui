use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use crate::model::action::Action;
use crate::components::component::{Component, ComponentRender};
use crate::model::state::{ActivePage, State};

struct Props {
    commands: Vec<Vec<String>>,  
}

impl From<&State> for Props {
    fn from(_state: &State) -> Self {
        Props {
            commands: vec![
                vec!["s".to_string(), "move back to the file manager window".to_string()],
                vec!["Esc".to_string(), "select/deselect files to transfer".to_string()],
                vec!["↔ / j / k".to_string(), "move up/down on the lists".to_string()],
                vec!["t".to_string(), "show currently selected files to transfer".to_string()],
                vec!["r".to_string(), "run currently selected transfers".to_string()],
                vec!["q".to_string(), "quit the application".to_string()],
                vec!["?".to_string(), "this help page".to_string()]
            ],
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
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::FileManager });
            }
            _ => {}
        }
    }
}

impl ComponentRender<()> for HelpPage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let v_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(3),
                Constraint::Percentage(94),
                Constraint::Percentage(3),
            ])
            .split(frame.size());
        let h_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(3),
                Constraint::Percentage(94),
                Constraint::Percentage(3),
            ])
            .split(v_layout[1]);
        let rows: Vec<Row> = self.props.commands.iter().map(|c| Row::new(c.clone())).collect();
        let header =
            Row::new(vec!["Command Name", "Description"]).bold().underlined().height(1).bottom_margin(0);
        let table = Table::new(rows, [Constraint::Length(30), Constraint::Length(70)])
            .block(Block::new().borders(Borders::ALL))
            .header(header);
        frame.render_widget(table, h_layout[1]);
    }
}