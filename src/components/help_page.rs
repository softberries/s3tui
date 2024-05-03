use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use crate::model::action::Action;
use crate::components::component::{Component, ComponentRender};
use crate::model::state::{ActivePage, State};

struct Props {
    commands: Vec<(String, String)>,  // command and its description
    list_state: ListState,
}

impl From<&State> for Props {
    fn from(_state: &State) -> Self {
        Props {
            commands: vec![
                ("s".to_string(), "select current s3 account".to_string()),
                ("Esc".to_string(), "move back to the file manager window".to_string()),
                ("↔".to_string(), "select/deselect files to transfer".to_string()),
                ("↕ / j / k".to_string(), "move up/down on the lists".to_string()),
                ("t".to_string(), "show currently selected files to transfer".to_string()),
                ("r".to_string(), "run currently selected transfers".to_string()),
                ("q".to_string(), "quit the application".to_string()),
                ("?".to_string(), "this help page".to_string()),
            ],
            list_state: ListState::default(),
        }
    }
}

pub struct HelpPage {
    pub action_tx: UnboundedSender<Action>,
    props: Props,
}

impl HelpPage {
    pub fn navigate(&mut self, up: bool) {
        let i = match self.props.list_state.selected() {
            Some(i) => {
                if up {
                    i.saturating_sub(1)
                } else {
                    i.saturating_add(1).min(self.props.commands.len().saturating_sub(1))
                }
            }
            None => 0,
        };
        self.props.list_state.select(Some(i));
    }
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
            KeyCode::Char('j') | KeyCode::Down => self.navigate(false),
            KeyCode::Char('k') | KeyCode::Up => self.navigate(true),
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
        let size = frame.size();

        // Create a list of ListItem from commands
        let items: Vec<ListItem> = self.props.commands.iter().map(|(cmd, desc)| {
            let text = vec![
                Line::from(vec![
                    Span::raw(cmd),
                    Span::raw("  -  "),
                    Span::styled(desc, Style::new().green().italic()),
                    ".".into(),
                ]),
            ];
            ListItem::new(text)
        }).collect();

        // Create a List widget
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Commands"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        // Render the list widget
        frame.render_stateful_widget(list, size, &mut self.props.list_state.clone());
    }
}