//! Core library for Cista: vault model, file format and crypto.

mod error;

pub mod cipher;
pub mod config;
pub mod format;
pub mod kdf;
pub mod model;
pub mod password_gen;
pub mod paths;
pub mod storage;

mod serialization;

pub use error::{CoreError, CoreResult};
pub use model::{Entry, SecretString, Vault};
