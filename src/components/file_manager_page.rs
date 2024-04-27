use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use crate::model::action::Action;
use crate::components::component::{Component, ComponentRender};
use crate::model::state::{ActivePage, State};
use crate::model::local_data_item::LocalDataItem;
use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::navigation_state::NavigationState;
use crate::model::s3_data_item::S3DataItem;
use crate::model::s3_selected_item::S3SelectedItem;

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
    current_s3_bucket: String,
    current_s3_path: String,
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
            current_s3_path: st.current_s3_path,
        }
    }
}

pub struct FileManagerPage {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,
    /// State Mapped ChatPage Props
    props: Props,
    s3_panel_selected: bool,
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
                    true => self.handle_go_back_s3(),
                    false => self.handle_go_back_local()
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
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::HelpPage });
            }
            KeyCode::Char('t') => {
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::TransfersPage });
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
        let focus_color = Color::Rgb(98, 114, 164);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(frame.size());
        if self.props.s3_loading {
            let loading_info = self.get_loading_info(focus_color);
            frame.render_widget(&loading_info, chunks[0]);
        } else {
            let s3_table = self.get_s3_table(focus_color);
            frame.render_stateful_widget(&s3_table, chunks[0], &mut self.props.clone().s3_table_state);
        }
        let local_table = self.get_local_table(focus_color);
        frame.render_stateful_widget(&local_table, chunks[1], &mut self.props.clone().local_table_state);
    }
}

impl FileManagerPage {
    fn get_loading_info(&self, focus_color: Color) -> Paragraph {
        Paragraph::new(Text::from("Loading data from s3....").fg(focus_color))
    }

    fn get_local_table(&self, focus_color: Color) -> Table {
        let header =
            Row::new(vec!["Name", "Size", "Type"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.local_data.iter().map(|item| FileManagerPage::get_local_row(self, item));
        let widths = [Constraint::Length(60), Constraint::Length(20), Constraint::Length(20)];
        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Local List").fg(self.get_home_local_color()))
            .highlight_style(Style::default().fg(focus_color).bg(Color::White).add_modifier(Modifier::REVERSED))
            .widths(&[Constraint::Percentage(60), Constraint::Percentage(20), Constraint::Percentage(20)]);
        table
    }

    fn get_s3_row(&self, item: &S3DataItem) -> Row {
        if Self::find_s3_item(&item, &self.props.s3_selected_items) {
            Row::new(item.to_columns().clone()).bg(Color::LightGreen)
        } else {
            Row::new(item.to_columns().clone())
        }
    }

    fn get_local_row(&self, item: &LocalDataItem) -> Row {
        if Self::find_local_item(&item, &self.props.local_selected_items) {
            Row::new(item.to_columns().clone()).bg(Color::LightGreen)
        } else {
            Row::new(item.to_columns().clone())
        }
    }

    fn find_s3_item(data_item: &S3DataItem, selected_items: &[S3SelectedItem]) -> bool {
        let search_item = S3SelectedItem::from(data_item.clone()); // Convert S3DataItem to S3SelectedItem
        selected_items.contains(&search_item) // Search for the item in the list
    }

    fn find_local_item(data_item: &LocalDataItem, selected_items: &[LocalSelectedItem]) -> bool {
        let search_item = LocalSelectedItem::from(data_item.clone());
        selected_items.contains(&search_item) // Search for the item in the list
    }

    fn get_s3_table(&self, focus_color: Color) -> Table {
        let header =
            Row::new(vec!["Name", "Size", "Type"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.s3_data.iter().map(|item| FileManagerPage::get_s3_row(self, item));
        let widths = [Constraint::Length(60), Constraint::Length(20), Constraint::Length(20)];
        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("S3 List").fg(self.get_home_s3_color()))
            .highlight_style(Style::default().fg(focus_color).bg(Color::White).add_modifier(Modifier::REVERSED))
            .widths(&[Constraint::Percentage(60), Constraint::Percentage(20), Constraint::Percentage(20)]);
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
        if self.props.s3_history.len() > 0 {
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
            let selected_item = S3SelectedItem::new(
                sr.name,
                sr.bucket,
                Some(sr.path),
                sr.is_directory,
                sr.is_bucket,
                self.props.current_local_path.clone(),
            );
            let _ = self.action_tx.send(Action::SelectS3Item {
                item: selected_item
            });
        }
    }

    fn transfer_from_local_to_s3(&mut self) {
        if let Some(selected_row) =
            self.props.local_table_state.selected().and_then(|index| self.props.local_data.get(index))
        {
            let sr = selected_row.clone();
            let selected_item = LocalSelectedItem::new(
                sr.name,
                sr.path,
                sr.is_directory,
                self.props.current_s3_bucket.clone(),
                self.props.current_s3_path.clone(),
            );
            let _ = self.action_tx.send(Action::SelectLocalItem {
                item: selected_item
            });
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
                self.props.current_s3_bucket.clone(),
                self.props.current_s3_path.clone(),
            );
            let _ = self.action_tx.send(Action::UnselectLocalItem {
                item: selected_item
            });
        }
    }
}

/*
let mut navigator = Navigator::new();

// Simulating navigation: first into a bucket
navigator.go_into(Some("my-bucket".to_string()), None);
println!("After bucket: {:?}", navigator.current_state());

// Then into a directory
navigator.go_into(None, Some("first_dir".to_string()));
println!("After first dir: {:?}", navigator.current_state());

// And deeper into another directory
navigator.go_into(None, Some("next_dir".to_string()));
println!("After next dir: {:?}", navigator.current_state());

// Going back up one level
navigator.go_up();
println!("After going up: {:?}", navigator.current_state());
 */