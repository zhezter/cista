//! Plaintext UI metadata (favourites, per-entry icons and types) kept
//! separate from the encrypted vault.
//!
//! These fields describe how entries are *presented*, not what they store, so
//! they are cheap to read/write and never need the master password. Keeping
//! them out of the sealed vault means toggling a favourite or swapping an icon
//! is instant (a small JSON write instead of a full argon2 re-seal).

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::EntryType;
use crate::{paths, CoreError, CoreResult};

/// UI metadata for a single opened vault. It is persisted as a small plain
/// JSON file under the XDG state directory, keyed by the vault's path.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UiState {
    #[serde(default)]
    favourites: HashSet<Uuid>,
    #[serde(default)]
    entries: HashMap<Uuid, EntryUi>,
}

/// Per-entry presentation metadata.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EntryUi {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entry_type: Option<EntryType>,
    /// Free-text category label used to group and filter entries. Kept as
    /// presentation metadata alongside favourites/icons, so changing it is an
    /// instant JSON write instead of a full vault re-seal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    group: Option<String>,
}

impl UiState {
    /// An empty UiState with no metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load the UiState for the given vault path, or an empty one if none has
    /// been saved yet (a missing file is not an error).
    pub fn load_for_vault(vault_path: &Path) -> Self {
        let Some(path) = state_file_path(vault_path) else {
            return Self::new();
        };
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => Self::new(),
        }
    }

    /// Persist this UiState for the given vault path, creating the state
    /// directory if needed.
    pub fn save_for_vault(&self, vault_path: &Path) -> CoreResult<()> {
        let Some(path) = state_file_path(vault_path) else {
            return Err(CoreError::Io(std::io::Error::other(
                "could not determine state directory",
            )));
        };
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Remove the on-disk state file for `vault_path` (used when the vault is
    /// deleted). Missing file or directory is not an error.
    pub fn remove_for_vault(vault_path: &Path) {
        if let Some(path) = state_file_path(vault_path) {
            let _ = std::fs::remove_file(path);
        }
    }

    /// Toggle the favourite flag for `id`; returns the new value.
    pub fn toggle_favourite(&mut self, id: Uuid) -> bool {
        if self.favourites.contains(&id) {
            self.favourites.remove(&id);
            false
        } else {
            self.favourites.insert(id);
            true
        }
    }

    /// Whether `id` is a favourite.
    pub fn is_favourite(&self, id: Uuid) -> bool {
        self.favourites.contains(&id)
    }

    /// Set the custom icon for `id` (clears it with `None`).
    pub fn set_icon(&mut self, id: Uuid, icon: Option<String>) {
        let cleaned = icon.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.entry_ui_mut(id).icon = cleaned;
    }

    /// The custom icon for `id`, if any. Falls back to the caller combining
    /// this with [`UiState::entry_type`]'s default.
    pub fn icon(&self, id: Uuid) -> Option<&str> {
        self.entries.get(&id).and_then(|e| e.icon.as_deref())
    }

    /// Set the entry type for `id`.
    pub fn set_entry_type(&mut self, id: Uuid, entry_type: EntryType) {
        self.entry_ui_mut(id).entry_type = Some(entry_type);
    }

    /// The entry type for `id`, defaulting to [`EntryType::Login`].
    pub fn entry_type(&self, id: Uuid) -> EntryType {
        self.entries
            .get(&id)
            .and_then(|e| e.entry_type)
            .unwrap_or_default()
    }

    /// Set the free-text group label for `id` (clears it with `None`).
    pub fn set_group(&mut self, id: Uuid, group: Option<String>) {
        let cleaned = group.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        self.entry_ui_mut(id).group = cleaned;
    }

    /// The group label for `id`, if any.
    pub fn group(&self, id: Uuid) -> Option<&str> {
        self.entries.get(&id).and_then(|e| e.group.as_deref())
    }

    /// Remove all metadata for `id` (used when an entry is deleted).
    pub fn remove_entry(&mut self, id: Uuid) {
        self.favourites.remove(&id);
        self.entries.remove(&id);
    }

    /// The icon to actually display for `id`: its custom icon when set,
    /// otherwise the type's default.
    pub fn display_icon(&self, id: Uuid) -> String {
        self.icon(id)
            .unwrap_or_else(|| self.entry_type(id).default_icon())
            .to_string()
    }

    fn entry_ui_mut(&mut self, id: Uuid) -> &mut EntryUi {
        self.entries.entry(id).or_default()
    }
}

/// Compute the on-disk state file path for a vault, keyed by a stable hash of
/// its canonical path so different vaults never collide on the same filename.
fn state_file_path(vault_path: &Path) -> Option<std::path::PathBuf> {
    let dir = paths::state_dir().ok()?;
    let canonical = std::fs::canonicalize(vault_path).unwrap_or_else(|_| vault_path.to_path_buf());
    let hash = blake2b_hex(canonical.to_string_lossy().as_bytes());
    Some(dir.join(format!("vault-{hash}.json")))
}

/// Hex digest (64 chars) of `input` using BLAKE2b-256.
fn blake2b_hex(input: &[u8]) -> String {
    use blake2::{Blake2b, Digest};
    let mut hasher = Blake2b::<blake2::digest::consts::U32>::new();
    hasher.update(input);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn missing_file_loads_empty() {
        let dir = tmp();
        let state = UiState::load_for_vault(&dir.path().join("nope.cista"));
        assert!(state.favourites.is_empty());
        assert!(state.entries.is_empty());
    }

    #[test]
    fn toggle_favourite_flips_value() {
        let mut s = UiState::new();
        let id = Uuid::new_v4();
        assert!(!s.is_favourite(id));
        assert!(s.toggle_favourite(id));
        assert!(s.is_favourite(id));
        assert!(!s.toggle_favourite(id));
        assert!(!s.is_favourite(id));
    }

    #[test]
    fn icon_type_and_round_trip() {
        let dir = tmp();
        let vault_path = dir.path().join("personal.cista");
        std::fs::write(&vault_path, b"x").ok();

        let id = Uuid::new_v4();
        let mut s = UiState::new();
        s.set_icon(id, Some("  star  ".to_string()));
        s.set_entry_type(id, EntryType::Card);
        s.set_group(id, Some("  Work  ".to_string()));
        s.toggle_favourite(id);
        s.save_for_vault(&vault_path).expect("save");

        let loaded = UiState::load_for_vault(&vault_path);
        assert!(loaded.is_favourite(id));
        assert_eq!(loaded.icon(id), Some("star"));
        assert_eq!(loaded.entry_type(id), EntryType::Card);
        assert_eq!(loaded.group(id), Some("Work"));
        assert_eq!(loaded.display_icon(id), "star");
    }

    #[test]
    fn set_group_none_clears_it() {
        let id = Uuid::new_v4();
        let mut s = UiState::new();
        assert_eq!(s.group(id), None);
        s.set_group(id, Some("work".to_string()));
        assert_eq!(s.group(id), Some("work"));
        s.set_group(id, None);
        assert_eq!(s.group(id), None);
    }

    #[test]
    fn default_type_is_login() {
        let s = UiState::new();
        let id = Uuid::new_v4();
        assert_eq!(s.entry_type(id), EntryType::Login);
        assert_eq!(s.display_icon(id), "🔑");
    }
}
