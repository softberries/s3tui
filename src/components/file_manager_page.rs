use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use throbber_widgets_tui::Throbber;
use tokio::sync::mpsc::UnboundedSender;
use crate::model::action::Action;
use crate::components::component::{Component, ComponentRender};
use crate::model::state::{ActivePage, State};
use crate::model::local_data_item::LocalDataItem;
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::navigation_state::NavigationState;
use crate::model::s3_data_item::S3DataItem;
use crate::model::s3_selected_item::S3SelectedItem;
use crate::settings::file_credentials::FileCredential;

#[derive(Clone)]
struct Props {
    local_table_state: TableState,
    local_data: Vec<LocalDataItem>,
    s3_table_state: TableState,
    s3_data: Vec<S3DataItem>,
    s3_history: Vec<NavigationState>,
    s3_loading: bool,
    s3_selected_items: Vec<S3SelectedItem>,
    local_selected_items: Vec<LocalSelectedItem>,
    current_local_path: String,
    current_s3_bucket: Option<String>,
    current_s3_path: String,
    current_s3_creds: FileCredential,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        let st = state.clone();
        Props {
            local_table_state: TableState::default(),
            local_data: st.local_data,
            s3_table_state: TableState::default(),
            s3_data: st.s3_data,
            s3_history: Vec::new(),
            s3_loading: st.s3_loading,
            s3_selected_items: st.s3_selected_items,
            local_selected_items: st.local_selected_items,
            current_local_path: st.current_local_path,
            current_s3_bucket: st.current_s3_bucket,
            current_s3_path: st.current_s3_path.unwrap_or("/".to_string()),
            current_s3_creds: st.current_creds,
        }
    }
}

