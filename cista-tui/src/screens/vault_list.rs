use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::time::SystemTime;

use crate::app::App;
use crate::widgets::human_size;

pub fn draw_vault_list(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new("CISTA - Vault Selector")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    // Split the main zone into the vault list (left) and a detail pane for the
    // currently selected vault (right), so the empty space carries useful info
    // about what's under the cursor.
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    draw_vault_table(f, app, main[0]);
    draw_vault_detail(f, app, main[1]);

    // Footer
    let footer = Paragraph::new(
        "[↑/↓] Navigate  [Enter] Open  [n] New  [g] Generate  [d] Delete  [q] Quit  [?] Help",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[2]);
}

/// Dense multi-column list of vaults (name | entries | last access).
fn draw_vault_table(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Header row with column labels, kept as the first (non-selectable) item.
    let header = Line::from(vec![
        Span::styled(
            format!("{:<20}", "Name"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:>8}", "Entries"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {:<12}", "Last"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let mut rows: Vec<ListItem> = Vec::with_capacity(app.vaults.len() + 1);
    rows.push(ListItem::new(header));

    for v in &app.vaults {
        let count = v
            .entry_count
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".into());
        let last = v.last_opened.as_deref().unwrap_or("never");
        rows.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{:<20}", v.name), Style::default().fg(Color::White)),
            Span::styled(
                format!("{:>8}", count),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("  {:<12}", last),
                Style::default().fg(Color::DarkGray),
            ),
        ])));
    }

    let list = List::new(rows)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Vaults"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    let mut state = ListState::default();
    state.select(Some(app.vault_list_selected.saturating_add(1)));
    f.render_stateful_widget(list, area, &mut state);
}

/// Detail pane for the selected vault.
fn draw_vault_detail(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let selected = app.vaults.get(app.vault_list_selected);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Selected")
        .border_style(Style::default().fg(Color::Blue));

    if let Some(vault) = selected {
        let created = vault
            .created
            .map(format_time)
            .unwrap_or_else(|| "unknown".into());
        let content = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    vault.name.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Entries: ", Style::default().fg(Color::Yellow)),
                Span::raw(
                    vault
                        .entry_count
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "?".into()),
                ),
            ]),
            Line::from(vec![
                Span::styled("Size: ", Style::default().fg(Color::Yellow)),
                Span::raw(human_size(vault.size)),
            ]),
            Line::from(vec![
                Span::styled("Created: ", Style::default().fg(Color::Yellow)),
                Span::raw(created),
            ]),
            Line::from(vec![
                Span::styled("Last opened: ", Style::default().fg(Color::Yellow)),
                Span::raw(vault.last_opened.as_deref().unwrap_or("never")),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Path: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    vault.path.display().to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        ];
        let detail = Paragraph::new(content)
            .style(Style::default().fg(Color::White))
            .block(block);
        f.render_widget(detail, area);
    } else {
        let detail = Paragraph::new("No vault selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(detail, area);
    }
}

/// Formats a `SystemTime` as `YYYY-MM-DD` using the local timezone.
fn format_time(t: SystemTime) -> String {
    let dt = time::OffsetDateTime::from(t);
    dt.format(&time::format_description::well_known::Rfc3339)
        .map(|s| s[..10].to_string())
        .unwrap_or_else(|_| "unknown".into())
}
