use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::{App, FormMode};
use crate::widgets::{centered_rect, cursor_offset};

pub fn draw_entry_form(f: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 80, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3), // name
            Constraint::Length(3), // username
            Constraint::Length(3), // password
            Constraint::Length(3), // confirm password
            Constraint::Length(3), // url
            Constraint::Length(3), // notes
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    let mode = match app.form_mode {
        FormMode::Add => "Add Entry",
        FormMode::Edit => "Edit Entry",
    };
    let header = Paragraph::new(mode)
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
        ("Service name", &app.form_fields.name, 1),
        ("Username", &app.form_fields.username, 2),
        ("Password", &mask_password(&app.form_fields.password), 3),
        (
            "Confirm password",
            &mask_password(&app.form_fields.password_confirm),
            4,
        ),
        ("URL", &app.form_fields.url, 5),
        ("Notes", &app.form_fields.notes, 6),
    ];

    for (label, value, idx) in fields {
        let is_active = app.form_field_idx == idx - 1;
        let style = if is_active {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let border_style = if is_active {
            Style::default().fg(Color::Blue)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let input = Paragraph::new(value.as_str()).style(style).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(" {} ", label))
                .border_style(border_style),
        );
        f.render_widget(input, chunks[idx]);
    }

    // Footer
    let footer = Paragraph::new(
        "Tab/Shift+Tab Next/Prev field  Ctrl+s Save  Esc Back  (secrets are not echoed)",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[8]);

    // Cursor for the active field.
    if let Some((_, value, idx)) = fields.get(app.form_field_idx) {
        let field_chunk = chunks[*idx];
        f.set_cursor_position((field_chunk.x + 1 + cursor_offset(value), field_chunk.y + 1));
    }
}

fn mask_password(pwd: &str) -> String {
    "•".repeat(pwd.chars().count())
}