pub struct FileManagerPage {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,
    /// State Mapped ChatPage Props
    props: Props,
    s3_panel_selected: bool,
    show_popup: bool,
    default_navigation_state: NavigationState,
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
            show_popup: false,
            s3_panel_selected: true,
            default_navigation_state: NavigationState::new(None, None),
        }
            .move_with_state(state)
    }

    fn move_with_state(self, state: &State) -> Self
        where
            Self: Sized,
    {
        let new_props = Props::from(state);
        FileManagerPage {
            props: Props {
                s3_history: self.props.s3_history.clone(),
                s3_table_state: self.props.s3_table_state.clone(),
                local_table_state: self.props.local_table_state.clone(),
                ..new_props
            },
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
            KeyCode::Char('j') | KeyCode::Down => {
                match self.s3_panel_selected {
                    true => self.move_down_s3_table_selection(),
                    false => self.move_down_local_table_selection()
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match self.s3_panel_selected {
                    true => self.move_up_s3_table_selection(),
                    false => self.move_up_local_table_selection()
                }
            }
            KeyCode::Enter => {
                match self.s3_panel_selected {
                    true => self.handle_selected_s3_row(),
                    false => self.handle_selected_local_row()
                }
            }
            KeyCode::Esc => {
                match self.s3_panel_selected {
                    true => {
                        if !self.props.s3_loading {
                            self.handle_go_back_s3()
                        }
                    }
                    false => {
                        if self.show_popup {
                            self.show_popup = false;
                        } else {
                            self.handle_go_back_local()
                        }
                    }
                }
            }
            KeyCode::Right => {
                if self.s3_panel_selected {
                    self.transfer_from_s3_to_local()
                } else {
                    self.cancel_transfer_from_local_to_s3()
                }
            }
            KeyCode::Left => {
                if self.s3_panel_selected {
                    self.cancel_transfer_from_s3_to_local()
                } else {
                    self.transfer_from_local_to_s3()
                }
            }
            KeyCode::Char('?') => {
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::Help });
            }
            KeyCode::Char('t') => {
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::Transfers });
            }
            KeyCode::Char('s') => {
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::S3Creds });
            }
            KeyCode::Tab => {
                self.s3_panel_selected = !&self.s3_panel_selected;
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
        let focus_color = Color::LightBlue;
        // Split the frame into two main vertical sections
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),   // Take all space left after accounting for the bottom line
                Constraint::Length(1) // Exactly one line for the bottom
            ])
            .split(frame.size());

        // Now split the top part horizontally into two side-by-side areas
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(vertical_chunks[0]);  // Apply this layout to the main area

        if self.props.s3_loading {
            let chunks_h = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25), // Adjust this percentage to better center the text
                    Constraint::Percentage(50),
                    Constraint::Percentage(25),
                ])
                .split(horizontal_chunks[0]);

            // Define vertical constraints: top, middle (50% of available height), bottom
            let chunks_v = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(25), // Adjust this percentage to better center the text
                    Constraint::Percentage(50),
                    Constraint::Percentage(25),
                ])
                .split(chunks_h[1]); // Apply vertical layout to the center horizontal chunk

            let loading_info = self.get_loading_info();
            let loader_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage((100 - 50) / 2),
                    Constraint::Percentage(50),
                    Constraint::Percentage((100 - 50) / 2),
                ])
                .split(chunks_v[1]);
            frame.render_widget(loading_info, loader_layout[1]);
        } else {
            let s3_table = self.get_s3_table(focus_color);
            frame.render_stateful_widget(&s3_table, horizontal_chunks[0], &mut self.props.clone().s3_table_state);
        }
        let local_table = self.get_local_table(focus_color);
        frame.render_stateful_widget(&local_table, horizontal_chunks[1], &mut self.props.clone().local_table_state);
        let to_transfer = self.props.s3_selected_items.len() + self.props.local_selected_items.len();
        let transferred = self.props.s3_selected_items.iter().filter(|i| i.transferred).count() +
            self.props.local_selected_items.iter().filter(|i| i.transferred).count();
        if let Some(bucket) = &self.props.current_s3_bucket {
            let bottom_text = Paragraph::new(format!(" Account: {} • Bucket: {} • Transfers: {}/{}", self.props.current_s3_creds.name, bucket, to_transfer, transferred))
                .style(Style::default().fg(Color::White)).bg(Color::Blue);
            frame.render_widget(bottom_text, vertical_chunks[1]);
        } else {
            let bottom_text = Paragraph::new(format!(" Account: {} • Transfers: {}/{}", self.props.current_s3_creds.name, to_transfer, transferred))
                .style(Style::default().fg(Color::White)).bg(Color::Blue);
            frame.render_widget(bottom_text, vertical_chunks[1]);
        }

        if self.show_popup {
            let block = Block::default().title("Problem detected").borders(Borders::ALL).fg(Color::Red);
            let area = Self::centered_rect(60, 20, frame.size());
            frame.render_widget(Clear, area); //this clears out the background
            frame.render_widget(block, area);
            // Define the text for the paragraph
            let text = "   To move data into s3 you need to select at least a bucket to which you want to transfer your files";

            // Create the paragraph widget
            let paragraph = Paragraph::new(text)
                .block(Block::default()) // Optional: set another block here if you want borders around the text
                .alignment(Alignment::Left); // Set text alignment within the paragraph

            // Optionally, adjust the area inside the block for the paragraph content
            // You might want to shrink the area to leave some padding inside the block borders
            let inner_area = Rect::new(area.x + 1, area.y + 2, area.width - 2, area.height - 2);

            // Render the paragraph widget
            frame.render_widget(paragraph, inner_area);
        }
    }
}

impl FileManagerPage {
    fn get_loading_info(&self) -> Throbber {
        Throbber::default().label("Loading s3 data...").style(Style::default().fg(Color::White))
            .throbber_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
    }

