pub mod status;
pub mod pills;
pub mod banner;

pub use status::draw_status;
pub use pills::{pills, pills_fit};
pub use banner::draw_banner;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use unicode_width::UnicodeWidthStr;

/// Number of terminal cells `value` occupies. Unlike `chars().count()`, this
/// counts wide characters (emoji, CJK) as two cells, so the cursor stays
/// aligned with the rendered text.
pub fn cursor_offset(value: &str) -> u16 {
    UnicodeWidthStr::width(value) as u16
}

/// Masks a password with bullet characters (one per Unicode character).
pub fn mask_password(pwd: &str) -> String {
    "•".repeat(pwd.chars().count())
}

/// Truncates `s` to at most `max_width` terminal cells (wide characters count
/// as two), replacing the cut tail with `…`. The result never exceeds
/// `max_width` cells. Used to keep long names from overflowing their pane.
pub fn truncate(s: &str, max_width: u16) -> String {
    let max = max_width as usize;
    if UnicodeWidthStr::width(s) <= max {
        return s.to_string();
    }
    // Reserve one cell for the ellipsis, so the total stays within `max`.
    let budget = max.saturating_sub(1);
    let mut out = String::new();
    let mut width = 0usize;
    for c in s.chars() {
        let w = UnicodeWidthStr::width(c.to_string().as_str());
        if width + w > budget {
            break;
        }
        out.push(c);
        width += w;
    }
    format!("{out}…")
}

/// Renders a byte size as `B`, `KB` or `MB`.
pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let value = bytes as f64;
    if value >= MB {
        format!("{:.1} MB", value / MB)
    } else if value >= KB {
        format!("{:.1} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

/// Splits `r` into a centered box occupying `percent_x`% of the width and
/// `percent_y`% of the height.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_short_strings() {
        assert_eq!(truncate("abc", 5), "abc");
        assert_eq!(truncate("ab", 2), "ab");
    }

    #[test]
    fn truncate_clips_by_cell_width_and_adds_ellipsis() {
        assert_eq!(truncate("abcdef", 3), "ab…");
        assert_eq!(truncate("abcdef", 4), "abc…");
    }

    #[test]
    fn truncate_counts_wide_chars_as_two_cells() {
        // 'é' is width 1; '你' is width 2 -> clipped after 2 cells.
        assert_eq!(truncate("你abc", 3), "你…");
    }
}
