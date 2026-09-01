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

pub fn draw_dashboard(f: &mut Frame, app: &mut App) {
    let area = centered_rect(78, 92, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(f, app, chunks[0]);
    draw_list(f, app, chunks[1]);

    let footer = Paragraph::new(
        "[↑/↓] Navigate  [Enter] View  [Esc] Back to list  [q] Quit",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[2]);
}

fn draw_header(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let total = app.entry_list.all_entries.len();
    let weak = app
        .entry_list
        .all_entries
        .iter()
        .filter(|e| e.health.score < 60)
        .count();
    let reused = app
        .entry_list
        .all_entries
        .iter()
        .filter(|e| e.health.used_in > 1)
        .count();

    let mut lines = vec![Line::from(vec![
        Span::styled(
            "Health Dashboard",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            format!("{total} entries"),
            Style::default().fg(Color::White),
        ),
    ])];

    let badges: Vec<Span> = vec![
        badge("Weak", weak, Color::Red),
        Span::raw("  "),
        badge("Reused", reused, Color::Yellow),
    ];
    lines.push(Line::from(badges));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, area);
}

fn badge(label: &str, count: usize, color: Color) -> Span<'static> {
    Span::raw(format!("{label}: {count}")).style(
        Style::default()
            .fg(color)
            .add_modifier(Modifier::BOLD),
    )
}

fn draw_list(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let rows: Vec<_> = app
        .dashboard
        .rows
        .iter()
        .filter_map(|id| app.entry_list.all_entries.iter().find(|e| e.id == *id))
        .collect();

    let total = rows.len();
    let viewport = area.height.saturating_sub(2).max(1) as usize;

    let max_scroll = total.saturating_sub(viewport);
    if app.dashboard.scroll as usize > max_scroll {
        app.dashboard.scroll = max_scroll as u16;
    }
    if !app.dashboard.rows.is_empty() && app.dashboard.selected >= total {
        app.dashboard.selected = total.saturating_sub(1);
    }

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {:<14} {:<30} {}", "Score", "Entry", "Why"),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    for (i, e) in rows.iter().enumerate() {
        let selected = i == app.dashboard.selected;
        let name = e.name.chars().take(30).collect::<String>();
        let name_w = name.chars().count();
        let pad = " ".repeat(30 - name_w);

        let score_style = if selected {
            Style::default()
                .fg(App::health_color(e.health.score))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(App::health_color(e.health.score))
        };

        let line = Line::from(vec![
            Span::styled(
                format!("  {:<13} ", format!("{}/100", e.health.score)),
                score_style,
            ),
            Span::raw(format!("{name}{pad}")),
            Span::styled(
                e.health.reason.clone(),
                Style::default().fg(Color::Gray),
            ),
        ]);
        if selected {
            lines.push(line.style(Style::default().add_modifier(Modifier::REVERSED)));
        } else {
            lines.push(line);
        }
    }

    let list = Paragraph::new(lines)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Passwords sorted by strength (weakest first)"),
        )
        .scroll((app.dashboard.scroll, 0));

    f.render_widget(list, area);

    if max_scroll > 0 {
        let mut state = ScrollbarState::new(total)
            .position((app.dashboard.scroll as usize).min(total.saturating_sub(1)));        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area,
            &mut state,
        );
    }
}
