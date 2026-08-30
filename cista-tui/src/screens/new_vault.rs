use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::widgets::{centered_rect, cursor_offset};

pub fn draw_new_vault(f: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 55, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3), // name
            Constraint::Length(3), // password
            Constraint::Length(3), // confirm
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    let header = Paragraph::new("New Vault")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);

    // Fields
    let fields = [
        ("Vault name", &app.new_vault_fields.name, 1),
        (
            "Master password",
            &mask_password(&app.new_vault_fields.password),
            2,
        ),
        (
            "Confirm password",
            &mask_password(&app.new_vault_fields.confirm),
            3,
        ),
    ];

    for (label, value, idx) in fields {
        let is_active = app.new_vault_field_idx == idx - 1;
        let fg = if is_active {
            Color::Yellow
        } else {
            Color::White
        };
        let border = if is_active {
            Color::Blue
        } else {
            Color::DarkGray
        };
        let input = Paragraph::new(value.as_str())
            .style(Style::default().fg(fg).add_modifier(if is_active {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(format!(" {label} "))
                    .title_style(Style::default().fg(fg))
                    .border_style(Style::default().fg(border)),
            );
        f.render_widget(input, chunks[idx]);

        // Cursor for the active field — placed at the end of the *visible*
        // text. `value` here is already the masked password, and
        // `cursor_offset` counts wide glyphs (like the multi-byte bullet) as a
        // single terminal cell, keeping the column right.
        if is_active {
            f.set_cursor_position((chunks[idx].x + 1 + cursor_offset(value), chunks[idx].y + 1));
        }
    }

    // Footer
    let footer = Paragraph::new(
        "Tab Next  Shift+Tab Prev  Ctrl+s Create  Esc Cancel  (secrets are not echoed)",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[5]);
}

fn mask_password(pwd: &str) -> String {
    "•".repeat(pwd.chars().count())
}
