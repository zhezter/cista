use crate::cli::FieldSelector;
use crate::clipboard;
use crate::handlers::resolve_entry_id;
use crate::prompts::InputSource;
use crate::ui;
use cista_core::Vault;
use secrecy::ExposeSecret;

pub fn apply_get(
    vault: &Vault,
    name: &str,
    input: &mut dyn InputSource,
    field: Option<FieldSelector>,
) -> anyhow::Result<()> {
    let id = resolve_entry_id(vault, name, input)?;
    let entry = vault
        .find_by_id(id)
        .ok_or_else(|| anyhow::anyhow!("Entry not found"))?;

    // Non-interactive single-field output, e.g. `get foo --field password`.
    if let Some(field) = field {
        match field {
            // The password is a secret: prefer the clipboard (auto-clears) and
            // only fall back to printing when no clipboard/display is available.
            FieldSelector::Password => {
                let value = entry.password().expose_secret().as_str();
                match clipboard::copy_secret_to_clipboard(value) {
                    Ok(()) => println!(
                        "{}",
                        ui::success("Password copied to clipboard. Cleared after 15s.")
                    ),
                    Err(_) => {
                        eprintln!(
                            "{}",
                            ui::warn("WARNING: clipboard unavailable; displaying password.")
                        );
                        println!("{value}");
                    }
                }
            }
            FieldSelector::Username => println!("{}", entry.username().unwrap_or("")),
            FieldSelector::Url => println!("{}", entry.url().unwrap_or("")),
            FieldSelector::Notes => println!(
                "{}",
                entry
                    .notes()
                    .map(|n| n.expose_secret().as_str())
                    .unwrap_or("")
            ),
        };
        return Ok(());
    }

    println!("Name:     {}", entry.name());
    if let Some(username) = entry.username() {
        println!("Username: {username}");
    }
    if let Some(url) = entry.url() {
        println!("URL:      {url}");
    }

    if input.prompt_yes_no(&format!(
        "Copy password for '{}' to clipboard? (y/N): ",
        entry.name()
    ))? {
        clipboard::copy_secret_to_clipboard(entry.password().expose_secret().as_str())?;
        println!("Password copied to clipboard. Cleared after 15s.");
    }

    if let Some(notes) = entry.notes() {
        if input.prompt_yes_no("Show notes on screen? (y/N): ")? {
            println!("Notes:    {}", notes.expose_secret().as_str());
        }
    }

    Ok(())
}
