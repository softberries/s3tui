use crate::components::component::{Component, ComponentRender};
use crate::components::widgets::QuitConfirmation;
use crate::model::action::Action;
use crate::model::has_children::flatten_items;
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::state::{ActivePage, State};
use crate::model::transfer_item::TransferItem;
use crate::utils::format_progress_bar;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
struct Props {
    table_state: TableState,
    selected_items: Vec<TransferItem>,
    s3_selected_items: Vec<S3SelectedItem>,
    local_selected_items: Vec<LocalSelectedItem>,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        let st = state.clone();
        let s3_items: Vec<TransferItem> = st
            .s3_selected_items
            .iter()
            .map(|i| TransferItem::from_s3_selected_item(i.clone()))
            .collect();
        let local_items: Vec<TransferItem> = st
            .local_selected_items
            .iter()
            .map(|i| TransferItem::from_local_selected_item(i.clone()))
            .collect();

        Props {
            table_state: TableState::default(),
            s3_selected_items: st.s3_selected_items,
            local_selected_items: st.local_selected_items,
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
    show_shortcuts_overlay: bool,
    show_quit_confirmation: bool,
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
            show_shortcuts_overlay: false,
            show_quit_confirmation: false,
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
            show_shortcuts_overlay: self.show_shortcuts_overlay,
            show_quit_confirmation: self.show_quit_confirmation,
            ..self
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Handle quit confirmation
        if self.show_quit_confirmation {
            if let Some(confirmed) = QuitConfirmation::handle_key_event(key) {
                if confirmed {
                    let _ = self.action_tx.send(Action::Exit);
                } else {
                    self.show_quit_confirmation = false;
                }
            }
            return;
        }

        // Handle shortcuts overlay
        if self.show_shortcuts_overlay {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') => {
                    self.show_shortcuts_overlay = false;
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_down_table_selection();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_up_table_selection();
            }
            KeyCode::Delete | KeyCode::Backspace => {
                self.unselect_transfer_item();
            }
            KeyCode::Char('c') => {
                self.cancel_selected_transfer();
            }
            KeyCode::Char('p') => {
                self.pause_selected_transfer();
            }
            KeyCode::Char('u') => {
                self.resume_selected_transfer();
            }
            KeyCode::Char('r') => {
                let _ = self.action_tx.send(Action::RunTransfers);
            }
            KeyCode::Char('s') => {
                let _ = self.action_tx.send(Action::Navigate {
                    page: ActivePage::S3Creds,
                });
            }
            KeyCode::Char('q') => {
                self.show_quit_confirmation = true;
            }
            KeyCode::Char('?') => {
                self.show_shortcuts_overlay = true;
            }
            KeyCode::Esc => {
                let _ = self.action_tx.send(Action::Navigate {
                    page: ActivePage::FileManager,
                });
            }
            _ => {}
        }
    }
}

impl TransfersPage {
    fn make_shortcuts_overlay(&self) -> Table<'_> {
        let shortcuts = vec![
            ("↑↓ / j k", "Navigate up/down"),
            ("r", "Run/start transfers"),
            ("p", "Pause selected transfer"),
            ("u", "Resume paused transfer"),
            ("c", "Cancel selected transfer"),
            ("Del / ⌫", "Remove from list"),
            ("s", "Select S3 account"),
            ("Esc", "Back to file manager"),
            ("q", "Quit application"),
            ("?", "Toggle this help"),
        ];

        let rows: Vec<Row> = shortcuts
            .iter()
            .map(|(key, desc)| {
                Row::new(vec![
                    Span::styled(*key, Style::default().fg(Color::Yellow).bold()),
                    Span::raw(*desc),
                ])
            })
            .collect();

        let header = Row::new(vec!["Key", "Action"])
            .style(Style::default().fg(Color::Cyan).bold())
            .bottom_margin(1);

