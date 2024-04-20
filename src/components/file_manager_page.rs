use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use crate::model::action::Action;
use crate::components::component::{Component, ComponentRender};
use crate::model::state::{ActivePage, State};
use crate::model::local_data_item::LocalDataItem;
use crate::model::s3_data_item::S3DataItem;

#[derive(Clone)]
struct Props {
    local_table_state: TableState,
    local_data: Vec<LocalDataItem>,
    s3_table_state: TableState,
    s3_data: Vec<S3DataItem>,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        let st = state.clone();
        Props {
            local_table_state: TableState::default(),
            local_data: st.local_data,
            s3_table_state: TableState::default(),
            s3_data: st.s3_data,
        }
    }
}

pub struct FileManagerPage {
    /// Action sender
    pub action_tx: UnboundedSender<Action>,
    /// State Mapped ChatPage Props
    props: Props,
    s3_panel_selected: bool,
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
        }
            .move_with_state(state)
    }

    fn move_with_state(self, state: &State) -> Self
        where
            Self: Sized,
    {
        FileManagerPage {
            props: Props::from(state),
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
            KeyCode::Char('?') => {
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::HelpPage });
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
        let local_table = self.get_local_table(focus_color);
        let s3_table = self.get_s3_table(focus_color);
        frame.render_stateful_widget(&s3_table, chunks[0], &mut self.props.clone().s3_table_state);
        frame.render_stateful_widget(&local_table, chunks[1], &mut self.props.clone().local_table_state);
    }
}

impl FileManagerPage {
    fn get_local_table(&self, focus_color: Color) -> Table {
        let header =
            Row::new(vec!["Name", "Size", "Type"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.local_data.iter().map(|item| Row::new(item.to_columns().clone()));
        let widths = [Constraint::Length(60), Constraint::Length(20), Constraint::Length(20)];
        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Local List").fg(self.get_home_local_color()))
            .highlight_style(Style::default().fg(focus_color).bg(Color::White).add_modifier(Modifier::REVERSED))
            .widths(&[Constraint::Percentage(60), Constraint::Percentage(20), Constraint::Percentage(20)]);
        table
    }

    fn get_s3_table(&self, focus_color: Color) -> Table {
        let header =
            Row::new(vec!["Name", "Size", "Type"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.s3_data.iter().map(|item| Row::new(item.to_columns().clone()));
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
}