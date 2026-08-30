use crate::clipboard;
use cista_core::password_gen::{generate_password, PasswordPolicy};

pub fn handle_generate(
    length: usize,
    no_symbols: bool,
    exclude_ambiguous: bool,
) -> anyhow::Result<()> {
    let policy = PasswordPolicy {
        length,
        include_symbols: !no_symbols,
        exclude_ambiguous,
        ..Default::default()
    };

    let password = generate_password(&policy)?;

    match clipboard::copy_secret_to_clipboard(&password) {
        Ok(()) => println!("Password copied to clipboard. Cleared after 15s."),
        Err(_) => {
            eprintln!("WARNING: clipboard unavailable; displaying password.");
            println!("{password}");
        }
    }

    Ok(())
}
