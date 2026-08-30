use crate::prompts::InputSource;
use cista_core::model::SecretString;
use cista_core::Vault;
use secrecy::{ExposeSecret, Secret};
use std::path::Path;

/// Reads the master password from `input` and unlocks the vault at `path`.
pub fn unlock_vault(
    path: &Path,
    input: &mut dyn InputSource,
) -> anyhow::Result<(Vault, Secret<SecretString>)> {
    let raw_password = input.read_password("Master password: ")?;
    unlock_raw(path, &raw_password)
}

/// Unlocks the vault at `path` with a password already read from the user.
pub fn unlock_raw(
    path: &Path,
    raw_password: &str,
) -> anyhow::Result<(Vault, Secret<SecretString>)> {
    let password = Secret::new(SecretString::from(raw_password.to_string()));

    let vault = cista_core::storage::load_vault_from_path(
        path,
        password.expose_secret().as_str().as_bytes(),
    )?;

    // Non-sensitive last-opened metadata; failures must not block unlocking.
    let _ = cista_core::config::record_opened(path);

    Ok((vault, password))
}
