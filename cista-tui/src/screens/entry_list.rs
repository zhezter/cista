use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::App;
use crate::widgets::{pills_fit, truncate};

pub fn draw_entry_list(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Top bar: vault identity + status, and the quick actions previously
    // crammed into the footer.
    let vault_name = app
        .session.vault_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|n| n.trim_end_matches(".cista"))
        .unwrap_or("unknown");

    let identity_text = if app.entry_list.in_search {
        format!("🔍 Search: {}_", app.entry_list.search_query)
    } else {
        let health = app
            .health_summary()
            .map(|h| format!("  Health: {h}   "))
            .unwrap_or_default();
        let group = app
            .entry_list
            .group_filter
            .as_deref()
            .map(|g| format!("  Group: {g}  "))
            .unwrap_or_default();
        format!(
            "{}  🔓  {}{}[o]Sort: {}  [L]Lock",
            vault_name,
            health,
            group,
            app.entry_list.sort_mode.label()
        )
    };

    let header = Paragraph::new(truncate(&identity_text, chunks[0].width))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);

    // Action bar: the per-row / create commands that used to crowd the footer.
    let action_bar = Paragraph::new(ratatui::text::Line::from(pills_fit(
        &[
            ("/", "Search"),
            ("a", "Add"),
            ("Ctrl+g", "Generate"),
            ("f", "Favourite"),
            ("d", "Delete"),
            ("g", "Group"),
            ("b", "Browse"),
        ],
        Some(Color::LightBlue),
        chunks[1].width,
    )))
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Left)
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(action_bar, chunks[1]);

    // Split the main zone into the entries table (left) and a detail pane for
    // the selected entry (right), keepassxc-style.
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(chunks[2]);

    draw_entry_table(f, app, main[0]);
    draw_entry_detail_pane(f, app, main[1]);

    // Footer (keybinding pills): navigation + view essentials; per-row/create
    // commands live in the action bar at the top.
    let footer_pills = if app.entry_list.in_search {
        pills_fit(
            &[
                ("Esc", "Clear search"),
                ("↑/↓", "Navigate"),
                ("Enter", "View"),
            ],
            None,
            chunks[3].width,
        )
    } else {
        pills_fit(
            &[
                ("↑/↓", "Navigate"),
                ("PgUp/PgDn", "Page"),
                ("Enter", "View"),
                ("c", "Copy pass"),
                ("u", "Copy user"),
                ("q", "Quit"),
                ("?", "Help"),
            ],
            None,
            chunks[3].width,
        )
    };

    let footer = Paragraph::new(ratatui::text::Line::from(footer_pills))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[3]);
}

