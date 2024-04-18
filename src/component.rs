use crossterm::event::KeyEvent;
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;
use crate::action::Action;
use crate::state::State;

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
