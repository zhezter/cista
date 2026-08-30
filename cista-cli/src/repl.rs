//! Interactive REPL over an already-open vault.

use crate::handlers;
use crate::prompts::InputSource;
use crate::ui;
use cista_core::config::Config;
use cista_core::SecretString;
use cista_core::Vault;
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::line_buffer::LineBuffer;
use rustyline::validate::Validator;
use rustyline::{Changeset, Context, Editor, Helper};
use secrecy::Secret;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

struct Session {
    path: PathBuf,
    vault: Option<Vault>,
    password: Option<Secret<SecretString>>,
    locked: bool,
    last_activity: Instant,
    auto_lock_seconds: u64,
}

#[derive(Debug)]
enum ReplCommand {
    Add,
    Get(String),
    List,
    Search(Option<String>),
    Edit(String),
    Rm(String),
    Passwd,
    Generate {
        length: usize,
        no_symbols: bool,
        exclude_ambiguous: bool,
    },
    Lock,
    Unlock,
    Help,
    Exit,
}

impl Session {
    fn lock(&mut self) {
        // Dropping the vault and password zeroizes the secrets via Secret::drop.
        self.vault = None;
        self.password = None;
        self.locked = true;
    }

    fn unlock(&mut self, input: &mut dyn InputSource) -> anyhow::Result<()> {
        let raw = input.read_password("Master password: ")?;
        let (vault, password) = crate::vault_session::unlock_raw(&self.path, &raw)?;
        self.vault = Some(vault);
        self.password = Some(password);
        self.locked = false;
        self.touch();
        Ok(())
    }

    fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    fn is_expired(&self) -> bool {
        !self.locked
            && self.auto_lock_seconds != 0
            && self.last_activity.elapsed() >= Duration::from_secs(self.auto_lock_seconds)
    }

    fn vault(&self) -> anyhow::Result<&Vault> {
        self.vault
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("vault is locked"))
    }
}

fn parse_command(line: &str) -> ReplCommand {
    let mut parts = line.split_whitespace();
    let cmd = parts.next().unwrap_or("");
    let rest: Vec<&str> = parts.collect();

    match cmd {
        "add" => ReplCommand::Add,
        "list" | "ls" => ReplCommand::List,
        "search" => {
            let term = if rest.is_empty() {
                None
            } else {
                Some(rest.join(" "))
            };
            ReplCommand::Search(term)
        }
        "get" => ReplCommand::Get(rest.join(" ")),
        "edit" => ReplCommand::Edit(rest.join(" ")),
        "rm" | "remove" | "delete" => ReplCommand::Rm(rest.join(" ")),
        "passwd" | "password" => ReplCommand::Passwd,
        "generate" | "gen" => {
            let default_len = Config::load()
                .map(|c| c.default_generate_length)
                .unwrap_or(20);
            let mut length = default_len;
            let mut no_symbols = false;
            let mut exclude_ambiguous = false;
            let mut it = rest.iter();
            while let Some(a) = it.next() {
                match *a {
                    "--length" | "-l" => {
                        if let Some(v) = it.next() {
                            length = v.parse().unwrap_or(default_len);
                        }
                    }
                    "--no-symbols" => no_symbols = true,
                    "--exclude-ambiguous" => exclude_ambiguous = true,
                    _ => {}
                }
            }
            ReplCommand::Generate {
                length,
                no_symbols,
                exclude_ambiguous,
            }
        }
        "lock" => ReplCommand::Lock,
        "unlock" => ReplCommand::Unlock,
        "help" | "?" => ReplCommand::Help,
        "exit" | "quit" | "q" => ReplCommand::Exit,
        _ => ReplCommand::Help,
    }
}

fn print_help() {
    println!("commands: add, get <name>, list, search [term], edit <name>,");
    println!("         rm <name>, passwd, generate [--length N] [--no-symbols]");
    println!("         lock, unlock, help, exit");
}

const COMMANDS: &[&str] = &[
    "add", "get", "list", "search", "edit", "rm", "passwd", "generate", "lock", "unlock", "help",
    "exit",
];

/// Commands whose second argument is an entry name, completed from the vault.
const NAME_COMMANDS: &[&str] = &["get", "edit", "rm"];

/// rustyline completer/highlighter/hinter/validator for the REPL.
///
/// Streams entry names are owned by the `Session`; the completer reads a
/// shared snapshot that is refreshed after every command mutating the vault.
struct ReplCompleter {
    entry_names: Rc<RefCell<Vec<String>>>,
}

impl Completer for ReplCompleter {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<String>)> {
        let before = &line[..pos];

