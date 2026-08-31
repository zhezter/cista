use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

/// Max width of the floating status notification, in cells.
const MAX_WIDTH: u16 = 64;

/// Renders a transient, floating notification in the bottom-right corner,
/// anchored just above the footer line so it never covers the key hints.
///
/// It reserves no layout space (the caller's content ignores it), so it is
/// safe to draw on top of the current screen.
pub fn draw_status(f: &mut Frame, msg: &str, is_error: bool) {
    let area = f.area();

    // The bordered box needs 2 extra cells for text + 2 border columns.
    let width = (UnicodeWidthStr::width(msg) as u16).min(MAX_WIDTH - 2) + 2;
    let x = area.right().saturating_sub(width + 1);
    let y = area.bottom().saturating_sub(4); // just above the footer zone
    if x < area.left() || y < area.top() {
        return;
    }

    let notify = Rect::new(x, y, width, 3);

    let style = if is_error {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Green)
    };

    let status = Paragraph::new(msg)
        .style(style)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
    f.render_widget(status, notify);
}
