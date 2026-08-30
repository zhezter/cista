use crate::Entry;
use crate::{
    cipher,
    format::{self, AeadId, CistaFile, KdfId, VaultHeader, SALT_LEN},
    kdf,
};
use crate::{CoreError, CoreResult, SecretString};
use chacha20poly1305::aead::{rand_core::RngCore, OsRng};
use secrecy::{ExposeSecret, Secret, Zeroize};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct Vault {
    version: u32,
    #[serde(default = "default_created_at")]
    created_at: OffsetDateTime,
    entries: Vec<Entry>,
}

/// Fallback for vaults created before `created_at` existed in the schema.
fn default_created_at() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
}

impl Vault {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_SCHEMA_VERSION,
            created_at: OffsetDateTime::now_utc(),
            entries: Vec::new(),
        }
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn add_entry(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    pub fn find_by_name(&self, name: &str) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|e| e.name().eq_ignore_ascii_case(name))
            .collect()
    }

    /// Case-insensitive substring search across name, username, URL and notes.
    /// Never matches against passwords.
    pub fn search(&self, query: &str) -> Vec<&Entry> {
        use secrecy::ExposeSecret;
        let query = query.trim().to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                if e.name().to_lowercase().contains(&query) {
                    return true;
                }
                if let Some(u) = e.username() {
                    if u.to_lowercase().contains(&query) {
                        return true;
                    }
                }
                if let Some(u) = e.url() {
                    if u.to_lowercase().contains(&query) {
                        return true;
                    }
                }
                if let Some(n) = e.notes() {
                    if n.expose_secret().as_str().to_lowercase().contains(&query) {
                        return true;
                    }
                }
                false
            })
            .collect()
    }

    pub fn find_by_id(&self, id: Uuid) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id() == id)
    }

    pub fn find_by_id_mut(&mut self, id: Uuid) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.id() == id)
    }

    pub fn remove_by_id(&mut self, id: Uuid) -> CoreResult<Entry> {
        let pos = self
            .entries
            .iter()
            .position(|e| e.id() == id)
            .ok_or(CoreError::EntryNotFound)?;
        Ok(self.entries.remove(pos))
    }
}

impl Default for Vault {
    fn default() -> Self {
        Self::new()
    }
}

impl Vault {
    pub fn save(&self, path: &std::path::Path, password: &Secret<SecretString>) -> CoreResult<()> {
        crate::storage::save_vault_to_path(path, self, password.expose_secret().as_str().as_bytes())
    }
}

impl Vault {
    pub fn seal(&self, password: &[u8]) -> CoreResult<Vec<u8>> {
        let mut plaintext = self.to_json_bytes()?;

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
}

impl Vault {
    pub fn open(data: &[u8], password: &[u8]) -> CoreResult<Self> {
        let file = CistaFile::from_bytes(data)?;

        if file.header.format_version != format::CURRENT_FORMAT_VERSION {
            return Err(CoreError::UnsupportedVersion(file.header.format_version));
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
}
