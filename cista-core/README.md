# cista-core

Core library for [Cista](https://github.com/zhezter/cista). This crate provides the vault model, the `.cista` file format, the cryptographic primitives, password generation, and health scoring independent of any user interface.

## Features

- **Vault model** — create, open, seal, search and persist encrypted vaults (`Vault`, `Entry`).
- **`.cista` file format** — versioned binary format with a header (signed as AAD) plus ciphertext. Magic `CISTA\0\0\0`, current format version `1`.
- **KDF** — Argon2id by default (64 MiB, 3 iterations, parallelism 1), parameters stored per-vault so a vault always opens with its own parameters.
- **AEAD** — XChaCha20-Poly1305 with the file header authenticated as additional data; 24-byte random nonce.
- **Zeroization** — secrets carried in `secrecy::Secret`/`SecretString` and zeroized on drop.
- **Password generation** — `PasswordPolicy` with length, character-class toggles and ambiguous-character exclusion; guarantees at least one char per active class.
- **Password strength feedback** — offline heuristic checks (common passwords, length, character-class variety).
- **Health scoring** — `health::assess`/`assess_all` produce a 0–100 score from password strength, reuse count and age.
- **Config & paths** — XDG paths (`~/.config/cista`, `~/.local/share/cista`, `~/.local/state/cista`) and a `Config` with auto-lock timeout and default generation length.

## Quick example

```rust
use cista_core::model::{Entry, Vault};
use cista_core::storage;
use cista_core::SecretString;
use secrecy::Secret;

let mut vault = Vault::new();

let entry = Entry::new(
    "github".to_string(),
    Some("octocat".to_string()),
    Secret::new(SecretString::from("hunter2".to_string())),
    Some("https://github.com".to_string()),
    None,
)?;
vault.add_entry(entry);

// Persist (also handles atomic writes + 0600 permissions)
storage::save_new_vault(
    std::path::Path::new("personal.cista"),
    &vault,
    &Secret::new(SecretString::from("master".to_string())),
)?;

// Load and open again
let opened = storage::load_vault_from_path(
    std::path::Path::new("personal.cista"),
    &Secret::new(SecretString::from("master".to_string())),
)?;
assert_eq!(opened.entries().len(), 1);
# Ok::<(), cista_core::CoreError>(())
```

## Search semantics

`Vault::search` matches case-insensitively across name, username, URL and notes — it **never** searches passwords.

## Behavior on Unix

Vault files are written with `0600` permissions on Unix and via an atomic temp-file + `fsync` so a crash never leaves a half-written vault.

## License

MIT
