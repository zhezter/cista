//! Shared table rendering for lists of entries.

use cista_core::Entry;
use comfy_table::Cell;
use comfy_table::ContentArrangement;
use comfy_table::Table;

/// Renders `entries` as a table with Name / Username / URL columns, and a
/// leading Group column when `groups` is provided (one label per entry).
pub fn render_entries<'a>(entries: impl IntoIterator<Item = &'a Entry>, groups: Option<&[&str]>) -> String {
    let mut table = Table::new();
    let mut headers = vec!["Name"];
    if groups.is_some() {
        headers.insert(1, "Group");
    }
    headers.extend_from_slice(&["Username", "URL"]);
    table
        .set_header(headers)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    for (i, entry) in entries.into_iter().enumerate() {
        let mut row = vec![Cell::new(entry.name())];
        if let Some(groups) = groups {
            row.push(Cell::new(groups.get(i).copied().unwrap_or("")));
        }
        row.push(Cell::new(entry.username().unwrap_or("")));
        row.push(Cell::new(entry.url().unwrap_or("")));
        table.add_row(row);
    }

    table.to_string()
}
