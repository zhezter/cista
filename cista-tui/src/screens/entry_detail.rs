use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use secrecy::ExposeSecret;

use crate::app::App;
use crate::widgets::{centered_rect, pills};

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
    let idx = app.entry_detail.entry_idx.unwrap_or(0);
    let entry = app.entry_list.entries.get(idx);
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
    if let Some(vault) = &app.session.vault {
        if let Some(entry) = entry.and_then(|e| vault.find_by_id(e.id)) {
            let user = entry.username().unwrap_or("-");
            let url = entry.url().unwrap_or("-");
            let password = entry.password().expose_secret().as_str();
            let visible_pass = if app.entry_detail.show_password {
                password.to_string()
            } else {
                "•".repeat(password.chars().count())
            };
            let notes = entry
                .notes()
                .map(|n| n.expose_secret().as_str())
                .unwrap_or("-");
            let ui = &app.session.ui_state;
            let id = entry.id();
            let icon = ui.display_icon(id);
            let etype = ui.entry_type(id).label();
            let group = ui.group(id).unwrap_or("-");
            let fav = ui.is_favourite(id);
            let created = format_date(entry.created_at());
            let updated = format_date(entry.updated_at());
            let health = app
                .entry_list
                .all_entries
                .iter()
                .find(|r| r.id == id)
                .map(|r| &r.health);

            let mut content = vec![
                Line::from(vec![
                    Span::styled("Type:     ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{icon} {etype}"),
                        Style::default().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Favourite:", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        if fav { "★ yes" } else { "no" },
                        if fav {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Group:    ", Style::default().fg(Color::Yellow)),
                    Span::styled(group, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(""),
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
                Line::from(vec![
                    Span::styled("Created:  ", Style::default().fg(Color::Yellow)),
                    Span::raw(created),
                ]),
                Line::from(vec![
                    Span::styled("Modified: ", Style::default().fg(Color::Yellow)),
                    Span::raw(updated),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Health:   ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        health
                            .map(|h| format!("{}/100", h.score))
                            .unwrap_or_else(|| "-".into()),
                        Style::default()
                            .fg(health.map(|h| App::health_color(h.score)).unwrap_or(Color::White))
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
            ];
            if let Some(h) = health {
                if !h.reason.is_empty() {
                    content.push(Line::from(vec![Span::styled(
                        format!("  {}", h.reason),
                        Style::default().fg(Color::DarkGray),
                    )]));
                }
            }

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

    // Footer (two pill rows so it never overflows the popup width).
    let footer = Paragraph::new(vec![
        ratatui::text::Line::from(pills(
            &[
                ("Space", "Reveal"),
                ("c", "Copy pass"),
                ("u", "Copy user"),
                ("l", "Copy URL"),
            ],
            None,
        )),
        ratatui::text::Line::from(pills(
            &[
                ("b", "Open URL"),
                ("e", "Edit"),
                ("d", "Delete"),
                ("Esc", "Back"),
            ],
            None,
        )),
    ])
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[2]);
}

fn format_date(t: time::OffsetDateTime) -> String {
    let date = t.date();
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month() as u8,
        date.day()
    )
}
