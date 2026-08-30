//! Abstraction over terminal input so handlers can be driven by a real
//! terminal (`StdinInput`), a test harness, or a future REPL.

use std::io::Write;

/// A source of interactive input for the handlers.
pub trait InputSource {
    /// Reads a line of text (echoed), trimming trailing whitespace.
    fn read_line(&mut self, prompt: &str) -> anyhow::Result<String>;

    /// Reads a secret (hidden echo) such as a password.
    fn read_password(&mut self, prompt: &str) -> anyhow::Result<String>;

    /// Reads a line and returns whether the answer starts with `y`/`Y`.
    fn prompt_yes_no(&mut self, prompt: &str) -> anyhow::Result<bool> {
        let answer = self.read_line(prompt)?;
        Ok(answer.eq_ignore_ascii_case("y"))
    }

    /// Reads a line, mapping an empty answer to `None`.
    fn prompt_optional_line(&mut self, prompt: &str) -> anyhow::Result<Option<String>> {
        let value = self.read_line(prompt)?;
        Ok(if value.is_empty() { None } else { Some(value) })
    }
}

/// Reads from the real terminal.
pub struct StdinInput;

impl InputSource for StdinInput {
    fn read_line(&mut self, prompt: &str) -> anyhow::Result<String> {
        print!("{prompt}");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    fn read_password(&mut self, prompt: &str) -> anyhow::Result<String> {
        Ok(rpassword::prompt_password(prompt)?)
    }
}

/// Reads non-secret lines from stdin like [`StdinInput`], but reads passwords
/// from stdin too (instead of `/dev/tty`). Enables scripting where the master
/// password is piped in as the first line of stdin.
pub struct StdinPasswordInput;

impl InputSource for StdinPasswordInput {
    fn read_line(&mut self, prompt: &str) -> anyhow::Result<String> {
        print!("{prompt}");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    fn read_password(&mut self, _prompt: &str) -> anyhow::Result<String> {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        Ok(input.trim_end_matches(['\r', '\n']).to_string())
    }
}
