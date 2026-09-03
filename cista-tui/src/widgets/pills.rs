//! Reusable "keybinding pill" building blocks.
//!
//! Renders shortcuts as individual bracketed keys followed by their action,
//! e.g. `[a] Add  [d] Delete`, with the key highlighted. This mirrors the
//! chunky, readable shortcut style used across modern terminal apps instead
//! of packing every command into one long run-on line.

use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

/// Builds a `Line` of keybinding pills for a footer/help bar.
///
/// Each entry becomes `[key] action`, later pills separated by two spaces.
/// `fg` colours the action text; the key is always emboldened cyan. Passing
/// `None` as `fg` uses a dim grey.
pub fn pills<'a>(
    entries: &[(&'a str, &'a str)],
    action_color: Option<Color>,
) -> Vec<Span<'a>> {
    let action_style = Style::default().fg(action_color.unwrap_or(Color::DarkGray));
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let mut spans: Vec<Span<'a>> = Vec::new();
    for (i, (key, action)) in entries.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(format!("[{key}]"), key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*action, action_style));
    }
    spans
}