        Table::new(rows, [Constraint::Length(12), Constraint::Length(30)])
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(" Transfers Shortcuts ")
                    .title_bottom(
                        Line::from(vec![
                            Span::raw(" Press "),
                            Span::styled("?", Style::default().fg(Color::Yellow).bold()),
                            Span::raw(" or "),
                            Span::styled("Esc", Style::default().fg(Color::Yellow).bold()),
                            Span::raw(" to close "),
                        ])
                        .alignment(Alignment::Center),
                    ),
            )
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::vertical([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

        Layout::horizontal([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
    }

    pub fn move_up_table_selection(&mut self) {
        let i = match self.props.table_state.selected() {
            Some(i) => {
                if i == 0_usize && !self.props.selected_items.is_empty() {
                    self.props.selected_items.len() - 1
                } else if i > 0 {
                    i - 1
                } else {
                    0
                }
            }
            None => 0,
        };
        if !self.props.selected_items.len() > i {
            self.props.table_state.select(Some(i));
        }
    }

    pub fn move_down_table_selection(&mut self) {
        let i = match self.props.table_state.selected() {
            Some(i) => {
                if !self.props.selected_items.is_empty() && i >= self.props.selected_items.len() - 1
                {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        if !self.props.selected_items.len() > i {
            self.props.table_state.select(Some(i));
        }
    }

    pub fn unselect_transfer_item(&mut self) {
        if let Some(selected_row) = self
            .props
            .table_state
            .selected()
            .and_then(|index| self.props.selected_items.get(index))
        {
            let sr = selected_row.clone();
            if let Some(s3_item) = self.find_s3_item_from_transfer_item(&sr) {
                let _ = self.action_tx.send(Action::UnselectS3Item {
                    item: s3_item.clone(),
                });
            }
            if let Some(local_item) = self.find_local_item_from_transfer_item(&sr) {
                let _ = self.action_tx.send(Action::UnselectLocalItem {
                    item: local_item.clone(),
                });
            }
        }
    }

    /// Cancel the currently selected transfer (if it has a job_id)
    pub fn cancel_selected_transfer(&mut self) {
        if let Some(selected_row) = self
            .props
            .table_state
            .selected()
            .and_then(|index| self.props.selected_items.get(index))
        {
            if let Some(job_id) = selected_row.job_id {
                let _ = self.action_tx.send(Action::CancelTransfer { job_id });
            }
        }
    }

    /// Pause the currently selected transfer (if it has a job_id and is in progress)
    pub fn pause_selected_transfer(&mut self) {
        if let Some(selected_row) = self
            .props
            .table_state
            .selected()
            .and_then(|index| self.props.selected_items.get(index))
        {
            if let Some(job_id) = selected_row.job_id {
                // Only pause if transfer is in progress
                if !selected_row.transfer_state.is_terminal()
                    && !selected_row.transfer_state.is_paused()
                {
                    let _ = self.action_tx.send(Action::PauseTransfer { job_id });
                }
            }
        }
    }

    /// Resume the currently selected transfer (if it has a job_id and is paused)
    pub fn resume_selected_transfer(&mut self) {
        if let Some(selected_row) = self
            .props
            .table_state
            .selected()
            .and_then(|index| self.props.selected_items.get(index))
        {
            if let Some(job_id) = selected_row.job_id {
                // Only resume if transfer is paused
                if selected_row.transfer_state.is_paused() {
                    let _ = self.action_tx.send(Action::ResumeTransfer { job_id });
                }
            }
        }
    }
    fn find_s3_item_from_transfer_item(
        &self,
        transfer_item: &TransferItem,
    ) -> Option<S3SelectedItem> {
        self.props
            .s3_selected_items
            .iter()
            .find(|&item| {
                if item.is_bucket {
                    item.name == transfer_item.name
                        && item.destination_dir == transfer_item.destination_dir
                } else {
                    item.name == transfer_item.name
                        && item.path == transfer_item.path
                        && item.destination_dir == transfer_item.destination_dir
                }
            })
            .cloned()
    }
    fn find_local_item_from_transfer_item(
        &self,
        transfer_item: &TransferItem,
    ) -> Option<LocalSelectedItem> {
        self.props
            .local_selected_items
            .iter()
            .find(|&item| {
                item.destination_bucket == transfer_item.bucket
                    && item.name == transfer_item.name
                    && item.path.as_str() == transfer_item.path.as_deref().unwrap_or("")
                    && item.destination_path == transfer_item.destination_dir
                    && item.s3_creds == transfer_item.s3_creds
            })
            .cloned()
    }

    fn get_row(&self, item: &TransferItem) -> Row<'_> {
        if item.error().is_some() {
            Row::new(item.to_columns().clone()).fg(Color::Red)
        } else if item.is_cancelled() {
            Row::new(item.to_columns().clone()).fg(Color::DarkGray)
        } else if item.is_paused() {
            Row::new(item.to_columns().clone()).fg(Color::Yellow)
        } else if item.is_transferred() {
            Row::new(item.to_columns().clone()).fg(Color::Blue)
        } else {
            Row::new(item.to_columns().clone())
        }
    }

    fn get_status_line(&self) -> Paragraph<'_> {
        let s3_items = flatten_items(self.props.s3_selected_items.clone());
        let local_items = flatten_items(self.props.local_selected_items.clone());
        let total = s3_items.len() + local_items.len();

        // Count by state
        let completed = s3_items.iter().filter(|i| i.is_transferred()).count()
            + local_items.iter().filter(|i| i.is_transferred()).count();
        let failed = s3_items
            .iter()
            .filter(|i| i.transfer_state.error().is_some())
            .count()
            + local_items
                .iter()
                .filter(|i| i.transfer_state.error().is_some())
                .count();
        let paused = s3_items
            .iter()
            .filter(|i| i.transfer_state.is_paused())
            .count()
            + local_items
                .iter()
                .filter(|i| i.transfer_state.is_paused())
                .count();
        let cancelled = s3_items
            .iter()
            .filter(|i| i.transfer_state.is_cancelled())
            .count()
            + local_items
                .iter()
                .filter(|i| i.transfer_state.is_cancelled())
                .count();
        let in_progress = s3_items
            .iter()
            .filter(|i| {
                !i.is_transferred()
                    && i.transfer_state.error().is_none()
                    && !i.transfer_state.is_paused()
                    && !i.transfer_state.is_cancelled()
                    && i.transfer_state.progress() > 0.0
            })
            .count()
            + local_items
                .iter()
                .filter(|i| {
                    !i.is_transferred()
                        && i.transfer_state.error().is_none()
                        && !i.transfer_state.is_paused()
                        && !i.transfer_state.is_cancelled()
                        && i.transfer_state.progress() > 0.0
                })
                .count();
        let pending = total - completed - failed - in_progress - paused - cancelled;

        // Calculate overall progress percentage (excluding cancelled)
        let active_total = total - cancelled;
        let overall_progress = if active_total > 0 {
            let total_progress: f64 = s3_items
                .iter()
                .filter(|i| !i.transfer_state.is_cancelled())
                .map(|i| i.transfer_state.progress())
                .sum::<f64>()
                + local_items
                    .iter()
                    .filter(|i| !i.transfer_state.is_cancelled())
                    .map(|i| i.transfer_state.progress())
                    .sum::<f64>();
            total_progress / active_total as f64
        } else {
            0.0
        };

        // Build status with progress bar
        let progress_bar = format_progress_bar(overall_progress, 10);
        let mut status = format!(
            " {} {:.0}% | ✓{} ▶{} •{}",
            progress_bar, overall_progress, completed, in_progress, pending
        );
        if paused > 0 {
            status.push_str(&format!(" ⏸{}", paused));
        }
        if failed > 0 {
            status.push_str(&format!(" ✗{}", failed));
        }
        if cancelled > 0 {
            status.push_str(&format!(" ⊘{}", cancelled));
        }

        Paragraph::new(status)
            .style(Style::default().fg(Color::White))
            .bg(Color::Blue)
    }

    fn get_help_line(&self) -> Paragraph<'_> {
        if self.props.s3_selected_items.is_empty() && self.props.local_selected_items.is_empty() {
            Paragraph::new("| 'Esc' - file manager, 's' to select s3 account, ⌫ to remove ")
                .style(Style::default().fg(Color::White))
                .bg(Color::Blue)
                .alignment(Alignment::Right)
        } else {
            Paragraph::new("| 'r' run, 'p' pause, 'u' resume, 'c' cancel, ⌫ remove ")
                .style(Style::default().fg(Color::White))
                .bg(Color::Blue)
                .alignment(Alignment::Right)
        }
    }

    fn get_transfers_table(&self) -> Table<'_> {
        let focus_color = Color::Rgb(98, 114, 164);
        let header = Row::new(vec![
            "↕",
            "Bucket",
            "Item",
            "Destination",
            "Account",
            "Progress",
            "Error",
        ])
        .fg(focus_color)
        .bold()
        .underlined()
        .height(1)
        .bottom_margin(0);
        let rows = self
            .props
            .selected_items
            .iter()
            .map(|item| TransfersPage::get_row(self, item));
        let widths = [
            Constraint::Length(3),
            Constraint::Length(15),
            Constraint::Length(20),
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Length(18),
            Constraint::Length(15),
        ];
        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Transfers List"),
            )
            .row_highlight_style(
                Style::default()
                    .fg(focus_color)
                    .add_modifier(Modifier::REVERSED),
            )
            .widths([
                Constraint::Percentage(3),
                Constraint::Percentage(14),
                Constraint::Percentage(18),
                Constraint::Percentage(18),
                Constraint::Percentage(10),
                Constraint::Percentage(17),
                Constraint::Percentage(15),
            ]);
        table
    }
}

