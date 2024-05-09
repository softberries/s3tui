use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use crate::model::action::Action;
use crate::components::component::{Component, ComponentRender};
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::state::{ActivePage, State};

#[derive(Clone)]
struct Props {
    s3_table_state: TableState,
    local_table_state: TableState,
    s3_selected_items: Vec<S3SelectedItem>,
    local_selected_items: Vec<LocalSelectedItem>,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        let st = state.clone();
        Props {
            s3_table_state: TableState::default(),
            local_table_state: TableState::default(),
            s3_selected_items: st.s3_selected_items,
            local_selected_items: st.local_selected_items,
        }
    }
}

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
                s3_table_state: self.props.s3_table_state,
                local_table_state: self.props.local_table_state,
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
    fn get_s3_row(&self, item: &S3SelectedItem) -> Row {
        if item.error.is_some() {
            Row::new(item.to_columns().clone()).fg(Color::Red)
        } else if item.transferred {
            Row::new(item.to_columns().clone()).fg(Color::Blue)
        } else {
            Row::new(item.to_columns().clone())
        }
    }
    fn get_local_row(&self, item: &LocalSelectedItem) -> Row {
        if item.transferred {
            Row::new(item.to_columns().clone()).fg(Color::Blue)
        } else {
            Row::new(item.to_columns().clone())
        }
    }

    fn get_s3_table(&self) -> Table {
        let focus_color = Color::Rgb(98, 114, 164);
        let header =
            Row::new(vec!["Bucket Name", "Resource Path", "Destination", "S3 Account", "Progress", "Error?"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.s3_selected_items.iter().map(|item| TransfersPage::get_s3_row(self, item));
        let widths = [Constraint::Length(20), Constraint::Length(20), Constraint::Length(10), Constraint::Length(10), Constraint::Length(20), Constraint::Length(5), Constraint::Length(5), Constraint::Length(10)];
        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Transfers List (S3 -> Local)").fg(Color::White))   
            .highlight_style(Style::default().fg(focus_color).bg(Color::White).add_modifier(Modifier::REVERSED))
            .widths([Constraint::Percentage(20), Constraint::Percentage(20), Constraint::Percentage(10), Constraint::Percentage(10), Constraint::Percentage(20), Constraint::Percentage(5), Constraint::Percentage(5), Constraint::Percentage(10)]);
        table
    }

    fn get_local_table(&self) -> Table {
        let focus_color = Color::Rgb(98, 114, 164);
        let header =
            Row::new(vec!["File Name", "Path", "Destination Bucket", "Destination Path", "S3 Account", "Progress"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.local_selected_items.iter().map(|item| TransfersPage::get_local_row(self, item));
        let widths = [Constraint::Length(20), Constraint::Length(20), Constraint::Length(20), Constraint::Length(10), Constraint::Length(10), Constraint::Length(10), Constraint::Length(10)];
        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Transfers List (Local -> S3)").fg(Color::White))
            .highlight_style(Style::default().fg(focus_color).bg(Color::White).add_modifier(Modifier::REVERSED))
            .widths([Constraint::Percentage(20), Constraint::Percentage(20), Constraint::Percentage(20), Constraint::Percentage(10), Constraint::Percentage(10), Constraint::Percentage(10), Constraint::Percentage(10)]);
        table
    }
}

impl ComponentRender<()> for TransfersPage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let s3_table = self.get_s3_table();
        let local_table = self.get_local_table();
        let size = frame.size();
        match (self.props.local_selected_items.is_empty(), self.props.s3_selected_items.is_empty()) {
            (false, false) => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(size);
                frame.render_stateful_widget(&s3_table, chunks[0], &mut self.props.clone().s3_table_state);
                frame.render_stateful_widget(&local_table, chunks[1], &mut self.props.clone().local_table_state);
            }
            (false, true) => frame.render_stateful_widget(&local_table, frame.size(), &mut self.props.clone().local_table_state),
            (true, false) => frame.render_stateful_widget(&s3_table, frame.size(), &mut self.props.clone().s3_table_state),
            (true, true) => {
                // Define horizontal constraints: previous, center (50% of available space), next
                let chunks_horizontal = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(25), // Adjust this percentage to better center the text
                        Constraint::Percentage(50),
                        Constraint::Percentage(25),
                    ])
                    .split(size);

                // Define vertical constraints: top, middle (50% of available height), bottom
                let chunks_vertical = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(25), // Adjust this percentage to better center the text
                        Constraint::Percentage(50),
                        Constraint::Percentage(25),
                    ])
                    .split(chunks_horizontal[1]); // Apply vertical layout to the center horizontal chunk

                let text = Text::from("No transfers created. Use arrows (↔) to select/deselect items for transfer");
                let info = Paragraph::new(text)
                    .alignment(Alignment::Center) // Center text horizontally
                    .block(Block::default().borders(Borders::NONE)); // Optional: Add borders to see the widget's extents

                frame.render_widget(&info, chunks_vertical[1]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use tokio::sync::mpsc;

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
            state: KeyEventState::NONE
        });
        assert_eq!(rx.recv().await.unwrap(), Action::RunTransfers, "Should send RunTransfers action");

        // Test 'q' key for exit action
        page.handle_key_event(KeyEvent {
            code: KeyCode::Char('q'),
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE
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
    }

    #[tokio::test]
    async fn test_initialization() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = State::default();
        let page = TransfersPage::new(&state, tx);

        // Assuming Props::from(&state) initializes TableStates as default and copies selected items lists
        assert!(page.props.s3_table_state.selected().is_none(), "S3 table state should be initialized to default");
        assert!(page.props.local_table_state.selected().is_none(), "Local table state should be initialized to default");
        assert_eq!(page.props.s3_selected_items, state.s3_selected_items, "S3 selected items should match state");
        assert_eq!(page.props.local_selected_items, state.local_selected_items, "Local selected items should match state");
    }
}
