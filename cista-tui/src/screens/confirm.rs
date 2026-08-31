use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::{App, ConfirmAction};
use crate::widgets::centered_rect;

pub fn draw_confirm(f: &mut Frame, app: &mut App) {
    let needs_password = app.confirm_on_yes == Some(ConfirmAction::DeleteVault);
    let extra_rows: u16 = if needs_password { 1 } else { 0 };

    let area = centered_rect(50, 25 + extra_rows * 5, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),              // Title
            Constraint::Length(3),              // Message
            Constraint::Length(3 * extra_rows), // Password field
            Constraint::Length(3),              // Buttons
            Constraint::Min(0),
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Confirm")
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Message
    let msg = Paragraph::new(app.confirm_message.as_str())
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
    f.render_widget(msg, chunks[1]);

    // Password field (only when deleting a vault)
    if needs_password {
        let masked: String = app.confirm_password.chars().map(|_| '*').collect();
        let label = if masked.is_empty() {
            "Master password:".to_string()
        } else {
            format!("Master password: {}", masked)
        };
        let pass_par = Paragraph::new(label)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            );
        f.render_widget(pass_par, chunks[2]);
    }

    // Buttons
    let hint = if needs_password {
        "[Enter] Confirm  [Esc] Cancel"
    } else {
        "[Enter] Yes  [Esc] No"
    };
    let buttons = Paragraph::new(hint)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(buttons, chunks[3]);
}
