use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{BorderType, Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;

pub fn draw_entry_list(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Header with vault name and search
    let vault_name = app
        .vault_path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|n| n.trim_end_matches(".cista"))
        .unwrap_or("unknown");

    let header_text = if app.in_search {
        format!("🔍 Search: {}_", app.search_query)
    } else {
        format!("{}  🔓  [/]Search  [a]Add  [g]Generate  [L]Lock", vault_name)
    };

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);

    // Entry table/list
    let start = app.entry_list_page * app.per_page;
    let end = (start + app.per_page).min(app.entries.len());
    let page_entries = &app.entries[start..end];

    let items: Vec<ListItem> = page_entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let idx = start + i;
            let selected = idx == app.entry_list_selected;
            let style = if selected {
                Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let user = e.username.as_deref().unwrap_or("-");
            let url = e.url.as_deref().unwrap_or("-");

            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<25}", e.name), style),
                Span::styled(format!(" {:<20}", user), style.fg(Color::DarkGray)),
                Span::styled(format!(" {}", url), style.fg(Color::DarkGray)),
            ]))
        })
        .collect();

    // A friendly placeholder when there is nothing to list, so the empty state
    // isn't mistaken for a broken render.
    if app.entries.is_empty() {
        let empty = Paragraph::new(if app.in_search {
            "No entries match your search.\n\nClear the query (Esc) to browse all entries."
        } else {
            "No entries yet.\n\nPress [a] to add your first entry\nor [g] to generate a password."
        })
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title("Entries (0)"));
        f.render_widget(empty, chunks[1]);
    } else {
        let total_pages = app.entries.len().div_ceil(app.per_page);
        let title = format!(
            "Entries ({})  Page {}/{}",
            app.entries.len(),
            app.entry_list_page + 1,
            total_pages.max(1)
        );
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL).border_type(BorderType::Rounded)
                    .title(title),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");
        let mut state = ListState::default();
        state.select(Some(app.entry_list_selected.saturating_sub(start)));
        f.render_stateful_widget(list, chunks[1], &mut state);
    }

    // Footer
    let footer_text = if app.in_search {
        "Type to filter  Esc Clear search  ↑/↓ Navigate  Enter View"
    } else {
        "↑/↓ or j/k Navigate  PgUp/PgDn Page  Home/End  /Search  a Add  g Generate  d Delete  Enter View  L Lock  q Quit  ? Help"
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[2]);
}