        let start = before
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_whitespace())
            .map(|(i, _)| i + 1)
            .unwrap_or(0);
        let partial = &before[start..];
        let prefix = before[..start].trim();

        if prefix.is_empty() {
            // First token: complete against the command list.
            let candidates = COMMANDS
                .iter()
                .filter(|c| c.starts_with(partial))
                .map(|c| c.to_string())
                .collect();
            return Ok((start, candidates));
        }

        // Second token of a name command: complete entry names.
        let first = prefix.split_whitespace().next().unwrap_or("");
        if NAME_COMMANDS.contains(&first) {
            let candidates = self
                .entry_names
                .borrow()
                .iter()
                .filter(|n| n.starts_with(partial))
                .cloned()
                .collect();
            return Ok((start, candidates));
        }

        Ok((start, Vec::new()))
    }

    fn update(&self, line: &mut LineBuffer, start: usize, elected: &str, cl: &mut Changeset) {
        line.replace(start..line.pos(), elected, cl);
    }
}

impl Helper for ReplCompleter {}

impl Highlighter for ReplCompleter {}

impl Hinter for ReplCompleter {
    type Hint = String;
}

impl Validator for ReplCompleter {}

fn refresh_entry_names(names: &Rc<RefCell<Vec<String>>>, vault: &Vault) {
    let mut list = names.borrow_mut();
    list.clear();
    for entry in vault.entries() {
        list.push(entry.name().to_string());
    }
}

fn run_command(
    session: &mut Session,
    cmd: ReplCommand,
    input: &mut dyn InputSource,
) -> anyhow::Result<bool> {
    match cmd {
        ReplCommand::Exit => return Ok(false),
        ReplCommand::Help => print_help(),
        ReplCommand::Lock => {
            session.lock();
            println!("Vault locked.");
        }
        ReplCommand::Unlock => {
            session.unlock(input)?;
            println!("Vault unlocked.");
        }
        ReplCommand::Add => {
            let vault = session
                .vault
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("vault is locked"))?;
            let password = session
                .password
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("vault is locked"))?;
            handlers::apply_add(vault, &session.path, password, input, false, 20)?;
        }
        ReplCommand::Get(name) => handlers::apply_get(session.vault()?, &name, input, None)?,
        ReplCommand::List => handlers::apply_list(session.vault()?)?,
        ReplCommand::Search(term) => handlers::apply_search(session.vault()?, term.as_deref())?,
        ReplCommand::Edit(name) => {
            let vault = session
                .vault
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("vault is locked"))?;
            let password = session
                .password
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("vault is locked"))?;
            handlers::apply_edit(vault, &session.path, password, &name, input)?;
        }
        ReplCommand::Rm(name) => {
            let vault = session
                .vault
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("vault is locked"))?;
            let password = session
                .password
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("vault is locked"))?;
            handlers::apply_rm(vault, &session.path, password, &name, input, false)?;
        }
        ReplCommand::Passwd => {
            let vault = session
                .vault
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("vault is locked"))?;
            let new_password = handlers::apply_passwd(vault, &session.path, input)?;
            let _ = session.password.replace(new_password);
        }
        ReplCommand::Generate {
            length,
            no_symbols,
            exclude_ambiguous,
        } => {
            handlers::handle_generate(length, no_symbols, exclude_ambiguous)?;
        }
    }
    session.touch();
    Ok(true)
}

/// Applies hardening that is safe to skip if the system disallows it.
pub fn apply_hardening() {
    // Disable core dumps so crashing with the vault open doesn't write secrets
    // to disk. Best-effort.
    let rlim = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    unsafe {
        libc::setrlimit(libc::RLIMIT_CORE, &rlim);
    }

    // Try to keep process memory out of swap. May fail on systems with a low
    // RLIMIT_MEMLOCK (e.g. default 64 KB); we warn and continue.
    if unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) } != 0 {
        eprintln!(
            "{}",
            ui::warn("mlockall failed: process memory may be swappable (raise ulimit -l to fix)")
        );
    }
}

