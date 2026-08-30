use crate::handlers::resolve_entry_id;
use crate::prompts::InputSource;
use cista_core::SecretString;
use cista_core::Vault;
use secrecy::Secret;
use std::path::Path;

pub fn apply_edit(
    vault: &mut Vault,
    path: &Path,
    password: &Secret<SecretString>,
    name: &str,
    input: &mut dyn InputSource,
) -> anyhow::Result<()> {
    let id = resolve_entry_id(vault, name, input)?;

    println!("Leave a field empty to keep its current value.");

    // Clone current values before we need mutable access to the entry.
    let (current_name, current_username, current_url) = {
        let entry = vault
            .find_by_id(id)
            .ok_or_else(|| anyhow::anyhow!("Entry not found"))?;
        (
            entry.name().to_string(),
            entry.username().unwrap_or("").to_string(),
            entry.url().unwrap_or("").to_string(),
        )
    };

    let new_name = input.read_line(&format!("Service name [{}]: ", current_name))?;
    let new_username = input.read_line(&format!("Username [{}]: ", current_username))?;
    let new_password_raw = input.read_password("New password (leave empty to keep current): ")?;
    let new_password = if new_password_raw.is_empty() {
        None
    } else {
        Some(Secret::new(SecretString::from(new_password_raw)))
    };
    let new_url = input.read_line(&format!("URL [{}]: ", current_url))?;
    let new_notes = input.read_line("Notes (leave empty to keep current): ")?;

    let entry = vault
        .find_by_id_mut(id)
        .ok_or_else(|| anyhow::anyhow!("Entry not found"))?;

    if !new_name.is_empty() {
        entry.rename(new_name)?;
    }
    if !new_username.is_empty() {
        entry.set_username(Some(new_username));
    }
    if let Some(new_password) = new_password {
        entry.set_password(new_password);
    }
    if !new_url.is_empty() {
        entry.set_url(Some(new_url));
    }
    if !new_notes.is_empty() {
        entry.set_notes(Some(new_notes));
    }

    vault.save(path, password)?;

    println!("Entry updated.");
    Ok(())
}
