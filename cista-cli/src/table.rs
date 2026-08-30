//! Shared table rendering for lists of entries.

use comfy_table::Cell;
use comfy_table::ContentArrangement;
use comfy_table::Table;
use cista_core::Entry;

/// Renders `entries` as a table with Name / Username / URL columns.
pub fn render_entries<'a>(entries: impl IntoIterator<Item = &'a Entry>) -> String {
    let mut table = Table::new();
    table
        .set_header(vec!["Name", "Username", "URL"])
        .load_preset(comfy_table::presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    for entry in entries {
        table.add_row(vec![
            Cell::new(entry.name()),
            Cell::new(entry.username().unwrap_or("")),
            Cell::new(entry.url().unwrap_or("")),
        ]);
    }

    table.to_string()
}
