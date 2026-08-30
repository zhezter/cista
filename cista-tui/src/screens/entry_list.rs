use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
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
        format!(
            "{}  🔓  [/]Search  [a]Add  [g]Generate  [L]Lock",
            vault_name
        )
    };

    let header = Paragraph::new(header_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);

    // Split the main zone into the entries table (left) and a detail pane for
    // the selected entry (right), keepassxc-style.
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(chunks[1]);

    draw_entry_table(f, app, main[0]);
    draw_entry_detail_pane(f, app, main[1]);

    // Footer
    let footer_text = if app.in_search {
        "Type to filter  [Esc] Clear search  [↑/↓] Navigate  [Enter] View"
    } else {
        "[↑/↓] Navigate  [PgUp/PgDn] Page  [/] Search  [a] Add  [g] Generate  [d] Delete  [Enter] View  [c] Copy pass  [L] Lock  [q] Quit  [?] Help"
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[2]);
}

/// Aligned multi-column table of entries (title | username | url).
fn draw_entry_table(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let start = app.entry_list_page * app.per_page;
    let end = (start + app.per_page).min(app.entries.len());
    let page_entries = &app.entries[start..end];

    if app.entries.is_empty() {
        let empty = Paragraph::new(if app.in_search {
            "No entries match your search.\n\nClear the query (Esc) to browse all entries."
        } else {
            "No entries yet.\n\nPress [a] to add your first entry\nor [g] to generate a password."
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

    let header = ratatui::widgets::Row::new(vec!["Title", "User", "URL"]).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<ratatui::widgets::Row> = page_entries
        .iter()
        .map(|e| {
            let user = e.username.as_deref().unwrap_or("-");
            let url = e.url.as_deref().unwrap_or("-");
            ratatui::widgets::Row::new(vec![
                ratatui::widgets::Cell::from(e.name.clone())
                    .style(Style::default().fg(Color::White)),
                ratatui::widgets::Cell::from(user).style(Style::default().fg(Color::DarkGray)),
                ratatui::widgets::Cell::from(url).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let total_pages = app.entries.len().div_ceil(app.per_page);
    let title = format!(
        "Entries ({})  Page {}/{}",
        app.entries.len(),
        app.entry_list_page + 1,
        total_pages.max(1)
    );

    let widths = [
        ratatui::layout::Constraint::Length(24),
        ratatui::layout::Constraint::Length(20),
        ratatui::layout::Constraint::Min(0),
    ];

    let table = ratatui::widgets::Table::new(rows, widths)
        .header(header)
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

    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(app.entry_list_selected.saturating_sub(start)));
    f.render_stateful_widget(table, area, &mut state);
}

/// Detail pane for the currently selected entry.
fn draw_entry_detail_pane(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Entry")
        .border_style(Style::default().fg(Color::Blue));

    let Some(e) = app.entries.get(app.entry_list_selected) else {
        let pane = Paragraph::new("No entry selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(pane, area);
        return;
    };

    let content = vec![
        Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                e.name.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
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
        Line::from(""),
        Line::from(vec![
            Span::styled("[c]", Style::default().fg(Color::Cyan)),
            Span::raw(" Copy password   "),
            Span::styled("[u]", Style::default().fg(Color::Cyan)),
            Span::raw(" Copy user"),
        ]),
        Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
            Span::raw(" Open details"),
        ]),
    ];

    let pane = Paragraph::new(content)
        .style(Style::default().fg(Color::White))
        .block(block);
    f.render_widget(pane, area);
}
