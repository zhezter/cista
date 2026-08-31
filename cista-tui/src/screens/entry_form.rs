use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::{App, FormMode};
use crate::widgets::{centered_rect, cursor_offset, mask_password};

pub fn draw_entry_form(f: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 90, f.area());
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
            Constraint::Length(3), // icon
            Constraint::Length(3), // entry type
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
    let masked_pw = mask_password(&app.form_fields.password);
    let masked_cf = mask_password(&app.form_fields.password_confirm);
    let fields = [
        ("Service name", app.form_fields.name.as_str(), 1, false),
        ("Username", app.form_fields.username.as_str(), 2, false),
        ("Password", masked_pw.as_str(), 3, false),
        ("Confirm password", masked_cf.as_str(), 4, false),
        ("URL", app.form_fields.url.as_str(), 5, false),
        ("Notes", app.form_fields.notes.as_str(), 6, false),
        ("Icon", app.form_fields.icon.as_str(), 7, false),
        ("Type", app.form_fields.entry_type.label(), 8, true),
    ];

    for (label, value, idx, is_type) in fields {
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
        let title = if is_type && is_active {
            format!(" {}  (Enter: cycle) ", label)
        } else {
            format!(" {} ", label)
        };

        let input = Paragraph::new(value.to_string()).style(style).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .border_style(border_style),
        );
        f.render_widget(input, chunks[idx]);
    }

    // Footer
    let footer = Paragraph::new(
        "[Tab] Shift+Tab Next/Prev  [Ctrl+s] Save  [Esc] Back  (secrets are not echoed)",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[10]);

    // Cursor for the active text field (skip the type selector).
    if let Some((_, value, idx, is_type)) = fields.get(app.form_field_idx) {
        if !*is_type {
            let field_chunk = chunks[*idx];
            f.set_cursor_position((field_chunk.x + 1 + cursor_offset(value), field_chunk.y + 1));
        }
    }
}
