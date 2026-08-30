use crate::table::render_entries;
use cista_core::Vault;

pub fn apply_search(vault: &Vault, term: Option<&str>) -> anyhow::Result<()> {
    let query = term.unwrap_or_default();
    let matches = vault.search(query);

    if matches.is_empty() {
        if query.trim().is_empty() {
            println!("No entries stored.");
        } else {
            println!("No entries found for '{}'.", query.trim());
        }
        return Ok(());
    }

    println!("{}", render_entries(matches));

    Ok(())
}
