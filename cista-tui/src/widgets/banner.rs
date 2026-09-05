//! Unicode block-art wordmark rendered on cover/front screens.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub const CISTA: &str = r#"
    █████████  █████  █████████  ███████████   █████████
   ███░░░░░███░░███  ███░░░░░███░█░░░███░░░█  ███░░░░░███
  ███     ░░░  ░███ ░███    ░░░ ░   ░███  ░  ░███    ░███
 ░███          ░███ ░░█████████     ░███     ░███████████
 ░███          ░███  ░░░░░░░░███    ░███     ░███░░░░░███
 ░░███     ███ ░███  ███    ░███    ░███     ░███    ░███
  ░░█████████  █████░░█████████     █████    █████   █████
   ░░░░░░░░░  ░░░░░  ░░░░░░░░░     ░░░░░    ░░░░░   ░░░░░
"#;

/// Draws the CISTA wordmark centered in `area`, tinted with `fg`.
pub fn draw_banner(f: &mut Frame, area: ratatui::layout::Rect, fg: Color) {
    // Trim leading/trailing blank lines so the multi-line raw string above can
    // start and end with a newline for readability without shifting the art.
    let raw: Vec<&str> = CISTA.lines().collect();
    let start = raw.iter().position(|l| !l.trim().is_empty()).unwrap_or(0);
    let end = raw
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map(|i| i + 1)
        .unwrap_or(raw.len());
    let lines = raw[start..end]
        .iter()
        .map(|l| Line::from(Span::styled(l.to_string(), Style::default().fg(fg))))
        .collect::<Vec<_>>();
    let p = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(p, area);
}
