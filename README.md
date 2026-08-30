# Cista

A local, encrypted password manager written in Rust. Educational project with real cryptographic practices — not toy implementations.

Your credentials live in a single encrypted file (`*.cista`). Without the master password, it's just random bytes.

## Commands

| Command | Description |
|---------|-------------|
| `cista init [path]` | Create a new vault. Bare name → `~/.local/share/cista/vaults/name.cista` |
| `cista add <path>` | Add entry. `--generate [--length N]` creates password non-interactively |
| `cista get <path> <name>` | Show entry. `--field <password\|username\|url\|notes>` prints single field |
| `cista list <path>` | List all entries (table) |
| `cista search <path> [term]` | Substring search across name/username/url/notes |
| `cista edit <path> <name>` | Edit entry fields interactively |
| `cista rm <path> <name>` | Remove entry. `--yes` skips confirmation |
| `cista passwd <path>` | Change master password |
| `cista generate [--length N] [--no-symbols] [--exclude-ambiguous]` | Generate random password |
| `cista list-vaults` | List vaults in default directory |
| `cista open <vault>` | Start interactive REPL session |

**Global flag:** `--password-stdin` — read master password from stdin (for scripting)

## Quick Examples

```bash
# Create vault
cista init personal

# Add entry with generated password
cista add personal --generate --length 16

# Get password to clipboard (scripting)
echo "$MASTER" | cista --password-stdin get personal github --field password

# List all entries
cista list personal

# Search
cista search personal github

# Change master password
cista passwd personal

# Generate standalone password
cista generate --length 20 --no-symbols
```

## On-disk layout (XDG)

```
~/.config/cista/config.toml              # config (auto-lock timeout)
~/.local/share/cista/vaults/*.cista      # encrypted vaults
~/.local/share/cista/backups/<name>/     # timestamped .cista.bak backups
~/.local/state/cista/meta/<hash>.json    # last-opened metadata (path-hashed)
```

## Security

- **KDF:** Argon2id (64 MiB, 3 iterations)
- **AEAD:** XChaCha20-Poly1305; file header authenticated as AAD
- **Zeroization:** `secrecy::Secret` — secrets zeroized on drop
- **Clipboard:** auto-cleared after 15s; Linux uses `wait_until` + `exclude_from_history` (sets `x-kde-passwordManagerHint`); macOS/Windows use timer thread
- **Core dumps:** `RLIMIT_CORE=0` at session start
- **Memory paging:** best-effort `mlockall` (requires `ulimit -l unlimited`)

### Threat model

Cista does **not** protect against malware or a compromised machine while the vault is unlocked. An attacker with root or `/proc/<pid>/mem` access can extract secrets. Mitigations only raise cost of low-privilege attacks.

### Not pursued

- Terminal injection (`TIOCSTI`) — kernel already restricts this; chasing it adds complexity without meaningful risk reduction

## Interactive session

```bash
cista open personal
```

Inside the REPL you can run `add`, `get`, `list`, `search`, `edit`, `rm`, `lock`, `unlock`, `help`, `exit` without re-entering the vault path or master password. Auto-locks after 300s inactivity (configurable in `config.toml`).

## Build

```bash
cargo build --release
# binary at target/release/cista
```

## License

MIT