    fn get_local_table(&self, focus_color: Color) -> Table {
        let header =
            Row::new(vec!["Name", "Size", "Type"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.local_data.iter().map(|item| FileManagerPage::get_local_row(self, item));
        let widths = [Constraint::Length(60), Constraint::Length(20), Constraint::Length(20)];
        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(format!("Local List ({} objects)", self.props.local_data.len())).fg(self.get_home_local_color()))
            .highlight_style(Style::default().fg(focus_color).bg(Color::White).add_modifier(Modifier::REVERSED))
            .widths([Constraint::Percentage(60), Constraint::Percentage(20), Constraint::Percentage(20)]);
        table
    }

    fn get_s3_row(&self, item: &S3DataItem) -> Row {
        if Self::contains_s3_item(item, &self.props.s3_selected_items, &self.props.current_s3_creds) {
            Row::new(item.to_columns().clone()).bg(Color::LightGreen)
        } else {
            Row::new(item.to_columns().clone())
        }
    }

    fn get_local_row(&self, item: &LocalDataItem) -> Row {
        if Self::contains_local_item(item, &self.props.local_selected_items, &self.props.current_s3_creds) {
            Row::new(item.to_columns().clone()).bg(Color::LightGreen)
        } else {
            Row::new(item.to_columns().clone())
        }
    }

    fn contains_s3_item(data_item: &S3DataItem, selected_items: &[S3SelectedItem], s3_creds: &FileCredential) -> bool {
        let search_item = S3SelectedItem::from_s3_data_item(data_item.clone(), s3_creds.clone()); // Convert S3DataItem to S3SelectedItem
        selected_items.contains(&search_item) // Search for the item in the list
    }

    fn contains_local_item(data_item: &LocalDataItem, selected_items: &[LocalSelectedItem], s3_creds: &FileCredential) -> bool {
        let search_item = LocalSelectedItem::from_local_data_item(data_item.clone(), s3_creds.clone());
        selected_items.contains(&search_item) // Search for the item in the list
    }

    fn get_s3_table(&self, focus_color: Color) -> Table {
        let header =
            Row::new(vec!["Name", "Size", "Type"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.s3_data.iter().map(|item| FileManagerPage::get_s3_row(self, item));
        let widths = [Constraint::Length(60), Constraint::Length(20), Constraint::Length(20)];
        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(format!("S3 List ({} objects)", self.props.s3_data.len())).fg(self.get_home_s3_color()))
            .highlight_style(Style::default().fg(focus_color).bg(Color::White).add_modifier(Modifier::REVERSED))
            .widths([Constraint::Percentage(60), Constraint::Percentage(20), Constraint::Percentage(20)]);
        table
    }

    fn get_home_s3_color(&self) -> Color {
        match self.s3_panel_selected {
            true => Color::White,
            false => Color::Blue,
        }
    }
    fn get_home_local_color(&self) -> Color {
        match self.s3_panel_selected {
            false => Color::White,
            true => Color::Blue,
        }
    }

    pub fn move_up_s3_table_selection(&mut self) {
        let i = match self.props.s3_table_state.selected() {
            Some(i) => {
                if i == 0_usize {
                    self.props.s3_data.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.props.s3_table_state.select(Some(i));
    }

    pub fn move_down_s3_table_selection(&mut self) {
        let i = match self.props.s3_table_state.selected() {
            Some(i) => {
                if i >= self.props.s3_data.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.props.s3_table_state.select(Some(i));
    }

    pub fn move_up_local_table_selection(&mut self) {
        let i = match self.props.local_table_state.selected() {
            Some(i) => {
                if i == 0_usize {
                    self.props.local_data.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.props.local_table_state.select(Some(i));
    }

    pub fn move_down_local_table_selection(&mut self) {
        let i = match self.props.local_table_state.selected() {
            Some(i) => {
                if i >= self.props.local_data.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.props.local_table_state.select(Some(i));
    }

    pub fn handle_selected_local_row(&mut self) {
        if let Some(selected_row) =
            self.props.local_table_state.selected().and_then(|index| self.props.local_data.get(index))
        {
            if selected_row.is_directory {
                let _ = self.action_tx.send(Action::FetchLocalData { path: selected_row.path.clone() });
                // self.fetch_local_data(selected_row.path.clone());
            }
        }
    }

    pub fn handle_selected_s3_row(&mut self) {
        if let Some(selected_row) =
            self.props.s3_table_state.selected().and_then(|index| self.props.s3_data.get(index))
        {
            if selected_row.is_bucket {
                self.go_into(Some(selected_row.path.clone()), None);
                let _ = self.action_tx.send(Action::FetchS3Data {
                    bucket: self.current_state().current_bucket.clone(),
                    prefix: self.current_state().current_prefix.clone(),
                });
            } else if selected_row.is_directory {
                self.go_into(None, Some(selected_row.path.clone())); //get bucket and prefix from previous entries
                let _ = self.action_tx.send(Action::FetchS3Data {
                    bucket: self.current_state().current_bucket.clone(),
                    prefix: self.current_state().current_prefix.clone(),
                });
            }
        }
    }

    fn go_into(&mut self, bucket: Option<String>, prefix: Option<String>) {
        if let Some(b) = bucket {
            self.props.s3_history.clear();
            self.props.s3_history.push(NavigationState::new(Some(b.clone()), None));
        }
        if let Some(p) = prefix {
            // Navigate into a new directory within the current bucket
            let current_state = self.current_state().clone();
            let new_prefix = match &current_state.current_prefix {
                Some(current_prefix) => format!("{}/{}", current_prefix, p),
                None => p,
            };
            self.props.s3_history.push(NavigationState::new(current_state.current_bucket.clone(), Some(new_prefix)));
        }
    }

    fn go_up(&mut self) {
        if !self.props.s3_history.is_empty() {
            self.props.s3_history.pop();
        }
    }

    fn current_state(&self) -> &NavigationState {
        self.props.s3_history.last().unwrap_or(&self.default_navigation_state)
    }
    pub fn handle_go_back_local(&mut self) {
        let _ = self.action_tx.send(Action::MoveBackLocal);
    }

    pub fn handle_go_back_s3(&mut self) {
        self.go_up();
        let _ = self.action_tx.send(Action::FetchS3Data {
            bucket: self.current_state().current_bucket.clone(),
            prefix: self.current_state().current_prefix.clone(),
        });
    }

    fn transfer_from_s3_to_local(&mut self) {
        if let Some(selected_row) =
            self.props.s3_table_state.selected().and_then(|index| self.props.s3_data.get(index))
        {
            let sr = selected_row.clone();
            //disable sending whole directories/buckets
            if !sr.is_directory && !sr.is_bucket {
                let cc = self.props.current_s3_creds.clone();
                let creds = FileCredential {
                    default_region: sr.region.unwrap_or(cc.default_region.clone()),
                    ..cc
                };
                let selected_item = S3SelectedItem::new(
                    sr.name,
                    sr.bucket,
                    Some(sr.path),
                    sr.is_directory,
                    sr.is_bucket,
                    self.props.current_local_path.clone(),
                    creds,
                );
                let _ = self.action_tx.send(Action::SelectS3Item {
                    item: selected_item
                });
            }
        }
    }

    fn transfer_from_local_to_s3(&mut self) {
        if let Some(selected_row) =
            self.props.local_table_state.selected().and_then(|index| self.props.local_data.get(index))
        {
            let sr = selected_row.clone();
            //disable selecting whole directories
            if !sr.is_directory {
                if let Some(selected_bucket) = self.props.current_s3_bucket.clone() {
                    let selected_item = LocalSelectedItem::new(
                        sr.name,
                        sr.path,
                        sr.is_directory,
                        selected_bucket,
                        self.props.current_s3_path.clone(),
                        self.props.current_s3_creds.clone(),
                    );
                    let _ = self.action_tx.send(Action::SelectLocalItem {
                        item: selected_item
                    });
                } else {
                    self.show_popup = true;
                }
            }
        }
    }

    fn cancel_transfer_from_s3_to_local(&mut self) {
        if let Some(selected_row) =
            self.props.s3_table_state.selected().and_then(|index| self.props.s3_data.get(index))
        {
            let sr = selected_row.clone();
            let selected_item = S3SelectedItem::new(
                sr.name,
                sr.bucket,
                Some(sr.path),
                sr.is_directory,
                sr.is_bucket,
                self.props.current_local_path.clone(),
                self.props.current_s3_creds.clone(),
            );
            let _ = self.action_tx.send(Action::UnselectS3Item {
                item: selected_item
            });
        }
    }

    fn cancel_transfer_from_local_to_s3(&mut self) {
        if let Some(selected_row) =
            self.props.local_table_state.selected().and_then(|index| self.props.local_data.get(index))
        {
            let sr = selected_row.clone();
            let selected_item = LocalSelectedItem::new(
                sr.name,
                sr.path,
                sr.is_directory,
                self.props.current_s3_bucket.clone().expect("Bucket has to be set for the selected item"),
                self.props.current_s3_path.clone(),
                self.props.current_s3_creds.clone(),
            );
            let _ = self.action_tx.send(Action::UnselectLocalItem {
                item: selected_item
            });
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_navigation_keys() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let state = State::default(); // Define a default or a mock state as needed
        let mut page = FileManagerPage::new(&state, tx);

        // Ensure the S3 panel is selected initially
        assert!(page.s3_panel_selected, "S3 panel should be initially selected");

        // Test tab switching
        page.handle_key_event(KeyEvent {
            code: KeyCode::Tab,
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        });
        assert!(!page.s3_panel_selected, "Local panel should be selected after tab");
    }
}
