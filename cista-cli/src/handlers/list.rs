use crate::table::render_entries;
use cista_core::ui_state::UiState;
use cista_core::Entry;
use cista_core::Vault;
use std::path::Path;

pub fn apply_list(vault: &Vault, path: &Path, group: Option<&str>) -> anyhow::Result<()> {
    if vault.entries().is_empty() {
        println!("No entries stored.");
        return Ok(());
    }

    let ui = UiState::load_for_vault(path);

    let (entries, groups): (Vec<&Entry>, Vec<Option<String>>) = if let Some(g) = group {
        let wanted = g.trim().to_lowercase();
        vault
            .entries()
            .iter()
            .filter(|e| ui.group(e.id()).map(|s| s.to_lowercase() == wanted).unwrap_or(false))
            .map(|e| (e, ui.group(e.id()).map(|s| s.to_string())))
            .collect()
    } else {
        vault
            .entries()
            .iter()
            .map(|e| (e, ui.group(e.id()).map(|s| s.to_string())))
            .collect()
    };

    if entries.is_empty() {
        if let Some(g) = group {
            println!("No entries in group '{g}'.");
        } else {
            println!("No entries stored.");
        }
        return Ok(());
    }

    let group_labels: Vec<&str> = groups.iter().map(|g| g.as_deref().unwrap_or("")).collect();
    println!("{}", render_entries(entries.iter().copied(), Some(&group_labels)));

    Ok(())
}