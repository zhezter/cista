use crate::table::render_entries;
use cista_core::Vault;

pub fn apply_list(vault: &Vault) -> anyhow::Result<()> {
    if vault.entries().is_empty() {
        println!("No entries stored.");
        return Ok(());
    }

    println!("{}", render_entries(vault.entries()));

    Ok(())
}