/// Aligned multi-column table of entries (title | username | modified | url).
fn draw_entry_table(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let start = app.entry_list.page * app.entry_list.per_page;
    let end = (start + app.entry_list.per_page).min(app.entry_list.entries.len());
    let page_entries = &app.entry_list.entries[start..end];

    if app.entry_list.entries.is_empty() {
        let empty = Paragraph::new(if app.entry_list.in_search {
            "No entries match your search.\n\nClear the query (Esc) to browse all entries."
        } else {
            "No entries yet.\n\nPress [a] to add your first entry\nor [Ctrl+g] to generate a password."
        })
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Entries (0)"),
        );
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(vec!["★", "Icon", "Title", "User", "Modified", "URL"]).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let needle = if app.entry_list.in_search {
        app.entry_list.search_query.to_lowercase()
    } else {
        String::new()
    };

    let rows: Vec<Row> = page_entries
        .iter()
        .map(|e| {
            let user = e.username.as_deref().unwrap_or("-");
            let url = e.url.as_deref().unwrap_or("-");
            let modified = format_date(e.updated_at);
            let fav = if e.favorite { "★" } else { "·" };
            let fav_style = if e.favorite {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Row::new(vec![
                Cell::from(fav).style(fav_style),
                Cell::from(e.icon.as_str()).style(Style::default().fg(Color::Cyan)),
                highlight_cell(&e.name, &needle, Color::White),
                highlight_cell(user, &needle, Color::DarkGray),
                Cell::from(modified).style(Style::default().fg(Color::DarkGray)),
                highlight_cell(url, &needle, Color::DarkGray),
            ])
        })
        .collect();

    let total_pages = app.entry_list.entries.len().div_ceil(app.entry_list.per_page);
    let title = format!(
        "Entries ({}) · {}  Page {}/{}",
        app.entry_list.entries.len(),
        app.entry_list.sort_mode.label(),
        app.entry_list.page + 1,
        total_pages.max(1)
    );

    let widths = [
        Constraint::Length(2),
        Constraint::Length(4),
        Constraint::Length(18),
        Constraint::Length(14),
        Constraint::Length(10),
        Constraint::Min(0),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    let mut state = TableState::default();
    state.select(Some(app.entry_list.selected.saturating_sub(start)));
    f.render_stateful_widget(table, area, &mut state);
}

/// Detail pane for the currently selected entry.
fn draw_entry_detail_pane(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Entry")
        .border_style(Style::default().fg(Color::Blue));

    let Some(e) = app.entry_list.entries.get(app.entry_list.selected) else {
        let pane = Paragraph::new("No entry selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(pane, area);
        return;
    };

    let mut content = vec![
        Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                e.name.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            if e.favorite {
                "★ Favourite"
            } else {
                "Not favourite"
            },
            if e.favorite {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        )]),
        Line::from(vec![
            Span::styled("Type: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{} {}", e.icon, e.entry_type.label()),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("Group: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                e.group.as_deref().unwrap_or("-"),
                if e.group.is_some() {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Username: ", Style::default().fg(Color::Yellow)),
            Span::raw(e.username.as_deref().unwrap_or("-")),
        ]),
        Line::from(vec![
            Span::styled("URL: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                e.url.as_deref().unwrap_or("-"),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("Modified: ", Style::default().fg(Color::Yellow)),
            Span::raw(format_date(e.updated_at)),
        ]),
        Line::from(vec![
            Span::styled("Created: ", Style::default().fg(Color::Yellow)),
            Span::raw(format_date(e.created_at)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Health: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{}/100", e.health.score),
                Style::default()
                    .fg(App::health_color(e.health.score))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            format!("  {}", e.health.reason),
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(""),
    ];

    // Notes, wrapped so long text stays inside the pane.
    if let Some(notes) = &e.notes {
        if !notes.is_empty() {
            content.push(Line::from(vec![Span::styled(
                "Notes: ",
                Style::default().fg(Color::Yellow),
            )]));
            for line in notes.lines() {
                content.push(Line::from(Span::styled(
                    line,
                    Style::default().fg(Color::White),
                )));
            }
            content.push(Line::from(""));
        }
    }

    content.push(Line::from(vec![
        Span::styled("[c]", Style::default().fg(Color::Cyan)),
        Span::raw(" Copy password   "),
        Span::styled("[u]", Style::default().fg(Color::Cyan)),
        Span::raw(" Copy user"),
    ]));
    content.push(Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
        Span::raw(" Open details"),
    ]));

    let pane = Paragraph::new(content)
        .wrap(ratatui::widgets::Wrap { trim: false })
        .style(Style::default().fg(Color::White))
        .block(block);
    f.render_widget(pane, area);
}

/// Formats an `OffsetDateTime` as `YYYY-MM-DD`.
fn format_date(t: time::OffsetDateTime) -> String {
    let date = t.date();
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month() as u8,
        date.day()
    )
}

/// Builds a cell that highlights every occurrence of `needle` (case-insensitive)
/// inside `text`. When the needle is empty or absent, the text is plain.
fn highlight_cell<'a>(text: &'a str, needle: &str, base: Color) -> Cell<'a> {
    if needle.is_empty() {
        return Cell::from(text).style(Style::default().fg(base));
    }

    let lower = text.to_lowercase();
    let matches: Vec<(usize, usize)> = {
        let mut spans = Vec::new();
        let mut search_from = 0;
        while let Some(pos) = lower[search_from..].find(needle) {
            let start = search_from + pos;
            let end = start + needle.len();
            spans.push((start, end));
            search_from = end;
        }
        spans
    };

    if matches.is_empty() {
        return Cell::from(text).style(Style::default().fg(base));
    }

    let highlight = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let mut line = Line::default();
    let mut cursor = 0;
    for (start, end) in matches {
        if start > cursor {
            line.push_span(Span::styled(
                &text[cursor..start],
                Style::default().fg(base),
            ));
        }
        line.push_span(Span::styled(&text[start..end], highlight));
        cursor = end;
    }
    if cursor < text.len() {
        line.push_span(Span::styled(&text[cursor..], Style::default().fg(base)));
    }

    Cell::from(line)
}
