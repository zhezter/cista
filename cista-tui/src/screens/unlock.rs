use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::widgets::{centered_rect, cursor_offset};

const WORDMARK: &str = " ####  #####   ####   #####   ### \n\
                        #        #    #         #    #   #\n\
                        #        #     ####     #    #####\n\
                        #        #         #    #    #   #\n\
                         ####  #####   ####     #    #   #";

pub fn draw_unlock(f: &mut Frame, app: &mut App) {
    let area = centered_rect(62, 66, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // wordmark
            Constraint::Length(3), // vault
            Constraint::Length(3), // password
            Constraint::Length(1), // error
            Constraint::Length(2), // hint
            Constraint::Min(0),
        ])
        .split(area);

    // Wordmark, centred as a single block (equal-width lines keep it square).
    let art = Paragraph::new(WORDMARK)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(art, chunks[0]);

    // Vault name
    let vault_name = app
        .vault_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|n| n.trim_end_matches(".cista"))
        .unwrap_or("unknown");
    let vault_text = Paragraph::new(vault_name.to_string())
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Vault "),
        );
    f.render_widget(vault_text, chunks[1]);

    // Password input
    let masked = "•".repeat(app.unlock_password.chars().count());
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
    if let Some(err) = &app.unlock_error {
        let error = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(error, chunks[3]);
    }

    // Hint
    let hint = Paragraph::new("Enter Unlock  Esc Back  (secrets are not echoed)")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(hint, chunks[4]);

    // Cursor position for password input
    f.set_cursor_position((chunks[2].x + 1 + cursor_col, chunks[2].y + 1));
}
