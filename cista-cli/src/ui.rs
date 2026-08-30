//! Tiny helpers for colored, consistent terminal output.

use colored::Colorize;

pub fn success(msg: impl std::fmt::Display) -> String {
    msg.to_string().green().bold().to_string()
}

pub fn warn(msg: impl std::fmt::Display) -> String {
    msg.to_string().yellow().bold().to_string()
}

pub fn error(msg: impl std::fmt::Display) -> String {
    msg.to_string().red().bold().to_string()
}
