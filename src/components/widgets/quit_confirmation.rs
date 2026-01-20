use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Shared quit confirmation dialog widget
pub struct QuitConfirmation;

impl QuitConfirmation {
    /// Size of the confirmation dialog (width%, height%)
    const DIALOG_SIZE: (u16, u16) = (40, 15);

    /// Handle key events when quit confirmation is showing.
    /// Returns Some(true) if user confirmed quit, Some(false) if cancelled, None if key not handled.
    pub fn handle_key_event(key: KeyEvent) -> Option<bool> {
        if key.kind != KeyEventKind::Press {
            return None;
        }

        match key.code {
            KeyCode::Enter | KeyCode::Char('y') => Some(true),
            KeyCode::Esc | KeyCode::Char('n') => Some(false),
            _ => None,
        }
    }

    /// Render the quit confirmation dialog
    pub fn render(frame: &mut Frame) {
        let area = Self::centered_rect(Self::DIALOG_SIZE.0, Self::DIALOG_SIZE.1, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(Self::make_dialog(), area);
    }

    fn make_dialog() -> Paragraph<'static> {
        Paragraph::new("Are you sure you want to quit?")
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Confirm Quit ")
                    .title_bottom(
                        Line::from(vec![
                            Span::raw(" Press "),
                            Span::styled("Enter/y", Style::default().fg(Color::Green).bold()),
                            Span::raw(" to quit, "),
                            Span::styled("Esc/n", Style::default().fg(Color::Red).bold()),
                            Span::raw(" to cancel "),
                        ])
                        .alignment(Alignment::Center),
                    ),
            )
            .alignment(Alignment::Center)
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
