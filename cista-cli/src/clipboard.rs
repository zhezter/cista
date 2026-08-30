//! Copy secrets to the clipboard with automatic clearing.

use anyhow::Result;
use arboard::Clipboard;
use std::time::Duration;

const CLEAR_DELAY_SECONDS: u64 = 15;

/// Copies `text` to the system clipboard and clears it after `CLEAR_DELAY_SECONDS`.
///
/// On Linux the clipboard is "owned" by the process that set it: it must keep
/// serving requests while a client (clipboard manager, paste) reads it, and the
/// contents vanish as soon as that owner exits. We therefore spawn a thread that
/// keeps a single `Clipboard` open and serves it for the full delay via
/// `wait_until` (from `arboard::SetExtLinux`).
///
/// `exclude_from_history()` sets the `x-kde-passwordManagerHint` MIME so
/// desktop clipboard managers do not persist the secret in their history.
///
/// On macOS and Windows the OS manages clipboard ownership, so we simply set the
/// text and spawn a timer thread to clear it after the delay.
pub fn copy_secret_to_clipboard(text: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use arboard::SetExtLinux;
        let owned = text.to_string();
        std::thread::spawn(move || {
            let mut clipboard = match Clipboard::new() {
                Ok(c) => c,
                Err(_) => return,
            };
            let until = std::time::Instant::now() + Duration::from_secs(CLEAR_DELAY_SECONDS);
            let result = clipboard
                .set()
                .wait_until(until)
                .exclude_from_history()
                .text(owned);
            if result.is_err() {
                return;
            }
            // `wait_until` blocks until the full delay has elapsed, so clear
            // immediately once it returns — no extra sleep here.
            let _ = clipboard.clear();
        });
    }

    #[cfg(not(target_os = "linux"))]
    {
        let owned = text.to_string();
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(owned.clone())?;
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(CLEAR_DELAY_SECONDS));
            if let Ok(mut c) = Clipboard::new() {
                let _ = c.clear();
            }
        });
    }

    Ok(())
}