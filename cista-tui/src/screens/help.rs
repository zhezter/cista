use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};

use crate::app::App;
use crate::widgets::centered_rect;

pub fn draw_help(f: &mut Frame, app: &mut App) {
    let area = centered_rect(62, 90, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new("Help - Keybindings")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);

    let lines = build_help_lines();
    let total = lines.len();
    let viewport = (chunks[1].height.saturating_sub(2).max(1)) as usize;
    let scroll = app.help_scroll as usize;

    // Clamp so the last line never scrolls above the bottom edge.
    let max_scroll = total.saturating_sub(viewport);
    if scroll > max_scroll {
        app.help_scroll = max_scroll as u16;
    }

    let help = Paragraph::new(lines)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .scroll((app.help_scroll, 0))
        .alignment(Alignment::Left);
    f.render_widget(help, chunks[1]);

    // Vertical scrollbar. ratatui's ScrollbarState places the thumb at the very
    // bottom of the track only when `position == content_length - 1`, but the
    // Paragraph reaches its last visible row at `scroll == total - viewport`.
    // We therefore map the scroll to the scrollbar's coordinate space in [0,
    // total - 1] so the thumb touches the bottom exactly when the last line is
    // visible.
    if max_scroll > 0 {
        let scrollbar_pos = if max_scroll <= 1 {
            scroll
        } else {
            (scroll as f64 * (total.saturating_sub(1) as f64) / max_scroll as f64).round() as usize
        }
        .min(total.saturating_sub(1));

        let mut state = ScrollbarState::new(total).position(scrollbar_pos);
        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            chunks[1],
            &mut state,
        );
    }

    let footer = Paragraph::new("↑/↓ Scroll  PgUp/PgDn Page  Home/End Top/Bottom  q or ? Close")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[2]);
}

fn section(title: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        title,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Wraps a plain text row into a `Line` with the given foreground colour.
fn row(text: &'static str) -> Line<'static> {
    Line::from(Span::styled(text, Style::default().fg(Color::White)))
}

/// Single-column reference of every key binding, grouped by screen.
fn build_help_lines() -> Vec<Line<'static>> {
    vec![
        section("Global"),
        row("  q           Quit"),
        row("  ? / Esc     Open / close help"),
        row("  L           Lock vault now"),
        row("  g           Generate password (anywhere)"),
        row("  Esc / Tab   Back / fields inside text inputs"),
        row("  Shift+Tab   Previous field"),
        row("  Ctrl+s      Save form"),
        Line::from(""),
        section("Vault List"),
        row("  ↑/↓ or j/k  Navigate"),
        row("  Enter       Open vault"),
        row("  n           New vault"),
        row("  d           Delete vault"),
        Line::from(""),
        section("Entry List (Dashboard)"),
        row("  ↑/↓ or j/k  Navigate"),
        row("  PgUp/PgDn   Page up/down"),
        row("  Home/End    First/last"),
        row("  /           Search (type to filter)"),
        row("  a           Add entry"),
        row("  d           Delete entry"),
        row("  Enter       View entry"),
        Line::from(""),
        section("Entry Detail"),
        row("  Space       Reveal/hide password"),
        row("  c           Copy password"),
        row("  u           Copy username"),
        row("  l           Copy URL"),
        row("  e           Edit entry"),
        row("  d           Delete entry"),
        row("  Esc         Back to list"),
        Line::from(""),
        section("Forms (Add/Edit/New Vault)"),
        row("  Tab/Shift+Tab Field navigation"),
        row("  Ctrl+s      Save"),
        row("  Esc         Cancel"),
        Line::from(""),
        section("Generate"),
        row("  ↑/↓ or j/k  Move between options"),
        row("  Space       Toggle selected option"),
        row("  ←/→         Adjust length / toggle"),
        row("  r           Reroll (regenerate preview)"),
        row("  c           Copy generated password"),
        row("  Enter       Generate"),
        row("  Esc         Back"),
        Line::from(""),
        section("Confirm Dialog"),
        row("  Enter       Confirm (Yes)"),
        row("  Esc         Cancel (No)"),
    ]
}
