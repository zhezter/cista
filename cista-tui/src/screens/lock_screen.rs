use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::widgets::centered_rect;

pub fn draw_lock_screen(f: &mut Frame, _app: &mut App) {
    let area = centered_rect(50, 30, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    // Title
    let title = Paragraph::new("🔒 Vault Locked")
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Message
    let msg =
        Paragraph::new("Session locked due to inactivity.\nPress Enter to unlock or 'q' to quit.")
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);
    f.render_widget(msg, chunks[1]);

    // Unlock prompt
    let hint = Paragraph::new("Enter Unlock  q Quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(hint, chunks[3]);
}
