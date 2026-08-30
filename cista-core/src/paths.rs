//! XDG Base Directory resolution for Cista's on-disk locations.

use std::path::PathBuf;

use crate::{CoreError, CoreResult};

/// `~/.config/cista`
pub fn config_dir() -> CoreResult<PathBuf> {
    dirs::config_dir().map(|p| p.join("cista")).ok_or_else(|| {
        CoreError::Io(std::io::Error::other(
            "could not determine config directory",
        ))
    })
}

/// `~/.local/share/cista`
pub fn data_dir() -> CoreResult<PathBuf> {
    dirs::data_dir()
        .map(|p| p.join("cista"))
        .ok_or_else(|| CoreError::Io(std::io::Error::other("could not determine data directory")))
}

/// `~/.local/share/cista/vaults`
pub fn vaults_dir() -> CoreResult<PathBuf> {
    Ok(data_dir()?.join("vaults"))
}

/// `~/.local/share/cista/backups`
pub fn backups_dir() -> CoreResult<PathBuf> {
    Ok(data_dir()?.join("backups"))
}

/// `~/.local/state/cista`
pub fn state_dir() -> CoreResult<PathBuf> {
    dirs::state_dir()
        .map(|p| p.join("cista"))
        .ok_or_else(|| CoreError::Io(std::io::Error::other("could not determine state directory")))
}
