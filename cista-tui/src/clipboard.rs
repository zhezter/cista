//! Copy secrets to the clipboard with automatic clearing (TUI version).

use anyhow::Result;
use arboard::Clipboard;
use std::time::Duration;

const CLEAR_DELAY_SECONDS: u64 = 15;

/// Copies `text` to the system clipboard and clears it after `CLEAR_DELAY_SECONDS`.
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
