use crate::prompts::InputSource;
use crate::ui;
use cista_core::SecretString;
use cista_core::Vault;
use secrecy::{ExposeSecret, Secret};
use std::path::Path;

pub fn apply_passwd(
    vault: &mut Vault,
    path: &Path,
    input: &mut dyn InputSource,
) -> anyhow::Result<Secret<SecretString>> {
    let new_password = Secret::new(SecretString::from(
        input.read_password("New master password: ")?,
    ));
    let confirm = Secret::new(SecretString::from(
        input.read_password("Confirm new master password: ")?,
    ));

    if new_password.expose_secret().as_str() != confirm.expose_secret().as_str() {
        anyhow::bail!("Passwords do not match");
    }

    vault.save(path, &new_password)?;

    println!("{}", ui::success("Master password changed."));
    Ok(new_password)
}