pub fn run(
    path: PathBuf,
    vault: Vault,
    password: Secret<SecretString>,
    input: &mut dyn InputSource,
) -> anyhow::Result<()> {
    apply_hardening();

    let auto_lock_seconds = cista_core::config::Config::load()
        .map(|c| c.auto_lock_seconds)
        .unwrap_or(300);

    let mut session = Session {
        path,
        vault: Some(vault),
        password: Some(password),
        locked: false,
        last_activity: Instant::now(),
        auto_lock_seconds,
    };

    println!(
        "Cista session on {:?}. Type 'help' for commands.",
        session.path
    );
    print_help();

    let entry_names = Rc::new(RefCell::new(Vec::new()));
    if let Some(vault) = session.vault.as_ref() {
        refresh_entry_names(&entry_names, vault);
    }
    let completer = ReplCompleter {
        entry_names: entry_names.clone(),
    };
    let mut editor = Editor::<ReplCompleter, DefaultHistory>::new()?;
    editor.set_helper(Some(completer));

    loop {
        // Auto-lock based on inactivity. Checked both before blocking on the
        // next line and, crucially, after readline returns: readline blocks for
        // an unbounded time, so an expiry that elapses while waiting must be
        // re-evaluated before dispatching, otherwise a whole command could run
        // after the timeout had already lapsed.
        if session.is_expired() {
            println!("Vault locked due to inactivity.");
            session.lock();
        }

        let prompt = if session.locked {
            "cista (locked)> "
        } else {
            "cista> "
        };
        let readline = editor.readline(prompt);
        match readline {
            Ok(line) => {
                // Re-check expiry now that a line arrived, before acting on it:
                // while readline was blocking, the session may have expired.
                if session.is_expired() {
                    println!("Vault locked due to inactivity.");
                    session.lock();
                }

                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let cmd = parse_command(trimmed);
                let allowed =
                    !session.locked || matches!(cmd, ReplCommand::Unlock | ReplCommand::Exit);
                if allowed {
                    let cont = run_command(&mut session, cmd, input)?;
                    if let Some(vault) = session.vault.as_ref() {
                        refresh_entry_names(&entry_names, vault);
                    }
                    if !cont {
                        break;
                    }
                } else {
                    println!("Vault is locked. Run 'unlock' or 'exit'.");
                }
            }
            Err(ReadlineError::Interrupted) => break,
            Err(ReadlineError::Eof) => break,
            Err(err) => return Err(err.into()),
        }
    }

    println!("bye.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_commands() {
        assert!(matches!(parse_command("list"), ReplCommand::List));
        assert!(matches!(parse_command("ls"), ReplCommand::List));
        assert!(matches!(parse_command("add"), ReplCommand::Add));
        assert!(matches!(parse_command("lock"), ReplCommand::Lock));
        assert!(matches!(parse_command("unlock"), ReplCommand::Unlock));
        assert!(matches!(parse_command("exit"), ReplCommand::Exit));
        assert!(matches!(parse_command("quit"), ReplCommand::Exit));
        assert!(matches!(parse_command("q"), ReplCommand::Exit));
    }

    #[test]
    fn parses_named_commands() {
        match parse_command("get github") {
            ReplCommand::Get(name) => assert_eq!(name, "github"),
            other => panic!("unexpected: {other:?}"),
        }
        match parse_command("edit my bank") {
            ReplCommand::Edit(name) => assert_eq!(name, "my bank"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parses_generate_flags() {
        match parse_command("generate --length 30 --no-symbols") {
            ReplCommand::Generate {
                length,
                no_symbols,
                exclude_ambiguous,
            } => {
                assert_eq!(length, 30);
                assert!(no_symbols);
                assert!(!exclude_ambiguous);
            }
            other => panic!("unexpected: {other:?}"),
        }
        match parse_command("gen") {
            ReplCommand::Generate {
                length,
                no_symbols,
                exclude_ambiguous,
            } => {
                assert_eq!(length, 20); // default config value
                assert!(!no_symbols);
                assert!(!exclude_ambiguous);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn unknown_command_falls_back_to_help() {
        assert!(matches!(
            parse_command("totally-unknown"),
            ReplCommand::Help
        ));
    }

    fn completer_with(names: &[&str]) -> ReplCompleter {
        ReplCompleter {
            entry_names: Rc::new(RefCell::new(names.iter().map(|s| s.to_string()).collect())),
        }
    }

    fn ctx() -> Context<'static> {
        let history: &'static DefaultHistory = Box::leak(Box::new(DefaultHistory::new()));
        Context::new(history)
    }

    #[test]
    fn completes_commands_from_command_prefix() {
        let c = completer_with(&[]);
        let (start, cands) = c.complete("li", 2, &ctx()).unwrap();
        assert_eq!(start, 0);
        assert!(cands.iter().any(|c| c == "list"));
        let (_, cands) = c.complete("gen", 3, &ctx()).unwrap();
        assert!(cands.iter().any(|c| c == "generate"));
    }

    #[test]
    fn complete_start_is_word_start() {
        let c = completer_with(&["github"]);
        let (start, cands) = c.complete("get g", 5, &ctx()).unwrap();
        assert_eq!(start, 4);
        assert_eq!(cands, vec!["github".to_string()]);
    }

    #[test]
    fn completes_entry_names_for_name_commands() {
        let c = completer_with(&["github", "gmail", "bank"]);
        let (start, cands) = c.complete("get g", 5, &ctx()).unwrap();
        assert_eq!(start, 4);
        assert_eq!(cands, vec!["github".to_string(), "gmail".to_string()]);
    }

    #[test]
    fn does_not_complete_entry_names_for_non_name_commands() {
        let c = completer_with(&["github", "gmail", "bank"]);
        let (_, cands) = c.complete("list g", 6, &ctx()).unwrap();
        assert!(cands.is_empty());
    }

    #[test]
    fn no_completion_for_unknown_value() {
        let c = completer_with(&[]);
        let (_, cands) = c.complete("zzz", 3, &ctx()).unwrap();
        assert!(cands.is_empty());
    }
}
