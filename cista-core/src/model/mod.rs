mod entry;
mod secret_string;
mod vault;

pub use entry::Entry;
pub use secret_string::SecretString;
pub use vault::Vault;

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::Secret;

    #[test]
    fn seal_and_open_round_trip() {
        let mut vault = Vault::new();
        let entry = Entry::new(
            "github".to_string(),
            Some("me@example.com".to_string()),
            Secret::new(SecretString::from("hunter2".to_string())),
            Some("https://github.com".to_string()),
            None,
        )
        .expect("entry should be valid");
        vault.add_entry(entry);

        let sealed = vault
            .seal(b"my master password")
            .expect("seal should succeed");
        let opened = Vault::open(&sealed, b"my master password").expect("open should succeed");

        assert_eq!(opened.entries().len(), 1);
        assert_eq!(opened.entries()[0].name(), "github");
    }

    #[test]
    fn open_fails_with_wrong_password() {
        let vault = Vault::new();
        let sealed = vault
            .seal(b"correct password")
            .expect("seal should succeed");

        let result = Vault::open(&sealed, b"wrong password");
        assert!(result.is_err());
    }

    #[test]
    fn opening_a_tampered_file_fails() {
        let mut vault = Vault::new();
        let entry = Entry::new(
            "github".to_string(),
            Some("me@example.com".to_string()),
            Secret::new(SecretString::from("hunter2".to_string())),
            Some("https://github.com".to_string()),
            None,
        )
        .expect("entry should be valid");
        vault.add_entry(entry);

        // Tamper with a byte in the ciphertext region of the sealed data.
        let mut sealed = vault.seal(b"master password").expect("seal should succeed");
        let last = sealed.len() - 1;
        sealed[last] ^= 0xFF;

        let result = Vault::open(&sealed, b"master password");
        assert!(result.is_err());
    }

    #[test]
    fn identical_vaults_produce_different_files() {
        let mut vault = Vault::new();
        let entry = Entry::new(
            "github".to_string(),
            Some("me@example.com".to_string()),
            Secret::new(SecretString::from("hunter2".to_string())),
            Some("https://github.com".to_string()),
            None,
        )
        .expect("entry should be valid");
        vault.add_entry(entry);

        // Sealing the same vault/password twice must yield different bytes
        // thanks to fresh random salt and nonce.
        let sealed_a = vault.seal(b"master password").expect("seal should succeed");
        let sealed_b = vault.seal(b"master password").expect("seal should succeed");

        assert_ne!(sealed_a, sealed_b);
    }

    #[test]
    fn search_matches_across_fields_but_not_password() {
        let mut vault = Vault::new();
        vault.add_entry(
            Entry::new(
                "github".to_string(),
                Some("thaleo@dev.org".to_string()),
                Secret::new(SecretString::from("shh-password-123".to_string())),
                Some("https://github.com".to_string()),
                None,
            )
            .expect("valid entry"),
        );
        vault.add_entry(
            Entry::new(
                "mail".to_string(),
                Some("me@gmail.com".to_string()),
                Secret::new(SecretString::from("itsasecret".to_string())),
                Some("https://mail.google.com".to_string()),
                Some("recovery key stored safely".to_string()),
            )
            .expect("valid entry"),
        );

        assert_eq!(vault.search("thaleo").len(), 1); // by username
        assert_eq!(vault.search("gmail").len(), 1); // by url
        assert_eq!(vault.search("GITHUB").len(), 1); // case-insensitive name
        assert_eq!(vault.search("recovery").len(), 1); // by notes

        // Searching for a password substring must never match.
        assert_eq!(vault.search("shh-password").len(), 0);
        assert_eq!(vault.search("secret").len(), 0); // only in the password "itsasecret"
    }

    #[test]
    fn created_at_defaults_to_epoch_for_old_vaults() {
        // A vault serialized before `created_at` existed must deserialize with
        // the fallback value, so old vaults keep opening.
        let old_json = br#"{"version":1,"entries":[]}"#;
        let vault: Vault = serde_json::from_slice(old_json).expect("old vault deserializes");
        assert_eq!(vault.created_at(), time::OffsetDateTime::UNIX_EPOCH);
    }
}
