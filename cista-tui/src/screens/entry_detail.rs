use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use secrecy::ExposeSecret;

use crate::app::App;
use crate::widgets::centered_rect;

pub fn draw_entry_detail(f: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 70, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    let idx = app.detail_entry_idx.unwrap_or(0);
    let entry = app.entries.get(idx);
    let name = entry.map(|e| e.name.as_str()).unwrap_or("Unknown");

    let header = Paragraph::new(format!("Entry: {}", name))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);

    // Detail content
    if let Some(vault) = &app.vault {
        if let Some(entry) = entry.and_then(|e| vault.find_by_id(e.id)) {
            let user = entry.username().unwrap_or("-");
            let url = entry.url().unwrap_or("-");
            let password = entry.password().expose_secret().as_str();
            let visible_pass = if app.show_password {
                password.to_string()
            } else {
                "•".repeat(password.chars().count())
            };
            let notes = entry
                .notes()
                .map(|n| n.expose_secret().as_str())
                .unwrap_or("-");

            let content = vec![
                Line::from(vec![
                    Span::styled("Username: ", Style::default().fg(Color::Yellow)),
                    Span::raw(user),
                ]),
                Line::from(vec![
                    Span::styled("URL:      ", Style::default().fg(Color::Yellow)),
                    Span::raw(url),
                ]),
                Line::from(vec![
                    Span::styled("Password: ", Style::default().fg(Color::Yellow)),
                    Span::raw(visible_pass),
                ]),
                Line::from(vec![
                    Span::styled("Notes:    ", Style::default().fg(Color::Yellow)),
                    Span::raw(notes),
                ]),
            ];

            let detail = Paragraph::new(content)
                .style(Style::default().fg(Color::White))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title("Details"),
                )
                .alignment(Alignment::Left);
            f.render_widget(detail, chunks[1]);
        }
    }

    // Footer
    let footer = Paragraph::new(
        "Space Reveal  c Copy pass  u Copy user  l Copy URL  e Edit  d Delete  Esc Back",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[2]);
}
