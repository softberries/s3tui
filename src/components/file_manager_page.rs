use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use ratatui::widgets::block::Title;
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
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

static INPUT_SIZE: usize = 60;

#[derive(Clone)]
struct Props {
    local_table_state: TableState,
    local_data: Vec<LocalDataItem>,
    s3_table_state: TableState,
    s3_data: Vec<S3DataItem>,
    s3_data_full_list: Vec<S3DataItem>,
    s3_history: Vec<NavigationState>,
    s3_loading: bool,
    s3_list_recursive_loading: bool,
    s3_selected_items: Vec<S3SelectedItem>,
    local_selected_items: Vec<LocalSelectedItem>,
    current_local_path: String,
    current_s3_bucket: Option<String>,
    current_s3_path: String,
    current_s3_creds: FileCredential,
    s3_delete_state: Option<String>,
    local_delete_state: Option<String>,
    create_bucket_state: Option<String>,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        let st = state.clone();
        Props {
            local_table_state: TableState::default(),
            local_data: st.local_data,
            s3_table_state: TableState::default(),
            s3_data: st.s3_data,
            s3_data_full_list: st.s3_data_full_list,
            s3_history: Vec::new(),
            s3_loading: st.s3_loading,
            s3_list_recursive_loading: st.s3_list_recursive_loading,
            s3_selected_items: st.s3_selected_items,
            local_selected_items: st.local_selected_items,
            current_local_path: st.current_local_path,
            current_s3_bucket: st.current_s3_bucket,
            current_s3_path: st.current_s3_path.unwrap_or("/".to_string()),
            current_s3_creds: st.current_creds,
            s3_delete_state: st.s3_delete_state,
            local_delete_state: st.local_delete_state,
            create_bucket_state: st.create_bucket_state,
        }
    }
}

/// Page displaying s3 and local items in the current path
/// This is the main page of the application where most of the interactions are happening
/// You can always navigate to this page from the other pages by clicking 'Esc' key
pub struct FileManagerPage {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,
    /// State Mapped ChatPage Props
    props: Props,
    s3_panel_selected: bool,
    show_problem_popup: bool,
    show_bucket_input: bool,
    show_delete_confirmation: bool,
    show_download_confirmation: bool,
    show_delete_error: bool,
    default_navigation_state: NavigationState,
    input: Input,
}


