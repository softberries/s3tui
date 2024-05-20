use crossterm::event::KeyEvent;
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;
use crate::model::action::Action;
use crate::model::state::State;

/// Represents a parent of every UI component in the application
/// The AppRouter/Component/State structure was inspired by another project
/// `<https://github.com/Yengas/rust-chat-server/>` and Ratatui template application
pub trait Component {
    fn new(state: &State, action_tx: UnboundedSender<Action>) -> Self
        where
            Self: Sized;
    fn move_with_state(self, state: &State) -> Self
        where
            Self: Sized;

    fn name(&self) -> &str;

    fn handle_key_event(&mut self, key: KeyEvent);
}

pub trait ComponentRender<Props> {
    fn render(&self, frame: &mut Frame, props: Props);
}
