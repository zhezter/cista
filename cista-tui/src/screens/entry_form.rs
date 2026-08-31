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
    let mode = match app.entry_form.mode {
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
    let masked_pw = mask_password(&app.entry_form.fields.password);
    let masked_cf = mask_password(&app.entry_form.fields.password_confirm);
    let icon_display = if app.entry_form.fields.icon.is_empty() {
        format!("{} (default)", app.entry_form.fields.entry_type.default_icon())
    } else {
        app.entry_form.fields.icon.clone()
    };
    let fields = [
        ("Service name", app.entry_form.fields.name.as_str(), 1, false),
        ("Username", app.entry_form.fields.username.as_str(), 2, false),
        ("Password", masked_pw.as_str(), 3, false),
        ("Confirm password", masked_cf.as_str(), 4, false),
        ("URL", app.entry_form.fields.url.as_str(), 5, false),
        ("Notes", app.entry_form.fields.notes.as_str(), 6, false),
        ("Icon", icon_display.as_str(), 7, false),
        ("Type", app.entry_form.fields.entry_type.label(), 8, true),
    ];

    for (label, value, idx, is_type) in fields {
        let is_active = app.entry_form.field_idx == idx - 1;
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
        // Icon and Type are selectors cycled with Enter rather than text fields.
        let is_selector = is_type || label == "Icon";
        let title = if is_selector && is_active {
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

    // Cursor for the active text field (skip the Icon and Type selectors).
    if let Some((label, value, idx, is_type)) = fields.get(app.entry_form.field_idx) {
        let is_selector = *is_type || *label == "Icon";
        if !is_selector {
            let field_chunk = chunks[*idx];
            f.set_cursor_position((field_chunk.x + 1 + cursor_offset(value), field_chunk.y + 1));
        }
    }
}
