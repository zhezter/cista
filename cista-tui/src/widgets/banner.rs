//! Unicode block-art wordmark rendered on cover/front screens.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub const CISTA: &str = "░██████  ░██████  ░██████   ░██████████   ░███        \n░██   ░██   ░██   ░██   ░██      ░██      ░██░██      \n░██          ░██  ░██             ░██     ░██  ░██     \n░██          ░██   ░████████      ░██    ░█████████    \n░██          ░██          ░██      ░██    ░██    ░██   \n░██   ░██   ░██   ░██   ░██      ░██    ░██    ░██    \n░██████  ░██████  ░██████       ░██    ░██    ░██ ░██";

/// Draws the CISTA wordmark centered in `area`, tinted with `fg`.
pub fn draw_banner(f: &mut Frame, area: ratatui::layout::Rect, fg: Color) {
    let lines = CISTA
        .lines()
        .map(|l| Line::from(Span::styled(l.to_string(), Style::default().fg(fg))))
        .collect::<Vec<_>>();
    let p = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(p, area);
}
