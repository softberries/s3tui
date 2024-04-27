use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use crate::model::action::Action;
use crate::components::component::{Component, ComponentRender};
use crate::model::s3_selected_item::S3SelectedItem;
use crate::model::state::{ActivePage, State};

#[derive(Clone)]
struct Props {
    table_state: TableState,
    s3_selected_items: Vec<S3SelectedItem>,
}

impl From<&State> for Props {
    fn from(state: &State) -> Self {
        let st = state.clone();
        Props {
            table_state: TableState::default(),
            s3_selected_items: st.s3_selected_items,
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
        TransfersPage {
            props: Props::from(state),
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
            KeyCode::Char('q') => {
                let _ = self.action_tx.send(Action::Exit);
            }
            KeyCode::Char('?') => {
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::HelpPage });
            }
            KeyCode::Esc => {
                let _ = self.action_tx.send(Action::Navigate { page: ActivePage::FileManagerPage });
            }
            _ => {}
        }
    }
}

impl TransfersPage {
    fn get_row(&self, item: &S3SelectedItem) -> Row {
        Row::new(item.to_columns().clone())
    }
}

impl ComponentRender<()> for TransfersPage {
    fn render(&self, frame: &mut Frame, _props: ()) {
        let focus_color = Color::Rgb(98, 114, 164);
        let header =
            Row::new(vec!["Bucket Name", "Resource Path", "Destination"]).fg(focus_color).bold().underlined().height(1).bottom_margin(0);
        let rows = self.props.s3_selected_items.iter().map(|item| TransfersPage::get_row(self, item));
        let widths = [Constraint::Length(60), Constraint::Length(20), Constraint::Length(20)];
        let table = Table::new(rows, widths)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Transfers List").fg(Color::White))
            .highlight_style(Style::default().fg(focus_color).bg(Color::White).add_modifier(Modifier::REVERSED))
            .widths(&[Constraint::Percentage(60), Constraint::Percentage(20), Constraint::Percentage(20)]);
        frame.render_stateful_widget(&table, frame.size(), &mut self.props.clone().table_state);
    }
}