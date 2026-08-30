use crate::{CoreError, CoreResult};
use argon2::{Algorithm, Argon2, Params, Version};
pub const ARGON2_MEMORY_KIB: u32 = 65_536;
pub const ARGON2_ITERATIONS: u32 = 3;
pub const ARGON2_PARALLELISM: u32 = 1;
pub const KEY_LEN: usize = 32;

pub fn derive_key(
    password: &[u8],
    salt: &[u8],
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
) -> CoreResult<[u8; KEY_LEN]> {
    let params = Params::new(memory_kib, iterations, parallelism, Some(KEY_LEN))
        .map_err(|_| CoreError::InvalidFormat)?; // invalid parameters (eg. absurdly low memory)

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password, salt, &mut key)
        .map_err(|_| CoreError::Unlock)?; // inner fail from Argon2

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_is_deterministic() {
        let salt = [1u8; 16];
        let key1 = derive_key(b"correct horse battery staple", &salt, 19_456, 2, 1)
            .expect("derivation should succeed");
        let key2 = derive_key(b"correct horse battery staple", &salt, 19_456, 2, 1)
            .expect("derivation should succeed");

        assert_eq!(key1, key2);
    }

    #[test]
    fn different_passwords_produce_different_keys() {
        let salt = [1u8; 16];
        let key1 =
            derive_key(b"password one", &salt, 19_456, 2, 1).expect("derivation should succeed");
        let key2 =
            derive_key(b"password two", &salt, 19_456, 2, 1).expect("derivation should succeed");

        assert_ne!(key1, key2);
    }

    #[test]
    fn different_salts_produce_different_keys() {
        let key1 = derive_key(b"same password", &[1u8; 16], 19_456, 2, 1)
            .expect("derivation should succeed");
        let key2 = derive_key(b"same password", &[2u8; 16], 19_456, 2, 1)
            .expect("derivation should succeed");

        assert_ne!(key1, key2);
    }
}
