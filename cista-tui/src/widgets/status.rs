use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub fn draw_status(f: &mut Frame, msg: &str, is_error: bool) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let style = if is_error {
        Style::default().fg(Color::Red).bg(Color::Black)
    } else {
        Style::default().fg(Color::Green).bg(Color::Black)
    };

    let status = Paragraph::new(msg)
        .style(style)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(if is_error { " Error " } else { " Status " }),
        );
    f.render_widget(status, chunks[1]);
}
