use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use crate::model::action::Action;
use crate::components::component::{Component, ComponentRender};
use crate::model::state::{ActivePage, State};
use crate::settings::file_credentials::FileCredential;

#[derive(Clone)]
struct Props {
    creds_table_state: TableState,
    creds_data: Vec<FileCredential>,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        let st = state.clone();
        Props {
            creds_table_state: TableState::default(),
            creds_data: st.creds,
        }
    }
}

pub struct S3CredsPage {
    pub action_tx: UnboundedSender<Action>,
    props: Props,
}

impl Component for S3CredsPage {
    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
        where
            Self: Sized,
    {
        S3CredsPage {
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
        let new_props = Props::from(state);
        S3CredsPage {
            props: Props {
                creds_table_state: self.props.creds_table_state.clone(),
                ..new_props
            },
            ..self
        }
    }

    fn name(&self) -> &str {
        "S3CredsPage"
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_down_creds_table_selection()
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_up_creds_table_selection()
            }
            KeyCode::Enter => {
                self.set_current_s3_account()
            }
            KeyCode::Char('q') => {
                let _ = self.action_tx.send(Action::Exit);
            }
            KeyCode::Char('?') => {
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::Help });
            }
            KeyCode::Esc => {
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::FileManager });
            }
            _ => {}
        }
    }
}

impl S3CredsPage {
    fn get_s3_row(&self, item: &FileCredential) -> Row {
        if item.selected {
            Row::new(vec![format!("{} (*)", item.name)])
        } else {
            Row::new(vec![format!("{}", item.name)])
        }
    }

    fn get_s3_table(&self) -> Table {
        let focus_color = Color::Rgb(98, 114, 164);
        let header =
            Row::new(vec!["Account Name"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.creds_data.iter().map(|item| S3CredsPage::get_s3_row(self, item));
        let widths = [Constraint::Length(10), Constraint::Length(35), Constraint::Length(35), Constraint::Length(10), Constraint::Length(10)];
        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Account list").fg(Color::White))
            .highlight_style(Style::default().fg(focus_color).bg(Color::White).add_modifier(Modifier::REVERSED))
            .widths([Constraint::Percentage(10), Constraint::Percentage(35), Constraint::Percentage(35), Constraint::Percentage(10), Constraint::Percentage(10)]);
        table
    }

    pub fn move_up_creds_table_selection(&mut self) {
        let i = match self.props.creds_table_state.selected() {
            Some(i) => {
                if i == 0_usize {
                    self.props.creds_data.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.props.creds_table_state.select(Some(i));
    }

    pub fn move_down_creds_table_selection(&mut self) {
        let i = match self.props.creds_table_state.selected() {
            Some(i) => {
                if i >= self.props.creds_data.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.props.creds_table_state.select(Some(i));
    }

    pub fn set_current_s3_account(&mut self) {
        if let Some(selected_row) =
            self.props.creds_table_state.selected().and_then(|index| self.props.creds_data.get(index))
        {
            let sr = selected_row.clone();
            let _ = self.action_tx.send(Action::SelectCurrentS3Creds {
                item: sr.clone()
            });
        }
    }
}

impl ComponentRender<()> for S3CredsPage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let s3_table = self.get_s3_table();
        frame.render_stateful_widget(&s3_table, frame.size(), &mut self.props.clone().creds_table_state)
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;
    use super::*;
    use tokio::sync::mpsc::unbounded_channel;

    #[tokio::test]
    async fn test_component_initialization() {
        let (tx, _rx) = unbounded_channel::<Action>();
        let creds = FileCredential {
            name: "test".to_string(),
            access_key: "accessKey".to_string(),
            secret_key: "secretKey".to_string(),
            default_region: "eu-north-1".to_string(),
            selected: true,
        };
        let state = State::new(vec![creds]);

        let component = S3CredsPage::new(&state, tx);
        assert_eq!(component.name(), "S3CredsPage");
    }

    #[tokio::test]
    async fn test_key_event_handling() {
        let (tx, mut rx) = unbounded_channel::<Action>();
        let creds = FileCredential {
            name: "test".to_string(),
            access_key: "accessKey".to_string(),
            secret_key: "secretKey".to_string(),
            default_region: "eu-north-1".to_string(),
            selected: true,
        };
        let state = State::new(vec![creds.clone()]);
        let mut component = S3CredsPage::new(&state, tx);

        // Simulate pressing 'q'
        component.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()));
        assert_eq!(rx.recv().await.unwrap(), Action::Exit);

        // Simulate pressing '?'
        component.handle_key_event(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::empty()));
        assert_eq!(rx.recv().await.unwrap(), Action::Navigate { page: ActivePage::Help });

        // Simulate pressing 'Esc'
        component.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert_eq!(rx.recv().await.unwrap(), Action::Navigate { page: ActivePage::FileManager });
    }

    #[test]
    fn get_s3_row_should_format_selected_row() {
        let (tx, _rx) = unbounded_channel::<Action>();
        let creds = FileCredential {
            name: "test".to_string(),
            access_key: "accessKey".to_string(),
            secret_key: "secretKey".to_string(),
            default_region: "eu-north-1".to_string(),
            selected: true,
        };
        let state = State::new(vec![creds.clone()]);
        let component = S3CredsPage::new(&state, tx);
        let res = component.get_s3_row(&creds);

        assert_eq!(res, Row::new(vec![format!("{} (*)", creds.name)]))
    }

    #[test]
    fn get_s3_row_should_format_not_selected_row() {
        let (tx, _rx) = unbounded_channel::<Action>();
        let creds = FileCredential {
            name: "test".to_string(),
            access_key: "accessKey".to_string(),
            secret_key: "secretKey".to_string(),
            default_region: "eu-north-1".to_string(),
            selected: false,
        };
        let state = State::new(vec![creds.clone()]);
        let component = S3CredsPage::new(&state, tx);
        let res = component.get_s3_row(&creds);

        assert_eq!(res, Row::new(vec![format!("{}", creds.name)]))
    }
}