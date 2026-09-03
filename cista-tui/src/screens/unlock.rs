use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::{App, UnlockMode};
use crate::widgets::{centered_rect, cursor_offset, draw_banner, pills};

pub fn draw_unlock(f: &mut Frame, app: &mut App) {
    let area = centered_rect(62, 66, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // banner
            Constraint::Length(3), // vault
            Constraint::Length(3), // password
            Constraint::Length(1), // error
            Constraint::Length(1), // hint
            Constraint::Min(0),
        ])
        .split(area);

    // Wordmark, tinted cyan.
    draw_banner(f, chunks[0], Color::Cyan);

    let deleting = app.unlock.mode == UnlockMode::Delete;

    // Vault name
    let vault_name = app
        .session.vault_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|n| n.trim_end_matches(".cista"))
        .unwrap_or("unknown");
    let vault_title = if deleting {
        format!(" Delete vault '{}'? ", vault_name)
    } else {
        " Vault ".to_string()
    };
    let vault_text = Paragraph::new(vault_name.to_string())
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(vault_title),
        );
    f.render_widget(vault_text, chunks[1]);

    // Password input
    let masked = "•".repeat(app.unlock.password.chars().count());
    let cursor_col = cursor_offset(&masked);
    let password = Paragraph::new(masked)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Master Password "),
        );
    f.render_widget(password, chunks[2]);

    // Error
    if let Some(err) = &app.unlock.error {
        let error = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(error, chunks[3]);
    }

    // Hint (keybinding pills)
    let mut hint_spans = if deleting {
        pills(
            &[
                ("Enter", "Delete vault"),
                ("Esc", "Cancel"),
            ],
            None,
        )
    } else {
        pills(&[("Enter", "Unlock"), ("Esc", "Back")], None)
    };
    hint_spans.push(Span::raw("  (secrets are not echoed)"));
    if !deleting
        && app.quick_unlock.enabled
        && app.quick_unlock.checked
        && app.quick_unlock.available
        && app.quick_unlock.has_secret
    {
        hint_spans.push(Span::raw("   "));
        hint_spans.push(Span::styled("[Ctrl+F]", Style::default().fg(Color::Cyan)));
        hint_spans.push(Span::raw(" Fingerprint unlock"));
    }
    let hint = Paragraph::new(Line::from(hint_spans))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(hint, chunks[4]);

    // Cursor position for password input
    f.set_cursor_position((chunks[2].x + 1 + cursor_col, chunks[2].y + 1));
}
