use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use crate::model::action::Action;
use crate::components::component::{Component, ComponentRender};
use crate::model::state::{ActivePage, State};
use crate::model::transfer_item::TransferItem;

#[derive(Clone)]
struct Props {
    table_state: TableState,
    selected_items: Vec<TransferItem>,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        let st = state.clone();
        let s3_items: Vec<TransferItem> = st.s3_selected_items.iter().map(|i| TransferItem::from_s3_selected_item(i.clone())).collect();
        let local_items: Vec<TransferItem> = st.local_selected_items.iter().map(|i| TransferItem::from_local_selected_item(i.clone())).collect();

        Props {
            table_state: TableState::default(),
            selected_items: {
                let mut all_vec = s3_items.clone();
                all_vec.extend(local_items);
                all_vec
            },
        }
    }
}

/// Page displaying selected transfers and their status
pub struct TransfersPage {
    pub action_tx: UnboundedSender<Action>,
    props: Props,
}

impl Component for TransfersPage {
    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
        where
            Self: Sized,
    {
        TransfersPage {
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
        TransfersPage {
            props: Props {
                table_state: self.props.table_state,
                ..new_props
            },
            ..self
        }
    }

    fn name(&self) -> &str {
        "Transfers"
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Char('r') => {
                let _ = self.action_tx.send(Action::RunTransfers);
            }
            KeyCode::Char('s') => {
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::S3Creds });
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

impl TransfersPage {
    fn get_row(&self, item: &TransferItem) -> Row {
        if item.error.is_some() {
            Row::new(item.to_columns().clone()).fg(Color::Red)
        } else if item.transferred {
            Row::new(item.to_columns().clone()).fg(Color::Blue)
        } else {
            Row::new(item.to_columns().clone())
        }
    }

    fn get_transfers_table(&self) -> Table {
        let focus_color = Color::Rgb(98, 114, 164);
        let header =
            Row::new(vec!["Up/Down", "Bucket", "Path", "Destination", "S3 Account", "Progress", "Error?"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.selected_items.iter().map(|item| TransfersPage::get_row(self, item));
        let widths = [Constraint::Length(5), Constraint::Length(15), Constraint::Length(20), Constraint::Length(20), Constraint::Length(10), Constraint::Length(10), Constraint::Length(10)];
        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Transfers List (S3 -> Local)"))
            .highlight_style(Style::default().fg(focus_color).add_modifier(Modifier::REVERSED))
            .widths([Constraint::Percentage(5), Constraint::Percentage(15), Constraint::Percentage(20), Constraint::Percentage(20), Constraint::Percentage(10), Constraint::Percentage(10), Constraint::Percentage(10)]);
        table
    }
}

impl ComponentRender<()> for TransfersPage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let table = self.get_transfers_table();
        frame.render_stateful_widget(&table, frame.size(), &mut self.props.clone().table_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use tokio::sync::mpsc;
    use crate::model::local_selected_item::LocalSelectedItem;
    use crate::model::s3_selected_item::S3SelectedItem;

    #[tokio::test]
    async fn test_key_event_handling() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let state = State::default();  // Assume State::default() properly initializes the state
        let mut page = TransfersPage::new(&state, tx);

        // Test 'r' key for triggering run transfers
        page.handle_key_event(KeyEvent {
            code: KeyCode::Char('r'),
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert_eq!(rx.recv().await.unwrap(), Action::RunTransfers, "Should send RunTransfers action");

        // Test 'q' key for exit action
        page.handle_key_event(KeyEvent {
            code: KeyCode::Char('q'),
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert_eq!(rx.recv().await.unwrap(), Action::Exit, "Should send Exit action");

        // Test 's' key for navigation to S3Creds page
        page.handle_key_event(KeyEvent {
            code: KeyCode::Char('s'),
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert_eq!(rx.recv().await.unwrap(), Action::Navigate { page: ActivePage::S3Creds }, "Should navigate to S3Creds");

        // Test '?' key for navigation to Help page
        page.handle_key_event(KeyEvent {
            code: KeyCode::Char('?'),
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert_eq!(rx.recv().await.unwrap(), Action::Navigate { page: ActivePage::Help }, "Should navigate to Help Page");

        // Test '?' key for navigation back to FileManager page
        page.handle_key_event(KeyEvent {
            code: KeyCode::Esc,
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert_eq!(rx.recv().await.unwrap(), Action::Navigate { page: ActivePage::FileManager }, "Should navigate to FileManager Page");
    }

    #[tokio::test]
    async fn test_initialization() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = State::default();
        let page = TransfersPage::new(&state, tx);

        // Assuming Props::from(&state) initializes TableStates as default and copies selected items lists
        assert!(page.props.table_state.selected().is_none(), "table state should be initialized to default");
        assert_eq!(page.props.selected_items.len(), state.s3_selected_items.len(), "selected items should match state");
    }

    #[test]
    fn get_s3_row_no_modifiers_constructs_plain_row() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = State::default();
        let page = TransfersPage::new(&state, tx);
        let item = S3SelectedItem {
            bucket: Some("test-bucket".into()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: true,
            destination_dir: "".to_string(),
            transferred: false,
            s3_creds: Default::default(),
            progress: 0f64,
            error: None,
        };
        let transfer_item = TransferItem::from_s3_selected_item(item);
        let res = page.get_row(&transfer_item);
        assert_eq!(res, Row::new(transfer_item.to_columns().clone()));
    }

    #[test]
    fn get_s3_row_with_error_constructs_red_row() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = State::default();
        let page = TransfersPage::new(&state, tx);
        let item = S3SelectedItem {
            bucket: Some("test-bucket".into()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: true,
            destination_dir: "".to_string(),
            transferred: false,
            s3_creds: Default::default(),
            progress: 0f64,
            error: Some("Error".into()),
        };
        let transfer_item = TransferItem::from_s3_selected_item(item);
        let res = page.get_row(&transfer_item);
        assert_eq!(res, Row::new(transfer_item.to_columns().clone()).fg(Color::Red));
    }

    #[test]
    fn get_s3_row_with_transferred_constructs_blue_row() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = State::default();
        let page = TransfersPage::new(&state, tx);
        let item = S3SelectedItem {
            bucket: Some("test-bucket".into()),
            name: "file1.txt".into(),
            path: Some("path/to/file1.txt".into()),
            is_directory: false,
            is_bucket: true,
            destination_dir: "".to_string(),
            transferred: true,
            s3_creds: Default::default(),
            progress: 0f64,
            error: None,
        };
        let transfer_item = TransferItem::from_s3_selected_item(item);
        let res = page.get_row(&transfer_item);
        assert_eq!(res, Row::new(transfer_item.to_columns().clone()).fg(Color::Blue));
    }

    #[test]
    fn get_local_row_without_modifiers_constructs_plain_row() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = State::default();
        let page = TransfersPage::new(&state, tx);
        let item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            transferred: false,
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            progress: 0.0,
            is_directory: false,
            s3_creds: Default::default(),
            error: None,
        };
        let transfer_item = TransferItem::from_local_selected_item(item);
        let res = page.get_row(&transfer_item);
        assert_eq!(res, Row::new(transfer_item.to_columns().clone()));
    }

    #[test]
    fn get_local_row_with_transferred_constructs_blue_row() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = State::default();
        let page = TransfersPage::new(&state, tx);
        let item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            transferred: true,
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            progress: 0.0,
            is_directory: false,
            s3_creds: Default::default(),
            error: None,
        };
        let transfer_item = TransferItem::from_local_selected_item(item);
        let res = page.get_row(&transfer_item);
        assert_eq!(res, Row::new(transfer_item.to_columns().clone()).fg(Color::Blue));
    }

    #[test]
    fn get_local_row_with_error_constructs_red_row() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = State::default();
        let page = TransfersPage::new(&state, tx);
        let item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            transferred: false,
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            progress: 0.0,
            is_directory: false,
            s3_creds: Default::default(),
            error: Some("Error".into()),
        };
        let transfer_item = TransferItem::from_local_selected_item(item);
        let res = page.get_row(&transfer_item);
        assert_eq!(res, Row::new(transfer_item.to_columns().clone()).fg(Color::Red));
    }
}
