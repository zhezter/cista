//! File logging for non-fatal runtime errors.
//!
//! Anything that could corrupt the terminal UI (a transient I/O error, a
//! panic) is written to a log file under the XDG state directory instead of
//! being printed to stderr, so the alternate screen and raw-mode layout are
//! never broken by stray output.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

static LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// Opens (or creates) the log file at `~/.local/state/cista/cista-tui.log`.
pub fn init() {
    let path = cista_core::paths::state_dir()
        .ok()
        .map(|dir| dir.join("cista-tui.log"));
    let file = path.and_then(|p| {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
            .ok()
    });
    *LOG.lock().expect("log mutex poisoned") = file;
}

/// Appends a timestamped message to the log file. Never panics and never
/// touches the terminal.
pub fn log_error(msg: &str) {
    if let Ok(mut guard) = LOG.lock() {
        if let Some(file) = guard.as_mut() {
            let ts = time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "unknown".into());
            let _ = writeln!(file, "[{ts}] {msg}");
            let _ = file.flush();
        }
    }
}

/// Logs a panic payload (used by the panic hook) and also lets the default
/// hook print, so the diagnosis survives on the *shell* after we tear down
/// the alternate screen.
pub fn log_panic(info: &std::panic::PanicHookInfo) {
    log_error(&format!("panic: {info}"));
}