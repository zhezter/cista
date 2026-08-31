//! Vault sealing/unsealing (KDF → AEAD → on-disk format).
//!
//! This is the low-level counterpart of `storage`: `storage` orchestrates
//! reading/writing the file on disk, and this module orchestrates the actual
//! encryption/decryption between the stringified `Vault` JSON and a `CistaFile`.
//!
//! `Vault::seal` / `Vault::open` remain the public entry points and simply
//! delegate here, mirroring how `Vault::save` delegates to `storage`. Keeping
//! the KDF/AEAD/format orchestration in a single place is what makes the
//! persist path consistent instead of split between `Vault` methods and the
//! storage helpers.

use chacha20poly1305::aead::{rand_core::RngCore, OsRng};
use secrecy::Zeroize;

use crate::cipher;
use crate::format::{self, AeadId, CistaFile, KdfId, VaultHeader, SALT_LEN};
use crate::kdf;
use crate::{CoreResult, Vault};

/// Serialize, encrypt and frame `vault` for the given `password`.
pub fn seal(vault: &Vault, password: &[u8]) -> CoreResult<Vec<u8>> {
    let mut plaintext = vault.to_json_bytes()?;

    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let nonce = cipher::generate_nonce();

    let key = kdf::derive_key(
        password,
        &salt,
        kdf::ARGON2_MEMORY_KIB,
        kdf::ARGON2_ITERATIONS,
        kdf::ARGON2_PARALLELISM,
    )?;

    let header = VaultHeader {
        format_version: format::CURRENT_FORMAT_VERSION,
        kdf_id: KdfId::Argon2id,
        kdf_memory_kib: kdf::ARGON2_MEMORY_KIB,
        kdf_iterations: kdf::ARGON2_ITERATIONS,
        kdf_parallelism: kdf::ARGON2_PARALLELISM,
        salt,
        aead_id: AeadId::XChaCha20Poly1305,
        nonce,
    };

    let aad = header.to_bytes();
    let ciphertext = cipher::encrypt(&key, &nonce, &plaintext, &aad)?;

    plaintext.zeroize();

    let file = CistaFile { header, ciphertext };
    Ok(file.to_bytes())
}

/// Parse, decrypt and deserialize a `CistaFile` for the given `password`.
pub fn open(data: &[u8], password: &[u8]) -> CoreResult<Vault> {
    let file = CistaFile::from_bytes(data)?;

    if file.header.format_version != format::CURRENT_FORMAT_VERSION {
        return Err(crate::CoreError::UnsupportedVersion(
            file.header.format_version,
        ));
    }

    let key = kdf::derive_key(
        password,
        &file.header.salt,
        file.header.kdf_memory_kib,
        file.header.kdf_iterations,
        file.header.kdf_parallelism,
    )?;

    let aad = file.header.to_bytes();
    let mut plaintext = cipher::decrypt(&key, &file.header.nonce, &file.ciphertext, &aad)?;

    let vault = Vault::from_json_bytes(&plaintext)?;
    plaintext.zeroize();
    Ok(vault)
}
