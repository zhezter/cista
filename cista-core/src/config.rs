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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_lock_seconds: 300,
            default_generate_length: 20,
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
fn meta_path(vault_path: &Path) -> CoreResult<PathBuf> {
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
