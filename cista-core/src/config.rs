//! Editable user config and non-sensitive vault metadata.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Inactivity timeout before a REPL session auto-locks, in seconds.
    #[serde(rename = "auto_lock_seconds")]
    pub auto_lock_seconds: u64,
    /// Default password length for `cista generate` and `cista add --generate`.
    #[serde(rename = "default_generate_length")]
    pub default_generate_length: usize,
    /// Device-local fingerprint quick unlock (opt-in).
    #[serde(rename = "quick_unlock")]
    pub quick_unlock: QuickUnlock,
}

/// Opt-in "quick unlock" that lets a fingerprint release the vault's master
/// password, which is kept in the OS keyring of this device.
///
/// This is a convenience layer, not a second factor: the master password is
/// still the real gate. The fingerprint only decides whether the stored copy
/// is handed to the app, and `fprintd` resolves "who owns this device" for us.
/// Both commands are strings run with `sh -c`; an empty string skips that step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QuickUnlock {
    /// Master switch. Everything below is ignored while this is `false`.
    pub enabled: bool,
    /// Command that exits 0 when a fingerprint is usable (reader present and a
    /// print enrolled for the current user). `fprintd-list "$USER"` returns 1
    /// when no device is available, so the affordance is hidden automatically.
    /// Empty string skips detection and always offers the finger.
    pub detect_cmd: String,
    /// Command that verifies the held fingerprint; exit 0 authenticates the
    /// request. Override for development/testing, e.g. a stub that always
    /// exits 0.
    pub verify_cmd: String,
}

impl Default for QuickUnlock {
    fn default() -> Self {
        Self {
            enabled: false,
            detect_cmd: "fprintd-list \"$USER\"".to_string(),
            verify_cmd: "fprintd-verify".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_lock_seconds: 300,
            default_generate_length: 20,
            quick_unlock: QuickUnlock::default(),
        }
    }
}

impl Config {
    /// Loads the user config from disk, or returns the defaults if the file
    /// does not exist or is unreadable.
    pub fn load() -> CoreResult<Config> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Config::default());
        }
        let raw = fs::read_to_string(&path)?;
        toml::from_str(&raw).map_err(|_| CoreError::InvalidFormat)
    }
}

/// `~/.config/cista/config.toml`
pub fn config_path() -> CoreResult<PathBuf> {
    Ok(crate::paths::config_dir()?.join("config.toml"))
}

/// `~/.local/state/cista/meta/<hash>.json`
///
/// The metadata file is keyed by a hash of the vault's absolute path so that
/// two vaults with the same file name in different directories (e.g.
/// `~/a/foo.cista` and `~/b/foo.cista`) do not collide in the meta directory.
/// Stable key for a vault derived from its absolute path's hash, used to key
/// per-vault data without leaking the path itself. Matches the metadata files
/// in `~/.local/state/cista/meta`.
pub fn vault_key(vault_path: &Path) -> CoreResult<String> {
    let abs = fs::canonicalize(vault_path).unwrap_or_else(|_| vault_path.to_path_buf());
    let mut key = String::new();
    {
        use blake2::{Blake2s256, Digest};
        let mut hasher = Blake2s256::new();
        hasher.update(abs.to_string_lossy().as_bytes());
        let digest = hasher.finalize();
        use std::fmt::Write;
        for byte in &digest[..16] {
            let _ = write!(key, "{byte:02x}");
        }
    }
    Ok(key)
}

fn meta_path(vault_path: &Path) -> CoreResult<PathBuf> {
    let key = vault_key(vault_path)?;
    let meta_dir = crate::paths::state_dir()?.join("meta");
    fs::create_dir_all(&meta_dir)?;
    Ok(meta_dir.join(format!("{key}.json")))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VaultMeta {
    pub last_opened: Option<time::OffsetDateTime>,
    /// Number of entries in the vault, updated whenever the vault is saved or
    /// opened. Unknown (`0`) for vaults that have never been opened.
    #[serde(default)]
    pub entry_count: usize,
}

/// Reads the non-sensitive metadata for a vault (e.g. `last_opened`).
pub fn load_meta(vault_path: &Path) -> CoreResult<VaultMeta> {
    let path = meta_path(vault_path)?;
    if !path.exists() {
        return Ok(VaultMeta::default());
    }
    let raw = fs::read_to_string(&path)?;
    serde_json::from_str(&raw).map_err(CoreError::Serialization)
}

/// Records the last time a vault was opened, preserving the rest of the meta.
pub fn record_opened(vault_path: &Path) -> CoreResult<()> {
    let mut meta = load_meta(vault_path)?;
    meta.last_opened = Some(time::OffsetDateTime::now_utc());
    write_meta(vault_path, &meta)
}

/// Records how many entries a vault holds, preserving the rest of the meta.
pub fn set_entry_count(vault_path: &Path, count: usize) -> CoreResult<()> {
    let mut meta = load_meta(vault_path)?;
    meta.entry_count = count;
    write_meta(vault_path, &meta)
}

fn write_meta(vault_path: &Path, meta: &VaultMeta) -> CoreResult<()> {
    let raw = serde_json::to_vec_pretty(meta)?;
    let path = meta_path(vault_path)?;
    fs::write(path, raw)?;
    Ok(())
}
