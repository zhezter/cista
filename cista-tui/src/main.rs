use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

mod app;
mod clipboard;
mod keys;
mod log;
mod screens;
mod tasks;
mod widgets;

use app::{App, AppSignal};
use keys::KeyBindings;

fn main() {
    log::init();
    install_panic_hook();

    // If setup itself fails there is no TUI running yet, so returning an error
    // to the caller is safe here.
    let res = (|| -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let keybindings = KeyBindings::default();
        let mut app = App::new(keybindings);

        let res =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_app(&mut terminal, &mut app)));

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        match res {
            Ok(Ok(())) => Ok(()),
            Ok(Err(err)) => {
                log::log_error(&format!("app error: {err}"));
                Ok(())
            }
            Err(payload) => {
                log::log_error(&format!("panic: {payload:?}"));
                Ok(())
            }
        }
    })();

    if let Err(err) = res {
        eprintln!("Error: {err}");
    }
}

/// Logs panics to the log file as well as to stderr. The default hook runs
/// second so the message is still visible on the shell once the alternate
/// screen has been torn down.
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::log_panic(info);
        default(info);
    }));
}

const MAX_CONSECUTIVE_ERRORS: u32 = 20;

/// Runs the render/input loop. Transient terminal errors are logged and
/// swallowed so they never break the layout; only a persistent failure ends
/// the session.
fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let mut consecutive_errors = 0;

    loop {
        if let Err(err) = terminal.draw(|f| app.draw(f)) {
            log::log_error(&format!("draw error: {err}"));
            consecutive_errors += 1;
            let _ = terminal.clear();
            if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                return Err(anyhow::anyhow!("terminal is no longer usable: {err}"));
            }
            continue;
        }
        consecutive_errors = 0;

        app.check_auto_lock();

        let ready = match event::poll(Duration::from_millis(100)) {
            Ok(ready) => ready,
            Err(err) => {
                log::log_error(&format!("event poll error: {err}"));
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        };

        if !ready {
            continue;
        }

        match event::read() {
            Ok(Event::Key(key)) => {
                if key.kind == KeyEventKind::Press
                    && matches!(app.handle_key(key), AppSignal::Quit)
                {
                    break;
                }
            }
            Ok(_) => {}
            Err(err) => {
                log::log_error(&format!("event read error: {err}"));
            }
        }
    }
    Ok(())
}
