//! Background tasks for long-running (KDF-heavy) operations.
//!
//! Unlocking, creating a vault and saving a vault all run Argon2, which can
//! take a second or more and would otherwise freeze the event loop. Those
//! operations are moved to worker threads that report back through a channel;
//! the UI keeps rendering and shows a spinner modal while they run.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

use cista_core::SecretString;
use secrecy::{ExposeSecret, Secret};

use cista_core::storage::{load_vault_from_path, save_new_vault};
use cista_core::Vault;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    Unlock,
    CreateVault,
    SaveEntryAdd,
    SaveEntryEdit,
    SaveEntryDelete,
    DeleteVault,
    VerifyPassword,
    ChangePassword,
    QuickUnlockCheck,
    QuickUnlockVerify,
}

impl TaskKind {
    pub fn label(self) -> &'static str {
        match self {
            TaskKind::Unlock => "Unlocking vault…",
            TaskKind::CreateVault => "Creating vault…",
            TaskKind::SaveEntryAdd | TaskKind::SaveEntryEdit | TaskKind::SaveEntryDelete => {
                "Saving vault…"
            }
            TaskKind::DeleteVault => "Deleting vault…",
            TaskKind::VerifyPassword => "Verifying password…",
            TaskKind::ChangePassword => "Saving vault…",
            TaskKind::QuickUnlockCheck => "Checking fingerprint…",
            TaskKind::QuickUnlockVerify => "Fingerprint…",
        }
    }
}

/// Result delivered by a worker thread, with the owned inputs it needs to hand
/// back (the password secret, the vault, the path/name for status messages).
pub enum TaskResult {
    Unlock {
        path: PathBuf,
        password: Secret<SecretString>,
        result: Result<Vault, String>,
    },
    CreateVault {
        name: String,
        result: Result<(), String>,
    },
    SaveVault {
        result: Result<(), String>,
    },
    /// Vault deletion: verifies the master password against the file, then
    /// removes it. Fails if the password is wrong or the path cannot be
    /// removed.
    DeleteVault {
        result: Result<(), String>,
    },
    /// Verifies the master password against the vault file without opening
    /// it (used to re-confirm the current password before changing it).
    VerifyPassword {
        result: Result<(), String>,
    },
    /// Re-seals the vault with a new master password, handing the new secret
    /// back so the in-memory session can switch to it.
    ChangePassword {
        result: Result<(), String>,
        password: Secret<SecretString>,
    },
    /// Fingerprint availability probe: reports whether a reader is usable and
    /// whether the keyring already holds a secret for this vault.
    QuickUnlockCheck {
        available: bool,
        has_secret: bool,
    },
    /// Fingerprint verification outcome. `password` is `Some` when the verify
    /// command accepted the fingerprint (the stored password may still be
    /// wrong); `result` carries the vault when opening succeeded.
    QuickUnlockVerify {
        path: PathBuf,
        password: Option<Secret<SecretString>>,
        result: Result<Vault, String>,
    },
    /// The thread died before sending anything (only possible on a bug/panic).
    Failed,
}

/// A task in flight, shown as a spinner modal until its result arrives.
pub struct PendingTask {
    pub kind: TaskKind,
    pub started: Instant,
    pub rx: Receiver<TaskResult>,
}

pub fn spawn_unlock(path: PathBuf, password: Secret<SecretString>) -> Receiver<TaskResult> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = load_vault_from_path(&path, password.expose_secret().as_str().as_bytes())
            .map_err(|e| e.to_string());
        let _ = tx.send(TaskResult::Unlock {
            path,
            password,
            result,
        });
    });
    rx
}

pub fn spawn_create_vault(
    name: String,
    path: PathBuf,
    password: Secret<SecretString>,
) -> Receiver<TaskResult> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = save_new_vault(
            &path,
            &Vault::new(),
            password.expose_secret().as_str().as_bytes(),
        )
        .map_err(|e| e.to_string());
        let _ = tx.send(TaskResult::CreateVault { name, result });
    });
    rx
}

pub fn spawn_save_vault(
    path: PathBuf,
    vault: Vault,
    password: Secret<SecretString>,
) -> Receiver<TaskResult> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = vault.save(&path, &password).map_err(|e| e.to_string());
        let _ = tx.send(TaskResult::SaveVault { result });
    });
    rx
}

/// Verifies the master password against the vault file and, only if it is
/// correct, removes the file. Runs on a worker thread because verification,
/// like unlocking, is Argon2-heavy.
pub fn spawn_delete_vault(path: PathBuf, password: Secret<SecretString>) -> Receiver<TaskResult> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = load_vault_from_path(&path, password.expose_secret().as_str().as_bytes())
            .map(|_| std::fs::remove_file(&path))
            .map_err(|e| e.to_string());
        let result = match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e.to_string()),
            Err(e) => Err(e),
        };
        let _ = tx.send(TaskResult::DeleteVault { result });
    });
    rx
}

/// Verifies the master password against the vault file only. Runs on a worker
/// thread because verification is Argon2-heavy, exactly like unlocking.
pub fn spawn_verify_password(
    path: PathBuf,
    password: Secret<SecretString>,
) -> Receiver<TaskResult> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = load_vault_from_path(&path, password.expose_secret().as_str().as_bytes())
            .map(|_| ())
            .map_err(|e| e.to_string());
        let _ = tx.send(TaskResult::VerifyPassword { result });
    });
    rx
}

/// Re-seals the vault file with `password` as the new master password, handing
/// the secret back through the channel so the session can adopt it on success.
pub fn spawn_change_password(
    path: PathBuf,
    vault: Vault,
    password: Secret<SecretString>,
) -> Receiver<TaskResult> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = vault.save(&path, &password).map_err(|e| e.to_string());
        let _ = tx.send(TaskResult::ChangePassword { result, password });
    });
    rx
}

/// Probing whether a fingerprint is usable and whether a secret is already
/// stored for this vault, so the unlock screen only offers the finger when it
/// can actually be used.
pub fn spawn_quick_unlock_check(
    detect_cmd: String,
    account: String,
) -> Receiver<TaskResult> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let available = crate::quick_unlock::run_command_ok(&detect_cmd);
        let has_secret = crate::quick_unlock::load_secret(&account).is_some();
        let _ = tx.send(TaskResult::QuickUnlockCheck {
            available,
            has_secret,
        });
    });
    rx
}

/// Runs the fingerprint verify command; on acceptance, releases the stored
/// master password and opens the vault in the same worker, mirroring
/// [`spawn_unlock`] so the argon2 work never blocks the event loop.
pub fn spawn_quick_unlock_verify(
    path: PathBuf,
    verify_cmd: String,
    account: String,
) -> Receiver<TaskResult> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        if !crate::quick_unlock::run_command_ok(&verify_cmd) {
            let _ = tx.send(TaskResult::QuickUnlockVerify {
                path,
                password: None,
                result: Err("fingerprint not accepted".into()),
            });
            return;
        }
        let Some(password) = crate::quick_unlock::load_secret(&account) else {
            let _ = tx.send(TaskResult::QuickUnlockVerify {
                path,
                password: None,
                result: Err("no stored secret".into()),
            });
            return;
        };
        let result =
            load_vault_from_path(&path, password.expose_secret().as_str().as_bytes())
                .map_err(|e| e.to_string());
        let _ = tx.send(TaskResult::QuickUnlockVerify {
            path,
            password: Some(password),
            result,
        });
    });
    rx
}
