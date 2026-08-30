//! Error types shared across the `cista-core` crate.

use thiserror::Error;

/// Errors that can occur when working with a vault: creating entries,
/// (de)serializing data, or reading/writing an encrypted `.cista` file.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("entry name cannot be empty")]
    EmptyName,

    #[error("no entry found with that identifier")]
    EntryNotFound,

    #[error("a vault already exists at this path")]
    VaultAlreadyExists,

    #[error("vault file is corrupted or has an unrecognized format")]
    InvalidFormat,

    #[error("unsupported format version: {0}")]
    UnsupportedVersion(u16),

    #[error("failed to unlock vault: wrong password or corrupted file")]
    Unlock,

    #[error("failed to (de)serialize vault data: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid password policy")]
    InvalidPasswordPolicy,

    #[error("password length must be greater than zero")]
    ZeroLength,

    #[error("password policy resulted in an empty character set")]
    EmptyCharset,
}

pub type CoreResult<T> = Result<T, CoreError>;
