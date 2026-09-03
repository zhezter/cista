use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::widgets::{centered_rect, draw_banner, pills};

pub fn draw_lock_screen(f: &mut Frame, _app: &mut App) {
    let area = centered_rect(62, 55, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    // Banner
    draw_banner(f, chunks[0], Color::Red);

    // Title
    let title = Paragraph::new("🔒 Vault Locked")
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[2]);

    // Message
    let msg =
        Paragraph::new("Session locked due to inactivity.\nPress Enter to unlock or 'q' to quit.")
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);
    f.render_widget(msg, chunks[3]);

    // Actions
    let hint = Paragraph::new(Line::from(pills(
        &[("Enter", "Unlock"), ("q", "Quit")],
        None,
    )))
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center);
    f.render_widget(hint, chunks[4]);
}
