//! Fingerprint quick-unlock helpers.
//!
//! The fingerprint never replaces the master password: it only decides whether
//! the vault's password (stored in the OS keyring of this device) is released
//! to the app. fprintd resolves "who owns this device" for us — we never manage
//! prints or users, we just run the configured commands.

use std::path::Path;

use cista_core::SecretString;
use secrecy::{ExposeSecret, Secret};

/// Runs `cmd` with `sh -c` and reports whether it exited 0. An empty command
/// counts as "yes", so a user can skip a step entirely.
pub fn run_command_ok(cmd: &str) -> bool {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return true;
    }
    std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Stable OS-keyring account name for a vault file, derived from its path hash
/// (same key the metadata store uses).
pub fn vault_account(vault_path: &Path) -> Option<String> {
    cista_core::config::vault_key(vault_path).ok()
}

fn entry(account: &str) -> keyring::Result<keyring::Entry> {
    keyring::Entry::new("cista", account)
}

/// Stores the vault's master password in the OS keyring. Only meaningful while
/// quick unlock is enabled; on machines without a Secret Service it fails
/// silently and the feature degrades to a plain password prompt.
pub fn store_secret(account: &str, password: &Secret<SecretString>) -> bool {
    entry(account)
        .and_then(|e| e.set_password(password.expose_secret().as_str()))
        .is_ok()
}

/// Reads the stored master password back, if the OS keyring has it.
pub fn load_secret(account: &str) -> Option<Secret<SecretString>> {
    let pw = entry(account).and_then(|e| e.get_password()).ok()?;
    Some(Secret::new(SecretString::from(pw)))
}

/// Removes the stored master password, e.g. when the vault is deleted.
pub fn delete_secret(account: &str) -> bool {
    entry(account)
        .and_then(|e| e.delete_credential())
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_command_means_yes() {
        assert!(run_command_ok(""));
        assert!(run_command_ok("   "));
    }

    #[test]
    fn exit_zero_counts_as_ok() {
        assert!(run_command_ok("/bin/true"));
        assert!(run_command_ok("true"));
        // A complete pipeline: the fence is the whole command's exit status.
        assert!(run_command_ok("false; true"));
    }

    #[test]
    fn nonzero_or_missing_command_counts_as_no() {
        assert!(!run_command_ok("/bin/false"));
        assert!(!run_command_ok("false"));
        assert!(!run_command_ok("definitely-not-a-real-binary-xyz"));
    }

    #[test]
    fn vault_account_is_deterministic() {
        let a1 = vault_account(std::path::Path::new("/tmp/aaa.cista")).unwrap();
        let a2 = vault_account(std::path::Path::new("/tmp/aaa.cista")).unwrap();
        assert_eq!(a1, a2);
        let b = vault_account(std::path::Path::new("/tmp/bbb.cista")).unwrap();
        assert_ne!(a1, b);
        assert_eq!(a1.len(), 32, "first 16 bytes of the blake2s digest, hex");
    }
}