use crate::format::NONCE_LEN;
use crate::{CoreError, CoreResult};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, XChaCha20Poly1305, XNonce,
};

pub fn generate_nonce() -> [u8; NONCE_LEN] {
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    nonce.into()
}

pub fn encrypt(
    key: &[u8; 32],
    nonce: &[u8; NONCE_LEN],
    plaintext: &[u8],
    aad: &[u8],
) -> CoreResult<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let nonce = XNonce::from_slice(nonce);

    cipher
        .encrypt(
            nonce,
            chacha20poly1305::aead::Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| CoreError::Unlock)
}

pub fn decrypt(
    key: &[u8; 32],
    nonce: &[u8; NONCE_LEN],
    ciphertext: &[u8],
    aad: &[u8],
) -> CoreResult<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let nonce = XNonce::from_slice(nonce);

    cipher
        .decrypt(
            nonce,
            chacha20poly1305::aead::Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| CoreError::Unlock)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let key = [7u8; 32];
        let nonce = [3u8; NONCE_LEN];
        let aad = b"some header bytes";
        let plaintext = b"super secret vault contents";

        let ciphertext = encrypt(&key, &nonce, plaintext, aad).expect("encryption should succeed");
        let decrypted = decrypt(&key, &nonce, &ciphertext, aad).expect("decryption should succeed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_fails_with_wrong_key() {
        let key = [7u8; 32];
        let wrong_key = [8u8; 32];
        let nonce = [3u8; NONCE_LEN];
        let aad = b"some header bytes";
        let plaintext = b"super secret vault contents";

        let ciphertext = encrypt(&key, &nonce, plaintext, aad).expect("encryption should succeed");
        let result = decrypt(&wrong_key, &nonce, &ciphertext, aad);

        assert!(result.is_err());
    }

    #[test]
    fn decrypt_fails_if_aad_is_tampered() {
        let key = [7u8; 32];
        let nonce = [3u8; NONCE_LEN];
        let plaintext = b"super secret vault contents";

        let ciphertext = encrypt(&key, &nonce, plaintext, b"original header")
            .expect("encryption should succeed");
        let result = decrypt(&key, &nonce, &ciphertext, b"tampered header!");

        assert!(result.is_err());
    }

    #[test]
    fn decrypt_fails_if_ciphertext_is_tampered() {
        let key = [7u8; 32];
        let nonce = [3u8; NONCE_LEN];
        let aad = b"some header bytes";
        let plaintext = b"super secret vault contents";

        let mut ciphertext =
            encrypt(&key, &nonce, plaintext, aad).expect("encryption should succeed");
        let last = ciphertext.len() - 1;
        ciphertext[last] ^= 0xFF; // flip bits in the last byte

        let result = decrypt(&key, &nonce, &ciphertext, aad);
        assert!(result.is_err());
    }
}
