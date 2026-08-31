# cista-tui

Terminal UI for [Cista](https://github.com/zhezter/cista), a local, encrypted password manager written in Rust. A keyboard-driven, KeepassXC-style interface built with [ratatui](https://ratatui.rs).

`cista-tui` installs a binary named **`cista-tui`**.

## Install

```bash
cargo install cista-tui
```

Or from the repository:

```bash
cargo build --release   # binary at target/release/cista-tui
cista-tui
```

## Features

- **Vault selector** — lists your `.cista` vaults with entry counts and last access; open (`Enter`), create (`n`) or delete with master-password confirmation (`d`).
- **KeepassXC-style entry list** — entries table (Title / User / Modified / URL) beside a detail pane showing username, URL, dates and a health score; paginated navigation.
- **Live search** — `/` filters in real time across name, username, URL and notes, with matching text highlighted; `Esc` clears.
- **Sorting** — `o` cycles through name, modified and created, ascending and descending.
- **Password generator** — `Ctrl+g` from any screen; configure length and character classes, `Enter`/`r` to generate and reroll, `c` to copy, and a second `Ctrl+g` to drop the result straight into the entry form's password field.
- **Health check** — after unlocking, every password is scored 0–100 (strength, class variety, reuse, age) and summarized in the header.
- **Clipboard auto-clear** — copied passwords/usernames/URLs are cleared after 15 s.
- **Auto-lock** — the vault locks after inactivity (default 300s, configurable) and the master password is dropped from memory.
- **Background tasks** — Argon2 unlock/create/save/delete run on threads behind a spinner so the UI never stalls.

## Key bindings

From the entry list:

```
[↑/↓] Navigate  [PgUp/PgDn] Page  [/] Search  [a] Add  [Ctrl+g] Generate
[d] Delete  [Enter] View  [c] Copy pass  [q] Quit  [?] Help
```

Within forms: `Tab`/`Shift+Tab` move between fields, `Ctrl+s` saves, `Esc` goes back.

## Plugin of the workspace

`cista-tui` uses `cista-core` for the vault model, file format and cryptography, and `arboard` for the clipboard.

## License

MIT
