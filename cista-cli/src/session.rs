//! Mutable session state for the interactive REPL over an open vault.

use crate::prompts::InputSource;
use cista_core::SecretString;
use cista_core::Vault;
use secrecy::Secret;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Holds the state of an interactive vault session: the open (or locked)
/// vault, its master password, and auto-lock bookkeeping.
pub struct Session {
    pub path: PathBuf,
    pub vault: Option<Vault>,
    pub password: Option<Secret<SecretString>>,
    pub locked: bool,
    last_activity: Instant,
    auto_lock_seconds: u64,
}

impl Session {
    pub fn new(
        path: PathBuf,
        vault: Vault,
        password: Secret<SecretString>,
        auto_lock_seconds: u64,
    ) -> Self {
        Self {
            path,
            vault: Some(vault),
            password: Some(password),
            locked: false,
            last_activity: Instant::now(),
            auto_lock_seconds,
        }
    }

    pub fn lock(&mut self) {
        // Dropping the vault and password zeroizes the secrets via Secret::drop.
        self.vault = None;
        self.password = None;
        self.locked = true;
    }

    pub fn unlock(&mut self, input: &mut dyn InputSource) -> anyhow::Result<()> {
        let raw = input.read_password("Master password: ")?;
        let (vault, password) = crate::vault_session::unlock_raw(&self.path, &raw)?;
        self.vault = Some(vault);
        self.password = Some(password);
        self.locked = false;
        self.touch();
        Ok(())
    }

    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn is_expired(&self) -> bool {
        !self.locked
            && self.auto_lock_seconds != 0
            && self.last_activity.elapsed() >= Duration::from_secs(self.auto_lock_seconds)
    }

    pub fn vault(&self) -> anyhow::Result<&Vault> {
        self.vault
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("vault is locked"))
    }
}
