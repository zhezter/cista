mod cli;
mod clipboard;
mod handlers;
mod prompts;
mod repl;
mod table;
mod ui;
mod vault_session;

use clap::{CommandFactory, Parser};
use cli::{resolve_vault_path, Cli, Command};
use prompts::InputSource;

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let mut stdin_input = prompts::StdinInput;
    let mut stdin_password_input = prompts::StdinPasswordInput;
    let input: &mut dyn InputSource = if cli.password_stdin {
        &mut stdin_password_input
    } else {
        &mut stdin_input
    };

    match cli.cmd {
        Command::Init { mut path } => {
            resolve_vault_path(&mut path)?;
            handlers::handle_init(path, input)?
        }
        Command::Add {
            mut path,
            generate,
            length,
        } => {
            resolve_vault_path(&mut path)?;
            let (mut vault, password) = vault_session::unlock_vault(&path, input)?;
            handlers::apply_add(&mut vault, &path, &password, input, generate, length)?
        }
        Command::Get {
            mut path,
            name,
            field,
        } => {
            resolve_vault_path(&mut path)?;
            let (vault, _password) = vault_session::unlock_vault(&path, input)?;
            handlers::apply_get(&vault, &name, input, field)?
        }
        Command::List { mut path } => {
            resolve_vault_path(&mut path)?;
            let (vault, _password) = vault_session::unlock_vault(&path, input)?;
            handlers::apply_list(&vault)?
        }
        Command::Search { mut path, term } => {
            resolve_vault_path(&mut path)?;
            let (vault, _password) = vault_session::unlock_vault(&path, input)?;
            handlers::apply_search(&vault, term.as_deref())?
        }
        Command::Rm {
            mut path,
            name,
            yes,
        } => {
            resolve_vault_path(&mut path)?;
            let (mut vault, password) = vault_session::unlock_vault(&path, input)?;
            handlers::apply_rm(&mut vault, &path, &password, &name, input, yes)?
        }
        Command::Edit { mut path, name } => {
            resolve_vault_path(&mut path)?;
            let (mut vault, password) = vault_session::unlock_vault(&path, input)?;
            handlers::apply_edit(&mut vault, &path, &password, &name, input)?
        }
        Command::Generate {
            length,
            no_symbols,
            exclude_ambiguous,
        } => handlers::handle_generate(length, no_symbols, exclude_ambiguous)?,
        Command::Passwd { mut path } => {
            resolve_vault_path(&mut path)?;
            let (mut vault, _old_password) = vault_session::unlock_vault(&path, input)?;
            handlers::apply_passwd(&mut vault, &path, input)?;
        }
        Command::ListVaults => handlers::handle_list_vaults()?,
        Command::Open { mut path } => {
            resolve_vault_path(&mut path)?;
            let (vault, password) = vault_session::unlock_vault(&path, input)?;
            repl::run(path, vault, password, input)?;
        }
        Command::GenerateCompletions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            let mut buf = Vec::new();
            clap_complete::generate(shell, &mut cmd, name, &mut buf);
            use std::io::Write;
            if let Err(e) = std::io::stdout().write_all(&buf) {
                if e.kind() != std::io::ErrorKind::BrokenPipe {
                    return Err(e.into());
                }
            }
        }
    }
    Ok(())
}
