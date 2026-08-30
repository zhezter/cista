use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{BorderType, Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::{App, GenOption};
use crate::widgets::centered_rect;

pub fn draw_generate(f: &mut Frame, app: &mut App) {
    let area = centered_rect(62, 70, f.area());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3), // generated result
            Constraint::Length(3),
        ])
        .split(area);

    // Header
    let header = Paragraph::new("Generate Password")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);

    // Options list: scroll with ↑/↓ (or j/k), toggle with Space or ←/→.
    let rows: Vec<Row> = GenOption::ALL
        .iter()
        .enumerate()
        .map(|(i, option)| {
            let is_focused = i == app.gen_selected;
            let (label, value) = match option {
                GenOption::Length => (
                    option.label().to_string(),
                    app.gen_policy.length.to_string(),
                ),
                _ => {
                    let on = match option {
                        GenOption::Lowercase => app.gen_policy.include_lowercase,
                        GenOption::Uppercase => app.gen_policy.include_uppercase,
                        GenOption::Digits => app.gen_policy.include_digits,
                        GenOption::Symbols => app.gen_policy.include_symbols,
                        GenOption::ExcludeAmbiguous => app.gen_policy.exclude_ambiguous,
                        GenOption::Length => false,
                    };
                    let state = if on { "[x]" } else { "[ ]" };
                    (option.label().to_string(), state.to_string())
                }
            };

            let base = if is_focused {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let value_style = if is_focused {
                base
            } else if option == &GenOption::Length {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            Row::new(vec![
                Cell::from(label).style(base),
                Cell::from(value).style(value_style),
            ])
            .height(1)
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(30), Constraint::Length(8)])
        .column_spacing(2)
        .block(
            Block::default()
                .borders(Borders::ALL).border_type(BorderType::Rounded)
                .title(" Options ")
                .border_style(Style::default().fg(Color::Blue)),
        );

    let mut state = TableState::default();
    state.select(Some(app.gen_selected));
    f.render_stateful_widget(table, chunks[1], &mut state);

    // Generated result
    if let Some(pwd) = &app.gen_result {
        let result = Paragraph::new(pwd.as_str())
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL).border_type(BorderType::Rounded)
                    .title(" Generated ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        f.render_widget(result, chunks[2]);
    }

    // Footer
    let footer = Paragraph::new(
        "↑/↓ or j/k Move  Space Toggle  ←/→ Change  ↵ Generate  r Reroll  c Copy  Esc Back",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, chunks[3]);
}