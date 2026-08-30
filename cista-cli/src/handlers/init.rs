use crate::prompts::InputSource;
use crate::ui;
use cista_core::SecretString;
use secrecy::{ExposeSecret, Secret};
use std::path::PathBuf;

pub fn handle_init(path: PathBuf, input: &mut dyn InputSource) -> anyhow::Result<()> {
    let password = Secret::new(SecretString::from(input.read_password(
        "Master password: ",
    )?));
    let confirm = Secret::new(SecretString::from(input.read_password(
        "Confirm master password: ",
    )?));

    if password.expose_secret().as_str() != confirm.expose_secret().as_str() {
        anyhow::bail!("Passwords do not match");
    }

    let vault = cista_core::Vault::new();
    cista_core::storage::save_new_vault(
        &path,
        &vault,
        password.expose_secret().as_str().as_bytes(),
    )?;

    println!("{}", ui::success(format!("Vault created at {:?}", path)));
    Ok(())
}