impl FileManagerPage {
    fn make_transfer_error_popup(&self) -> Paragraph {
        // Define the text for the paragraph
        let text = "   To move data into s3 you need to select at least a bucket to which you want to transfer your files";
        // Create the paragraph widget
        Paragraph::new(text)
            .block(Block::default()) // Optional: set another block here if you want borders around the text
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default()
                    )
                    .title(
                        ratatui::widgets::block::Title::from(Line::from(vec![
                            Span::raw("|"),
                            Span::styled("cancel", Style::default().fg(Color::Yellow)),
                            Span::raw("("),
                            Span::styled(
                                "Esc",
                                Style::default().add_modifier(Modifier::BOLD).fg(Color::Red),
                            ),
                            Span::raw(")"),
                            Span::raw("|"),
                        ]))
                            .alignment(Alignment::Left)
                            .position(ratatui::widgets::block::Position::Bottom),
                    )
                    .title(
                        ratatui::widgets::block::Title::from(Line::from(vec![
                            Span::raw("| Problem detected! |"),
                        ]))
                            .alignment(Alignment::Left)
                            .position(ratatui::widgets::block::Position::Top),
                    )
            ).fg(Color::Red)
    }

    fn make_delete_alert(&self, text: String, text_color: Color) -> Paragraph {
        let input = Paragraph::new(text)
            .style(Style::default().fg(text_color))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default()
                    )
                    .title(
                        ratatui::widgets::block::Title::from(Line::from(vec![
                            Span::raw("|"),
                            Span::styled("ok", Style::default().fg(Color::Yellow)),
                            Span::raw("("),
                            Span::styled(
                                "Enter",
                                Style::default().add_modifier(Modifier::BOLD).fg(Color::Red),
                            ),
                            Span::raw(")"),
                            Span::raw("|"),
                        ]))
                            .alignment(Alignment::Right)
                            .position(block::Position::Bottom),
                    )
                    .title(
                        ratatui::widgets::block::Title::from(Line::from(vec![
                            Span::raw("|"),
                            Span::styled("cancel", Style::default().fg(Color::Yellow)),
                            Span::raw("("),
                            Span::styled(
                                "Esc",
                                Style::default().add_modifier(Modifier::BOLD).fg(Color::Red),
                            ),
                            Span::raw(")"),
                            Span::raw("|"),
                        ]))
                            .alignment(Alignment::Left)
                            .position(block::Position::Bottom),
                    )
            );
        input
    }

    fn make_confirm_download_alert(&self, text: String, text_color: Color, show_buttons: bool) -> Paragraph {
        let ok_button = ratatui::widgets::block::Title::from(Line::from(vec![
            Span::raw("|"),
            Span::styled("ok", Style::default().fg(Color::Yellow)),
            Span::raw("("),
            Span::styled(
                "Enter",
                Style::default().add_modifier(Modifier::BOLD).fg(Color::Red),
            ),
            Span::raw(")"),
            Span::raw("|"),
        ]))
            .alignment(Alignment::Right)
            .position(block::Position::Bottom);
        let cancel_button = ratatui::widgets::block::Title::from(Line::from(vec![
            Span::raw("|"),
            Span::styled("cancel", Style::default().fg(Color::Yellow)),
            Span::raw("("),
            Span::styled(
                "Esc",
                Style::default().add_modifier(Modifier::BOLD).fg(Color::Red),
            ),
            Span::raw(")"),
            Span::raw("|"),
        ]))
            .alignment(Alignment::Left)
            .position(block::Position::Bottom);
        let input = Paragraph::new(text)
            .style(Style::default().fg(text_color))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default()
                    )
                    .title(
                        if show_buttons {
                            ok_button
                        } else {
                            Title::default()
                        },
                    )
                    .title(
                        if show_buttons {
                            cancel_button
                        } else {
                            Title::default()
                        },
                    )
            );
        input
    }
    fn make_bucket_name_input(&self) -> Paragraph {
        let scroll = self.input.visual_scroll(INPUT_SIZE);
        let input = Paragraph::new(self.input.value())
            .style(Style::default().fg(Color::Green))
            .scroll((0, scroll as u16))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default()
                    )
                    .title(
                        ratatui::widgets::block::Title::from(Line::from(vec![
                            Span::raw("|"),
                            Span::styled("save", Style::default().fg(Color::Yellow)),
                            Span::raw("("),
                            Span::styled(
                                "Enter",
                                Style::default().add_modifier(Modifier::BOLD).fg(Color::Red),
                            ),
                            Span::raw(")"),
                            Span::raw("|"),
                        ]))
                            .alignment(Alignment::Right)
                            .position(ratatui::widgets::block::Position::Bottom),
                    )
                    .title(
                        ratatui::widgets::block::Title::from(Line::from(vec![
                            Span::raw("|"),
                            Span::styled("cancel", Style::default().fg(Color::Yellow)),
                            Span::raw("("),
                            Span::styled(
                                "Esc",
                                Style::default().add_modifier(Modifier::BOLD).fg(Color::Red),
                            ),
                            Span::raw(")"),
                            Span::raw("|"),
                        ]))
                            .alignment(Alignment::Left)
                            .position(ratatui::widgets::block::Position::Bottom),
                    )
                    .title(
                        ratatui::widgets::block::Title::from(Line::from(vec![
                            Span::raw("| Enter new bucket name |"),
                        ]))
                            .alignment(Alignment::Left)
                            .position(ratatui::widgets::block::Position::Top),
                    )
            );
        input
    }

    fn get_loading_info(&self) -> Throbber {
        Throbber::default().label("Loading s3 data...").style(Style::default())
            .throbber_style(Style::default().add_modifier(Modifier::BOLD))
    }

    fn get_local_table(&self, focus_color: Color) -> Table {
        let header =
            Row::new(vec!["Name", "Size", "Type"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.local_data.iter().map(|item| FileManagerPage::get_local_row(self, item, focus_color));
        let widths = [Constraint::Length(60), Constraint::Length(20), Constraint::Length(20)];
        let block = self.get_home_local_block();
        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .highlight_style(Style::default().fg(focus_color).bold().add_modifier(Modifier::REVERSED))
            .widths([Constraint::Percentage(60), Constraint::Percentage(20), Constraint::Percentage(20)]);
        table
    }

    fn flatten_s3_items(&self, s3_selected_items: Vec<S3SelectedItem>) -> Vec<S3SelectedItem> {
        let nested: Vec<Vec<S3SelectedItem>> = s3_selected_items.iter().map(|i| i.clone().children.unwrap_or_default()).collect();
        let mut children: Vec<S3SelectedItem> = nested.into_iter().flatten().collect();
        let single_files: Vec<S3SelectedItem> = s3_selected_items.into_iter().filter(|i| i.children.is_none()).collect();
        children.extend(single_files);
        children
    }

    fn flatten_local_items(&self, local_selected_items: Vec<LocalSelectedItem>) -> Vec<LocalSelectedItem> {
        let nested: Vec<Vec<LocalSelectedItem>> = local_selected_items.iter().map(|i| i.clone().children.unwrap_or_default()).collect();
        let mut children: Vec<LocalSelectedItem> = nested.into_iter().flatten().collect();
        let single_files: Vec<LocalSelectedItem> = local_selected_items.into_iter().filter(|i| i.children.is_none()).collect();
        children.extend(single_files);
        children
    }

    fn get_status_line(&self) -> Paragraph {
        let s3_items = self.flatten_s3_items(self.props.s3_selected_items.clone());
        let local_items = self.flatten_local_items(self.props.local_selected_items.clone());
        let to_transfer = s3_items.len() + local_items.len();
        let transferred = s3_items.iter().filter(|i| i.transferred).count() +
            local_items.iter().filter(|i| i.transferred).count();
        if let Some(bucket) = &self.props.current_s3_bucket {
            let bottom_text = Paragraph::new(format!(" Account: {} • Bucket: {} • Transfers: {}/{}", self.props.current_s3_creds.name, bucket, to_transfer, transferred))
                .style(Style::default().fg(Color::White)).bg(Color::Blue);
            bottom_text
        } else {
            let bottom_text = Paragraph::new(format!(" Account: {} • Transfers: {}/{}", self.props.current_s3_creds.name, to_transfer, transferred))
                .style(Style::default().fg(Color::White)).bg(Color::Blue);
            bottom_text
        }
    }

    fn get_help_line(&self) -> Paragraph {
        if self.props.s3_selected_items.is_empty() && self.props.local_selected_items.is_empty() {
            Paragraph::new("| 't' transfer select, 's' s3 account, 'l' transfers list, 'Esc/Enter' browsing")
                .style(Style::default().fg(Color::White)).bg(Color::Blue)
                .alignment(Alignment::Right)
        } else {
            Paragraph::new("| Press 'l' to see the transfers list,'s' to select s3 account ")
                .style(Style::default().fg(Color::White)).bg(Color::Blue)
                .alignment(Alignment::Right)
        }
    }

    fn get_s3_row(&self, item: &S3DataItem, focus_color: Color) -> Row {
        if self.contains_s3_item(item, &self.props.s3_selected_items, &self.props.current_s3_creds) {
            Row::new(item.to_columns().clone()).fg(focus_color).add_modifier(Modifier::REVERSED)
        } else {
            Row::new(item.to_columns().clone())
        }
    }

    fn get_local_row(&self, item: &LocalDataItem, focus_color: Color) -> Row {
        if self.contains_local_item(item, &self.props.local_selected_items, &self.props.current_s3_creds) {
            Row::new(item.to_columns().clone()).fg(focus_color).add_modifier(Modifier::REVERSED)
        } else {
            Row::new(item.to_columns().clone())
        }
    }

    fn contains_s3_item(&self, data_item: &S3DataItem, selected_items: &[S3SelectedItem], s3_creds: &FileCredential) -> bool {
        let destination_dir = self.props.current_local_path.clone();
        let search_item = S3SelectedItem::from_s3_data_item(data_item.clone(), s3_creds.clone(), destination_dir.clone()); // Convert S3DataItem to S3SelectedItem
        selected_items.contains(&search_item) // Search for the item in the list
    }

    fn contains_local_item(&self, data_item: &LocalDataItem, selected_items: &[LocalSelectedItem], s3_creds: &FileCredential) -> bool {
        let search_item = LocalSelectedItem::from_local_data_item(data_item.clone(), s3_creds.clone());
        selected_items.contains(&search_item) // Search for the item in the list
    }

    fn get_s3_table(&self, focus_color: Color) -> Table {
        let header =
            Row::new(vec!["Name", "Size", "Type"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.s3_data.iter().map(|item| FileManagerPage::get_s3_row(self, item, focus_color));
        let widths = [Constraint::Length(60), Constraint::Length(20), Constraint::Length(20)];
        let block = self.get_home_s3_block();
        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .highlight_style(Style::default().fg(focus_color).bold().add_modifier(Modifier::REVERSED))
            .widths([Constraint::Percentage(60), Constraint::Percentage(20), Constraint::Percentage(20)]);
        table
    }

    fn get_home_s3_block(&self) -> Block {
        if self.s3_panel_selected {
            Block::default().borders(Borders::ALL).title(format!("S3 List ({} objects)", self.props.s3_data.len())).fg(Color::Blue)
        } else {
            Block::default().borders(Borders::ALL).title(format!("S3 List ({} objects)", self.props.s3_data.len()))
        }
    }


    fn get_home_local_block(&self) -> Block {
        if !self.s3_panel_selected {
            Block::default().borders(Borders::ALL).title(format!("Local List ({} objects)", self.props.local_data.len())).fg(Color::Blue)
        } else {
            Block::default().borders(Borders::ALL).title(format!("Local List ({} objects)", self.props.local_data.len()))
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
        if !self.props.s3_data.is_empty() {
            self.props.s3_table_state.select(Some(i));
        }
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
        if !self.props.s3_data.is_empty() {
            self.props.s3_table_state.select(Some(i));
        }
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
        if !self.props.local_data.is_empty() {
            self.props.local_table_state.select(Some(i));
        }
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
        if !self.props.local_data.is_empty() {
            self.props.local_table_state.select(Some(i));
        }
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
            self.props.s3_history.push(NavigationState::new(current_state.current_bucket.clone(), Some(p)));
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
                None,
            );
            if !self.props.s3_selected_items.contains(&selected_item) {
                if selected_item.is_bucket || selected_item.is_directory {
                    self.show_download_confirmation = true;
                    self.props.s3_list_recursive_loading = true;
                    let _ = self.action_tx.send(Action::ListS3DataRecursiveForItem {
                        item: selected_item
                    });
                } else {
                    let _ = self.action_tx.send(Action::SelectS3Item {
                        item: selected_item
                    });
                }
            } else {
                let _ = self.action_tx.send(Action::UnselectS3Item {
                    item: selected_item
                });
            }
        }
    }

    fn finish_recursive_transfer_from_s3_to_local(&mut self) {
        if let Some(selected_row) =
            self.props.s3_table_state.selected().and_then(|index| self.props.s3_data.get(index))
        {
            let sr = selected_row.clone();
            let cc = self.props.current_s3_creds.clone();
            let creds = FileCredential {
                default_region: sr.region.unwrap_or(cc.default_region.clone()),
                ..cc
            };
            let destination_dir = self.props.current_local_path.clone();
            let children = self.props.s3_data_full_list.iter().map(|i| S3SelectedItem::from_s3_data_item(i.clone(), creds.clone(), destination_dir.clone())).collect();
            let selected_item = S3SelectedItem::new(
                sr.name,
                sr.bucket,
                Some(sr.path),
                sr.is_directory,
                sr.is_bucket,
                self.props.current_local_path.clone(),
                creds.clone(),
                Some(children),
            );
            if !self.props.s3_selected_items.contains(&selected_item) {
                let _ = self.action_tx.send(Action::SelectS3Item {
                    item: selected_item
                });
            } else {
                let _ = self.action_tx.send(Action::UnselectS3Item {
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
            if let Some(selected_bucket) = self.props.current_s3_bucket.clone() {
                let destination_path = if sr.is_directory {
                    sr.name.clone()
                } else {
                    "/".to_string()
                };
                let selected_item = LocalSelectedItem::new(
                    sr.name.clone(),
                    sr.path,
                    sr.is_directory,
                    selected_bucket,
                    destination_path,
                    self.props.current_s3_creds.clone(),
                    None,
                );
                if !self.props.local_selected_items.contains(&selected_item) {
                    let _ = self.action_tx.send(Action::SelectLocalItem {
                        item: selected_item
                    });
                } else {
                    let _ = self.action_tx.send(Action::UnselectLocalItem {
                        item: selected_item
                    });
                }
            } else {
                self.show_problem_popup = true;
            }
        }
    }

    fn delete_selected_s3_item(&mut self) {
        if let Some(selected_row) =
            self.props.s3_table_state.selected().and_then(|index| self.props.s3_data.get(index))
        {
            let sr = selected_row.clone();
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
                None,
            );
            let _ = self.action_tx.send(Action::DeleteS3Item {
                item: selected_item
            });
        }
    }

    fn delete_selected_local_item(&mut self) {
        if let Some(selected_row) =
            self.props.local_table_state.selected().and_then(|index| self.props.local_data.get(index))
        {
            let sr = selected_row.clone();
            let selected_item = LocalSelectedItem::new(
                sr.name,
                sr.path,
                sr.is_directory,
                "".to_string(),
                self.props.current_s3_path.clone(),
                self.props.current_s3_creds.clone(),
                None,
            );
            let _ = self.action_tx.send(Action::DeleteLocalItem {
                item: selected_item
            });
        }
    }

    fn send_clear_delete_errors_message(&mut self) {
        let _ = self.action_tx.send(Action::ClearDeletionErrors);
        self.show_delete_error = false;
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

impl Component for FileManagerPage {
    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
        where
            Self: Sized,
    {
        FileManagerPage {
            action_tx: action_tx.clone(),
            props: Props::from(state),
            show_problem_popup: false,
            show_bucket_input: false,
            show_delete_confirmation: false,
            show_download_confirmation: false,
            show_delete_error: false,
            s3_panel_selected: true,
            default_navigation_state: NavigationState::new(None, None),
            input: Input::default().with_value(String::from("")),
        }
            .move_with_state(state)
    }


    fn move_with_state(self, state: &State) -> Self
        where
            Self: Sized,
    {
        let new_props = Props::from(state);
        FileManagerPage {
            show_delete_error: state.s3_delete_state.is_some() || state.local_delete_state.is_some(),
            show_bucket_input: state.create_bucket_state.is_some(),
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
        if self.show_bucket_input {
            match key.code {
                KeyCode::Enter => {
                    let _ = self.action_tx.send(Action::CreateBucket {
                        name: self.input.value().to_string()
                    });
                    self.show_bucket_input = false;
                }
                KeyCode::Esc => {
                    self.show_bucket_input = false;
                    self.send_clear_delete_errors_message();
                }
                _ => {
                    let _ = self.input.handle_event(&crossterm::event::Event::Key(key));
                }
            }
        } else if self.show_delete_confirmation {
            match key.code {
                KeyCode::Enter => {
                    match self.s3_panel_selected {
                        true => {
                            self.delete_selected_s3_item();
                            self.props.s3_loading = true;
                        }
                        false => {
                            self.delete_selected_local_item();
                        }
                    }
                    self.show_delete_confirmation = false;
                }
                KeyCode::Esc => {
                    self.show_delete_confirmation = false;
                }
                _ => {}
            }
        } else if self.show_delete_error {
            match key.code {
                KeyCode::Enter => {
                    self.send_clear_delete_errors_message();
                }
                KeyCode::Esc => {
                    self.send_clear_delete_errors_message();
                }
                _ => {}
            }
        } else if self.show_download_confirmation && !self.props.s3_list_recursive_loading {
            match key.code {
                KeyCode::Enter => {
                    self.finish_recursive_transfer_from_s3_to_local();
                    self.show_download_confirmation = false;
                }
                KeyCode::Esc => {
                    self.show_download_confirmation = false;
                }
                _ => {}
            }
        } else {
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
                KeyCode::Char('c') => {
                    if self.s3_panel_selected {
                        self.input.reset();
                        self.show_bucket_input = true;
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
                            if self.show_problem_popup {
                                self.show_problem_popup = false;
                            } else {
                                self.handle_go_back_local()
                            }
                        }
                    }
                }
                KeyCode::Delete | KeyCode::Backspace => {
                    self.show_delete_confirmation = true;
                }
                KeyCode::Char('t') => {
                    if self.s3_panel_selected {
                        self.transfer_from_s3_to_local()
                    } else {
                        self.transfer_from_local_to_s3()
                    }
                }
                KeyCode::Left => {
                    self.s3_panel_selected = true;
                }
                KeyCode::Right => {
                    self.s3_panel_selected = false;
                }
                KeyCode::Char('?') => {
                    let _ = self.action_tx.send(Action::Navigate { page: ActivePage::Help });
                }
                KeyCode::Char('l') => {
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
}

impl ComponentRender<()> for FileManagerPage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let focus_color = Color::Rgb(98, 114, 164);
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

        let status_line = self.get_status_line();
        let help_line = self.get_help_line();
        let status_line_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(vertical_chunks[1]);
        frame.render_widget(status_line, status_line_layout[0]);
        frame.render_widget(help_line, status_line_layout[1]);

        if self.show_problem_popup {
            let area = Self::centered_rect(60, 20, frame.size());
            frame.render_widget(Clear, area); //this clears out the background
            let block = self.make_transfer_error_popup();
            frame.render_widget(block, area);
        } else if self.show_bucket_input {
            let block = self.make_bucket_name_input();
            let area = Self::centered_rect(40, 20, frame.size());

            frame.render_widget(Clear, area); //this clears out the background
            frame.render_widget(block, area);
            if let Some(error) = self.props.create_bucket_state.clone() {
                let error_paragraph = Paragraph::new(format!("* {:?}", error))
                    .style(Style::default().fg(Color::Red));
                let error_rect = Rect::new(area.x + 1, area.y + 4, area.width, area.height);
                frame.render_widget(Clear, error_rect);
                frame.render_widget(error_paragraph, error_rect);
            }
            frame.set_cursor(
                area.x
                    + self.input.visual_cursor() as u16
                    + 1,
                area.y + 1,
            );
        } else if self.show_delete_confirmation {
            let area = Self::centered_rect(60, 20, frame.size());
            frame.render_widget(Clear, area); //this clears out the background
            let block = self.make_delete_alert("Are you sure you want to delete this object?".to_string(), Color::Green);
            frame.render_widget(block, area);
        } else if self.show_download_confirmation {
            let area = Self::centered_rect(60, 20, frame.size());
            frame.render_widget(Clear, area);
            let block = if self.props.s3_list_recursive_loading {
                self.make_confirm_download_alert("Loading selected object information recursively...".to_string(), Color::Green, false)
            } else {
                self.make_confirm_download_alert(format!("You have selected {} items to download. Proceed?", self.props.s3_data_full_list.len()).to_string(), Color::Green, true)
            };
            frame.render_widget(block, area);
        } else if self.show_delete_error {
            let possible_error = match (self.props.s3_delete_state.clone(), self.props.local_delete_state.clone()) {
                (Some(err), None) => Some(err),
                (None, Some(err)) => Some(err),
                _ => None
            };
            if let Some(err) = possible_error {
                let area = Self::centered_rect(60, 20, frame.size());
                frame.render_widget(Clear, area); //this clears out the background
                let block = self.make_delete_alert(err, Color::Red);
                frame.render_widget(block, area);
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
