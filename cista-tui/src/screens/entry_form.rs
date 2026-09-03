use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::{App, FormMode};
use crate::widgets::{centered_rect, cursor_offset, mask_password, pills};

pub fn draw_entry_form(f: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 80, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
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
        ("Service name", app.entry_form.fields.name.as_str(), false),
        ("Username", app.entry_form.fields.username.as_str(), false),
        ("Password", masked_pw.as_str(), false),
        ("Confirm password", masked_cf.as_str(), false),
        ("URL", app.entry_form.fields.url.as_str(), false),
        ("Notes", app.entry_form.fields.notes.as_str(), false),
        ("Icon", icon_display.as_str(), true),
        ("Type", app.entry_form.fields.entry_type.label(), true),
    ];

    // The eight fields may exceed the available height on short terminals, so
    // scroll vertically keeping the active field visible.
    let field_area = chunks[1];
    let field_rows: u16 = fields.len() as u16 * 3;
    let max_visible = field_area.height.saturating_sub(1) / 3;
    let active = app.entry_form.field_idx as u16;
    let scroll = if field_rows > field_area.height - 1 && active >= max_visible {
        active - max_visible + 1
    } else {
        0
    };
    let top_y = field_area.y.saturating_sub(scroll * 3);
    let bottom = field_area.y + field_area.height;

    for (idx, (label, value, is_selector)) in fields.iter().enumerate() {
        let field_top = top_y + (idx as u16) * 3;
        if field_top + 3 <= field_area.y || field_top >= bottom {
            continue;
        }
        let chunk = ratatui::layout::Rect {
            x: field_area.x,
            y: field_top,
            width: field_area.width,
            height: 3,
        };
        // Clip the field to the modal's field area so a partially visible row
        // never spills over the header or footer above/below.
        let chunk = chunk.intersection(field_area);
        if chunk.width == 0 || chunk.height == 0 {
            continue;
        }
        let is_active = app.entry_form.field_idx == idx;
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
        let title = if *is_selector && is_active {
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
        f.render_widget(input, chunk);

        // Cursor for the active text field (skip the Icon and Type selectors);
        // only when the whole field is visible so the caret never lands on a
        // clipped edge.
        if is_active && !is_selector && field_top >= field_area.y && field_top + 3 <= bottom {
            f.set_cursor_position((chunk.x + 1 + cursor_offset(value), chunk.y + 1));
        }
    }

    // Footer
    let mut footer_spans = pills(
        &[
            ("Tab", "Next"),
            ("Shift+Tab", "Prev"),
            ("Ctrl+s", "Save"),
            ("Esc", "Back"),
        ],
        None,
    );
    footer_spans.push(ratatui::text::Span::raw("  (secrets are not echoed)"));
    let footer = Paragraph::new(ratatui::text::Line::from(footer_spans))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[2]);
}
