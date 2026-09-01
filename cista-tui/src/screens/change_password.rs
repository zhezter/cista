use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::{App, ChangeStage};
use crate::widgets::{centered_rect, cursor_offset};

pub fn draw_change_password(f: &mut Frame, app: &mut App) {
    let area = centered_rect(62, 45, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // password
            Constraint::Length(1), // error
            Constraint::Length(2), // hint
            Constraint::Min(0),
        ])
        .split(area);

    let header = Paragraph::new("Change Master Password")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);

    let (title, masked) = match app.change_password.stage {
        ChangeStage::Current => (
            " Current master password ",
            "•".repeat(app.change_password.current.chars().count()),
        ),
        ChangeStage::New => (
            " New master password ",
            "•".repeat(app.change_password.new.chars().count()),
        ),
        ChangeStage::Confirm => (
            " Confirm new master password ",
            "•".repeat(app.change_password.confirm.chars().count()),
        ),
    };

    let input = Paragraph::new(masked.clone())
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title),
        );
    f.render_widget(input, chunks[1]);

    if let Some(err) = &app.change_password.error {
        let error = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(error, chunks[2]);
    }

    let stage_label = match app.change_password.stage {
        ChangeStage::Current => "Step 1/3 · confirm your identity",
        ChangeStage::New => "Step 2/3 · choose a new master password",
        ChangeStage::Confirm => "Step 3/3 · re-type the new password",
    };
    let hint = Paragraph::new(format!("{stage_label}   [Enter] Continue  [Esc] Cancel"))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(hint, chunks[3]);

    let col = cursor_offset(&masked);
    f.set_cursor_position((chunks[1].x + 1 + col, chunks[1].y + 1));
}