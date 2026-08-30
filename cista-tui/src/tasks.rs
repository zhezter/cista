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
}

impl TaskKind {
    pub fn label(self) -> &'static str {
        match self {
            TaskKind::Unlock => "Unlocking vault…",
            TaskKind::CreateVault => "Creating vault…",
            TaskKind::SaveEntryAdd | TaskKind::SaveEntryEdit | TaskKind::SaveEntryDelete => {
                "Saving vault…"
            }
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
