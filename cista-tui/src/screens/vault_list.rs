use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

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

    // Vault list
    let items: Vec<ListItem> = app
        .vaults
        .iter()
        .map(|v| {
            let last = v.last_opened.as_deref().unwrap_or("never");
            let count = v
                .entry_count
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".into());
            let size = human_size(v.size);
            ListItem::new(Line::from(vec![
                Span::styled(v.name.clone(), Style::default().fg(Color::White)),
                Span::styled(
                    format!("  ({count} entries, {size}, last: {last})"),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
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
    state.select(Some(app.vault_list_selected));
    f.render_stateful_widget(list, chunks[1], &mut state);

    // Footer
    let footer = Paragraph::new(
        "↑/↓ or j/k Navigate  Enter Open  n New  g Generate  d Delete  q Quit  ? Help",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[2]);
}
