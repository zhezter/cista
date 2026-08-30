use crate::handlers::resolve_entry_id;
use crate::prompts::InputSource;
use crate::ui;
use cista_core::SecretString;
use cista_core::Vault;
use secrecy::Secret;
use std::path::Path;

pub fn apply_rm(
    vault: &mut Vault,
    path: &Path,
    password: &Secret<SecretString>,
    name: &str,
    input: &mut dyn InputSource,
    yes: bool,
) -> anyhow::Result<()> {
    let id = resolve_entry_id(vault, name, input)?;

    let entry_name = vault
        .find_by_id(id)
        .map(|e| e.name().to_string())
        .unwrap_or_else(|| name.to_string());

    if !yes {
        let confirm = input.read_line(&format!("Delete '{}'? (y/N): ", entry_name))?;
        if !confirm.eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let removed = vault.remove_by_id(id)?;
    vault.save(path, password)?;

    println!(
        "{}",
        ui::error(format!("Removed entry: {}", removed.name()))
    );
    Ok(())
}
