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

/// Like [`pills`], but drops trailing pills that would not fit within
/// `max_width` columns, appending a trailing `…` when anything was omitted.
/// This keeps footers/top bars from ever clipping mid-pill on narrow terms.
pub fn pills_fit<'a>(
    entries: &[(&'a str, &'a str)],
    action_color: Option<Color>,
    max_width: u16,
) -> Vec<Span<'a>> {
    let action_style = Style::default().fg(action_color.unwrap_or(Color::DarkGray));
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    // Treat each `[key] action` pill as atomic; every pill after the first
    // carries a two-column leading gap so a pill is never split across the
    // width boundary.
    let mut pills: Vec<(usize, Vec<Span<'a>>)> = entries
        .iter()
        .map(|(key, action)| {
            let spans = vec![
                Span::styled(format!("[{key}]"), key_style),
                Span::raw(" "),
                Span::styled(*action, action_style),
            ];
            let w = spans.iter().map(|s| s.width()).sum::<usize>() + 2;
            (w, spans)
        })
        .collect();

    let total_width: usize = pills
        .iter()
        .enumerate()
        .map(|(i, (w, _))| if i == 0 { w - 2 } else { *w })
        .sum();
    let mut fits: Vec<Span<'a>> = Vec::new();
    let mut width: usize = 0;
    while !pills.is_empty() {
        let first = width == 0;
        let (w, spans) = pills.remove(0);
        let gap = if first { 0 } else { 2 };
        if width + gap + w - 2 > max_width as usize {
            // Only the leading gap wouldn't fit but the pill body would: drop
            // the gap so we can still show it flush against the previous pill.
            let body = w - 2;
            if width + body <= max_width as usize {
                fits.extend(spans);
                width += body;
                break;
            }
            break;
        }
        if !first {
            fits.push(Span::raw("  "));
        }
        fits.extend(spans);
        width += gap + w - 2;
    }

    // Total width of the fully-rendered pills also counts every leading gap.
    if width < total_width && width + 3 <= max_width as usize {
        fits.push(Span::raw("  "));
        fits.push(Span::styled("…", Style::default().fg(Color::DarkGray)));
    } else if width < total_width && width < max_width as usize {
        fits.push(Span::styled("…", Style::default().fg(Color::DarkGray)));
    }

    fits
}

#[cfg(test)]
mod tests {
    use super::*;

    const SET: &[(&str, &str)] = &[
        ("/", "Search"),
        ("a", "Add"),
        ("Ctrl+g", "Generate"),
        ("f", "Favourite"),
    ];

    fn text(spans: &[Span]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn pills_flattens_key_and_action_into_spans() {
        let spans = pills(SET, None);
        assert_eq!(text(&spans), "[/] Search  [a] Add  [Ctrl+g] Generate  [f] Favourite");
    }

    #[test]
    fn pills_fit_keeps_everything_when_roomy() {
        let spans = pills_fit(SET, Some(Color::LightBlue), 60);
        assert_eq!(
            text(&spans),
            "[/] Search  [a] Add  [Ctrl+g] Generate  [f] Favourite"
        );
        assert!(spans.len() > 4);
    }

    #[test]
    fn pills_fit_drops_trailing_pills_and_adds_ellipsis() {
        let spans = pills_fit(SET, None, 24);
        let out = text(&spans);
        assert!(out.ends_with('…'));
        assert!(out.starts_with("[/] Search"));
        assert!(!out.contains("Favourite"));
    }

    #[test]
    fn pills_fit_adds_bare_ellipsis_when_only_it_fits() {
        let spans = pills_fit(SET, None, 12);
        let out = text(&spans);
        assert_eq!(out, "[/] Search…");
    }

    #[test]
    fn pills_fit_never_exceeds_max_width() {
        for w in 1..40u16 {
            let out = text(&pills_fit(SET, None, w));
            assert!(
                out.chars().count() <= w as usize,
                "w={w}: {out:?} too wide"
            );
        }
    }
}
