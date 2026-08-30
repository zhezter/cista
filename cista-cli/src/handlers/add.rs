use crate::clipboard;
use crate::prompts::InputSource;
use crate::ui;
use cista_core::password_gen::{generate_password, PasswordPolicy};
use cista_core::SecretString;
use cista_core::Vault;
use secrecy::{ExposeSecret, Secret};
use std::path::Path;

pub fn apply_add(
    vault: &mut Vault,
    path: &Path,
    password: &Secret<SecretString>,
    input: &mut dyn InputSource,
    generate: bool,
    length: usize,
) -> anyhow::Result<()> {
    let name = input.read_line("Service name: ")?;
    let username = input.prompt_optional_line("Username (optional): ")?;
    let entry_password = if generate {
        // Non-interactive generation: no prompt, no clipboard dependency.
        let generated = generate_password(&PasswordPolicy {
            length,
            ..Default::default()
        })?;
        for warning in cista_core::password_gen::password_feedback(&generated) {
            eprintln!("{}: {}", ui::warn("weak password"), warning);
        }
        Secret::new(SecretString::from(generated))
    } else {
        let use_generator = input.read_line("Generate a random password? (y/N): ")?;
        if use_generator.eq_ignore_ascii_case("y") {
            let generated = generate_password(&Default::default())?;
            match clipboard::copy_secret_to_clipboard(&generated) {
                Ok(()) => println!(
                    "{}",
                    ui::success("Generated password copied to clipboard. Cleared after 15s.")
                ),
                Err(_) => {
                    eprintln!(
                        "{}",
                        ui::warn("WARNING: clipboard unavailable; displaying password.")
                    );
                    println!("Generated password: {generated}");
                }
            }
            Secret::new(SecretString::from(generated))
        } else {
            let entered = Secret::new(SecretString::from(input.read_password("Entry password: ")?));
            for warning in
                cista_core::password_gen::password_feedback(entered.expose_secret().as_str())
            {
                eprintln!("{}: {}", ui::warn("weak password"), warning);
            }
            entered
        }
    };
    let url = input.prompt_optional_line("URL (optional): ")?;
    let notes = input.prompt_optional_line("Notes (optional): ")?;

    let entry = cista_core::Entry::new(name, username, entry_password, url, notes)?;
    vault.add_entry(entry);

    vault.save(path, password)?;

    println!("{}", ui::success("Entry added."));
    Ok(())
}