impl ComponentRender<()> for TransfersPage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Take all space left after accounting for the bottom line
                Constraint::Length(1), // Exactly one line for the bottom
            ])
            .split(frame.area());
        let table = self.get_transfers_table();
        frame.render_stateful_widget(
            &table,
            vertical_chunks[0],
            &mut self.props.clone().table_state,
        );
        let status_line = self.get_status_line();
        let help_line = self.get_help_line();
        let status_line_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(vertical_chunks[1]);
        frame.render_widget(status_line, status_line_layout[0]);
        frame.render_widget(help_line, status_line_layout[1]);

        // Show quit confirmation overlay
        if self.show_quit_confirmation {
            QuitConfirmation::render(frame);
        }
        // Show keyboard shortcuts overlay
        else if self.show_shortcuts_overlay {
            let area = Self::centered_rect(50, 50, frame.area());
            frame.render_widget(Clear, area);
            let table = self.make_shortcuts_overlay();
            frame.render_widget(table, area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::local_selected_item::LocalSelectedItem;
    use crate::model::s3_selected_item::S3SelectedItem;
    use crate::model::transfer_state::TransferState;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_key_event_handling() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let state = State::default(); // Assume State::default() properly initializes the state
        let mut page = TransfersPage::new(&state, tx);

        // Test 'r' key for triggering run transfers
        page.handle_key_event(KeyEvent {
            code: KeyCode::Char('r'),
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert_eq!(
            rx.recv().await.unwrap(),
            Action::RunTransfers,
            "Should send RunTransfers action"
        );

        // Test 'q' key shows quit confirmation
        assert!(!page.show_quit_confirmation, "Quit confirmation should be hidden initially");
        page.handle_key_event(KeyEvent {
            code: KeyCode::Char('q'),
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert!(page.show_quit_confirmation, "Quit confirmation should be shown after pressing q");

        // Test Enter in quit confirmation sends Exit
        page.handle_key_event(KeyEvent {
            code: KeyCode::Enter,
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert_eq!(
            rx.recv().await.unwrap(),
            Action::Exit,
            "Should send Exit action after confirming"
        );

        // Reset for next test - show quit confirmation again
        page.show_quit_confirmation = true;
        // Test Esc cancels quit confirmation
        page.handle_key_event(KeyEvent {
            code: KeyCode::Esc,
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert!(!page.show_quit_confirmation, "Quit confirmation should be hidden after pressing Esc");

        // Test 's' key for navigation to S3Creds page
        page.handle_key_event(KeyEvent {
            code: KeyCode::Char('s'),
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert_eq!(
            rx.recv().await.unwrap(),
            Action::Navigate {
                page: ActivePage::S3Creds
            },
            "Should navigate to S3Creds"
        );

        // Test '?' key for showing shortcuts overlay
        assert!(!page.show_shortcuts_overlay, "Overlay should be hidden initially");
        page.handle_key_event(KeyEvent {
            code: KeyCode::Char('?'),
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert!(page.show_shortcuts_overlay, "Overlay should be shown after pressing ?");

        // Test '?' key again to close overlay
        page.handle_key_event(KeyEvent {
            code: KeyCode::Char('?'),
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert!(!page.show_shortcuts_overlay, "Overlay should be hidden after pressing ? again");

        // Test Esc key for navigation back to FileManager page
        page.handle_key_event(KeyEvent {
            code: KeyCode::Esc,
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert_eq!(
            rx.recv().await.unwrap(),
            Action::Navigate {
                page: ActivePage::FileManager
            },
            "Should navigate to FileManager Page"
        );
    }

    #[tokio::test]
    async fn test_initialization() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = State::default();
        let page = TransfersPage::new(&state, tx);

        // Assuming Props::from(&state) initializes TableStates as default and copies selected items lists
        assert!(
            page.props.table_state.selected().is_none(),
            "table state should be initialized to default"
        );
        assert_eq!(
            page.props.selected_items.len(),
            state.s3_selected_items.len(),
            "selected items should match state"
        );
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
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
            job_id: None,
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
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::Failed("Error".into()),
            job_id: None,
        };
        let transfer_item = TransferItem::from_s3_selected_item(item);
        let res = page.get_row(&transfer_item);
        assert_eq!(
            res,
            Row::new(transfer_item.to_columns().clone()).fg(Color::Red)
        );
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
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::Completed,
            job_id: None,
        };
        let transfer_item = TransferItem::from_s3_selected_item(item);
        let res = page.get_row(&transfer_item);
        assert_eq!(
            res,
            Row::new(transfer_item.to_columns().clone()).fg(Color::Blue)
        );
    }

    #[test]
    fn get_local_row_without_modifiers_constructs_plain_row() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = State::default();
        let page = TransfersPage::new(&state, tx);
        let item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::default(),
            job_id: None,
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
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::Completed,
            job_id: None,
        };
        let transfer_item = TransferItem::from_local_selected_item(item);
        let res = page.get_row(&transfer_item);
        assert_eq!(
            res,
            Row::new(transfer_item.to_columns().clone()).fg(Color::Blue)
        );
    }

    #[test]
    fn get_local_row_with_error_constructs_red_row() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = State::default();
        let page = TransfersPage::new(&state, tx);
        let item = LocalSelectedItem {
            destination_bucket: "test-bucket".into(),
            destination_path: "".to_string(),
            name: "file1.txt".into(),
            path: "path/to/file1.txt".into(),
            is_directory: false,
            s3_creds: Default::default(),
            children: None,
            transfer_state: TransferState::Failed("Error".into()),
            job_id: None,
        };
        let transfer_item = TransferItem::from_local_selected_item(item);
        let res = page.get_row(&transfer_item);
        assert_eq!(
            res,
            Row::new(transfer_item.to_columns().clone()).fg(Color::Red)
        );
    }
}
