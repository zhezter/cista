use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "cista", version, about = "Local encrypted password manager")]
pub struct Cli {
    /// Read the master password from stdin instead of the terminal.
    ///
    /// For scripting/automation. The password must be the first (and typically only) line on stdin; use with non-interactive commands such as
    /// `get --field`, `list`, `search`, or `rm --yes`. Never pass a password as
    /// a command-line argument.
    #[arg(long, global = true)]
    pub password_stdin: bool,

    #[command(subcommand)]
    pub cmd: Command,
}

/// Validates a vault path. Bare names (no directory separators) resolve to
/// `~/.local/share/cista/vaults/<name>.cista`; paths with a directory are used
/// as-is. Always requires the `.cista` extension.
pub fn resolve_vault_path(path: &mut PathBuf) -> anyhow::Result<()> {
    let bare_name = path
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)));

    if bare_name {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let ends_with_cista = Path::new(&name)
            .extension()
            .map(|e| e.eq_ignore_ascii_case("cista"))
            .unwrap_or(false);

        if ends_with_cista {
            *path = cista_core::paths::vaults_dir()?.join(name);
        } else if Path::new(&name).extension().is_some() {
            // A bare name with a non-.cista extension is likely a typo.
            anyhow::bail!("vault path '{}' must end with the .cista extension", name);
        } else {
            // No extension at all: complete it to a .cista name.
            let resolved = format!("{name}.cista");
            *path = cista_core::paths::vaults_dir()?.join(resolved);
        }
    }

    validate_vault_path(path)
}

/// Returns an error unless `path` ends with the `.cista` extension,
/// so vaults are always given an identifiable file name.
pub fn validate_vault_path(path: &Path) -> anyhow::Result<()> {
    let has_cista_ext = path
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("cista"))
        .unwrap_or(false);
    if !has_cista_ext {
        anyhow::bail!(
            "vault path '{}' must end with the .cista extension",
            path.display()
        );
    }
    Ok(())
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Init {
        path: PathBuf,
    },
    Add {
        path: PathBuf,
        /// Generate a random password without prompting.
        #[arg(long)]
        generate: bool,
        #[arg(long, default_value_t = 20)]
        length: usize,
    },
    Get {
        path: PathBuf,
        name: String,
        /// Print only this field (non-interactive).
        #[arg(long)]
        field: Option<FieldSelector>,
    },
    List {
        path: PathBuf,
    },
    Search {
        path: PathBuf,
        term: Option<String>,
    },
    Rm {
        path: PathBuf,
        name: String,
        /// Skip the confirmation prompt.
        #[arg(long)]
        yes: bool,
    },
    Edit {
        path: PathBuf,
        name: String,
    },
    Generate {
        #[arg(short, long, default_value_t = 20)]
        length: usize,
        #[arg(long)]
        no_symbols: bool,
        #[arg(long)]
        exclude_ambiguous: bool,
    },
    Passwd {
        path: PathBuf,
    },
    ListVaults,
    Open {
        path: PathBuf,
    },
    /// Generate shell completion scripts for the `cista` command line.
    GenerateCompletions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

/// A single field that `get --field` can print.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldSelector {
    Password,
    Username,
    Url,
    Notes,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_name_without_extension_resolves_to_cista() {
        let mut path = PathBuf::from("personal");
        resolve_vault_path(&mut path).expect("should resolve");
        assert!(path.is_absolute());
        assert!(path.ends_with("personal.cista"));
    }

    #[test]
    fn bare_name_with_extension_resolves() {
        let mut path = PathBuf::from("personal.cista");
        resolve_vault_path(&mut path).expect("should resolve");
        assert!(path.is_absolute());
        assert!(path.ends_with("personal.cista"));
    }

    #[test]
    fn explicit_path_with_directory_is_preserved() {
        let mut path = PathBuf::from("/tmp/a.cista");
        resolve_vault_path(&mut path).expect("should resolve");
        assert_eq!(path, PathBuf::from("/tmp/a.cista"));
    }

    #[test]
    fn relative_path_with_directory_is_preserved() {
        let mut path = PathBuf::from("./my.cista");
        resolve_vault_path(&mut path).expect("should resolve");
        assert_eq!(path, PathBuf::from("./my.cista"));
    }

    #[test]
    fn rejects_non_cista_extension() {
        let mut path = PathBuf::from("foo.txt");
        assert!(resolve_vault_path(&mut path).is_err());
    }

    /// Splits a typed line into argv tokens the same way a REPL would, then parses
    /// it with the exact same `Command` enum used for real argv.
    fn parse_line(line: &str) -> clap::error::Result<Cli> {
        let mut argv: Vec<&str> = vec!["cista"];
        argv.extend(line.split_whitespace());
        Cli::try_parse_from(argv)
    }

    #[test]
    fn parse_line_reuses_command_surface() {
        let cli = parse_line("list personal").expect("'list personal' should parse");
        match cli.cmd {
            Command::List { path } => assert_eq!(path, PathBuf::from("personal")),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_line_with_flags() {
        let cli = parse_line("generate --length 30 --no-symbols").expect("should parse");
        match cli.cmd {
            Command::Generate {
                length,
                no_symbols,
                exclude_ambiguous,
            } => {
                assert_eq!(length, 30);
                assert!(no_symbols);
                assert!(!exclude_ambiguous);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_line_rejects_unknown_subcommand() {
        assert!(parse_line("totally-not-a-command").is_err());
    }

    #[test]
    fn parse_line_generate_flag_on_add() {
        let cli = parse_line("add personal --generate --length 24").expect("should parse");
        match cli.cmd {
            Command::Add {
                path,
                generate,
                length,
            } => {
                assert_eq!(path, PathBuf::from("personal"));
                assert!(generate);
                assert_eq!(length, 24);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_line_add_generate_defaults_length() {
        let cli = parse_line("add personal --generate").expect("should parse");
        match cli.cmd {
            Command::Add {
                path,
                generate,
                length,
            } => {
                assert_eq!(path, PathBuf::from("personal"));
                assert!(generate);
                assert_eq!(length, 20);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_line_get_field_selector() {
        let cli = parse_line("get personal example --field password").expect("should parse");
        match cli.cmd {
            Command::Get { path, name, field } => {
                assert_eq!(path, PathBuf::from("personal"));
                assert_eq!(name, "example");
                assert_eq!(field, Some(FieldSelector::Password));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_line_get_without_field() {
        let cli = parse_line("get personal example").expect("should parse");
        match cli.cmd {
            Command::Get { field, .. } => assert_eq!(field, None),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_line_rm_yes_flag() {
        let cli = parse_line("rm personal example --yes").expect("should parse");
        match cli.cmd {
            Command::Rm { name, yes, .. } => {
                assert_eq!(name, "example");
                assert!(yes);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_line_global_password_stdin() {
        // Global flag accepted before the subcommand...
        let cli = parse_line("--password-stdin get personal example --field password")
            .expect("should parse");
        assert!(cli.password_stdin);

        // ...and after it (because it is `global = true`).
        let cli = parse_line("get personal example --field password --password-stdin")
            .expect("should parse");
        assert!(cli.password_stdin);

        // Default is off.
        let cli = parse_line("get personal example --field password").expect("should parse");
        assert!(!cli.password_stdin);
    }
}
