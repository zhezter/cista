# cista-cli

Command-line interface for [Cista](https://github.com/zhezter/cista), a local, encrypted password manager written in Rust.

`cista-cli` installs a binary named **`cista`**.

## Install

```bash
cargo install cista-cli
```

Or from the repository:

```bash
cargo build --release   # binary at target/release/cista
```

## Usage

A vault is a single encrypted `*.cista` file. A bare name (no directory separator) resolves to `~/.local/share/cista/vaults/<name>.cista`.

| Command | Description |
|---------|-------------|
| `cista init [path]` | Create a new vault |
| `cista add <path>` | Add an entry. `--generate [--length N]` creates the password non-interactively |
| `cista get <path> <name>` | Show an entry. `--field <password\|username\|url\|notes>` prints a single field; the password is copied to the clipboard |
| `cista list <path>` | List all entries as a table |
| `cista search <path> [term]` | Substring search across name/username/url/notes |
| `cista edit <path> <name>` | Edit an entry interactively (leave blank to keep a value) |
| `cista rm <path> <name>` | Remove an entry. `--yes` skips confirmation |
| `cista passwd <path>` | Change the master password |
| `cista generate [--length N] [--no-symbols] [--exclude-ambiguous]` | Generate a standalone password to the clipboard |
| `cista list-vaults` | List vaults in the vault directory |
| `cista open <vault>` | Start an interactive REPL session |
| `cista generate-completions <shell>` | Emit shell completions (bash/zsh/fish/…) |

**Global flag:** `--password-stdin` reads the master password from the first line of stdin — for scripting with non-interactive commands such as `get --field`, `list`, `search` or `rm --yes`. Never pass a password as a command-line argument.

## Quick examples

```bash
# Create a vault
cista init personal

# Add an entry with a generated password
cista add personal --generate --length 16

# Get a password to the clipboard (scripting)
echo "$MASTER" | cista --password-stdin get personal github --field password

# List entries
cista list personal

# Browse interactively
cista open personal
```

## Interactive session (`cista open`)

Run `add`, `get`, `list`, `search`, `edit`, `rm`, `passwd`, `generate`, `lock`, `unlock`, `help` and `exit` without re-entering the vault path or master password. Includes command/entry-name autocompletion, inactivity auto-lock (default 300s, configurable), and hardening (core dumps disabled, best-effort `mlockall`).

## Security

- Secrets handled with `secrecy::Secret` — zeroized on drop.
- Clipboard auto-cleared after 15 s; Linux sets the `x-kde-passwordManagerHint` and excludes the copy from clipboard-manager history.
- Argon2id KDF, XChaCha20-Poly1305 AEAD (see `cista-core`).

## License

MIT
