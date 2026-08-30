use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use crate::keys::{Action, ActionMapper, KeyBindings};
use crate::screens::*;
use crate::tasks::{self, PendingTask, TaskKind, TaskResult};
use crate::widgets::*;
use cista_core::config::Config;
use cista_core::{SecretString, Vault};
use secrecy::{ExposeSecret, Secret};
use zeroize::Zeroize;

/// How long a status notification stays visible before auto-dismissing.
pub const STATUS_LIFETIME: Duration = Duration::from_secs(3);

/// A transient status line. Dismissed automatically by `App::draw`.
#[derive(Debug, Clone)]
pub struct StatusNotice {
    pub text: String,
    pub at: Instant,
}

/// Result of handling a key event.
///
/// This deliberately never uses `bool`: a handler returning "I consumed the
/// key" must not be confused with the user asking to quit the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSignal {
    Continue,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    VaultList,
    Unlock,
    EntryList,
    EntryDetail,
    EntryForm,
    NewVault,
    Generate,
    Locked,
    Confirm,
    Help,
}

pub struct App {
    pub screen: Screen,
    pub previous_screen: Option<Screen>,

    // Vault list
    pub vaults: Vec<VaultInfo>,
    pub vault_list_selected: usize,

    // Unlock
    pub unlock_password: String,
    pub unlock_error: Option<String>,

    // Session (when unlocked)
    pub vault_path: Option<PathBuf>,
    pub vault: Option<Vault>,
    pub master_password: Option<Secret<SecretString>>,
    pub locked: bool,
    pub last_activity: Instant,
    pub auto_lock_seconds: u64,

    // Entry list (dashboard)
    pub all_entries: Vec<EntryRow>,
    pub entries: Vec<EntryRow>,
    pub entry_list_selected: usize,
    pub entry_list_page: usize,
    pub per_page: usize,
    pub search_query: String,
    pub in_search: bool,

    // Entry detail
    pub detail_entry_idx: Option<usize>,
    pub show_password: bool,

    // Entry form (add/edit)
    pub form_mode: FormMode,
    pub form_fields: FormFields,
    pub form_field_idx: usize,

    // New vault form
    pub new_vault_fields: NewVaultFields,
    pub new_vault_field_idx: usize,

    // Generate
    pub gen_policy: GenPolicy,
    pub gen_selected: usize,
    pub gen_result: Option<String>,

    // Confirm dialog
    pub confirm_message: String,
    pub confirm_on_yes: Option<ConfirmAction>,

    // Help
    pub help_scroll: u16,

    // Input
    mapper: ActionMapper,

    // Runtime
    pub status_message: Option<StatusNotice>,
    pub status_error: Option<StatusNotice>,
    pub pending: Option<PendingTask>,
}

#[derive(Debug, Clone)]
pub struct VaultInfo {
    pub name: String,
    pub path: PathBuf,
    pub last_opened: Option<String>,
    pub entry_count: Option<usize>,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct EntryRow {
    pub id: uuid::Uuid,
    pub name: String,
    pub username: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormMode {
    Add,
    Edit,
}

#[derive(Debug, Clone, Default)]
pub struct FormFields {
    pub name: String,
    pub username: String,
    pub password: String,
    pub password_confirm: String,
    pub url: String,
    pub notes: String,
}

#[derive(Debug, Clone, Default)]
pub struct NewVaultFields {
    pub name: String,
    pub password: String,
    pub confirm: String,
}

#[derive(Debug, Clone)]
pub struct GenPolicy {
    pub length: usize,
    pub include_lowercase: bool,
    pub include_uppercase: bool,
    pub include_digits: bool,
    pub include_symbols: bool,
    pub exclude_ambiguous: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenOption {
    Length,
    Lowercase,
    Uppercase,
    Digits,
    Symbols,
    ExcludeAmbiguous,
}

impl GenOption {
    pub const ALL: [GenOption; 6] = [
        GenOption::Length,
        GenOption::Lowercase,
        GenOption::Uppercase,
        GenOption::Digits,
        GenOption::Symbols,
        GenOption::ExcludeAmbiguous,
    ];

    pub fn label(self) -> &'static str {
        match self {
            GenOption::Length => "Length",
            GenOption::Lowercase => "Lowercase (a-z)",
            GenOption::Uppercase => "Uppercase (A-Z)",
            GenOption::Digits => "Digits (0-9)",
            GenOption::Symbols => "Symbols (!@#$...)",
            GenOption::ExcludeAmbiguous => "Exclude ambiguous (0/O, 1/l/I)",
        }
    }
}

impl Default for GenPolicy {
    fn default() -> Self {
        let default_len = Config::load()
            .map(|c| c.default_generate_length)
            .unwrap_or(20);
        Self {
            length: default_len,
            include_lowercase: true,
            include_uppercase: true,
            include_digits: true,
            include_symbols: true,
            exclude_ambiguous: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    DeleteEntry,
    DeleteVault,
}

impl App {
    pub fn new(keybindings: KeyBindings) -> Self {
        let mapper = ActionMapper::new(keybindings);
        let auto_lock_seconds = Config::load().map(|c| c.auto_lock_seconds).unwrap_or(300);
        let mut app = Self {
            screen: Screen::VaultList,
            previous_screen: None,
            vaults: Vec::new(),
            vault_list_selected: 0,
            unlock_password: String::new(),
            unlock_error: None,
            vault_path: None,
            vault: None,
            master_password: None,
            locked: false,
            last_activity: Instant::now(),
            auto_lock_seconds,
            all_entries: Vec::new(),
            entries: Vec::new(),
            entry_list_selected: 0,
            entry_list_page: 0,
            per_page: 20,
            search_query: String::new(),
            in_search: false,
            detail_entry_idx: None,
            show_password: false,
            form_mode: FormMode::Add,
            form_fields: FormFields::default(),
            form_field_idx: 0,
            new_vault_fields: NewVaultFields::default(),
            new_vault_field_idx: 0,
            gen_policy: GenPolicy::default(),
            gen_selected: 0,
            gen_result: None,
            confirm_message: String::new(),
            confirm_on_yes: None,
            help_scroll: 0,
            mapper,
            status_message: None,
            status_error: None,
            pending: None,
        };
        app.load_vaults();
        app
    }

    fn load_vaults(&mut self) {
        self.vaults.clear();
        if let Ok(dir) = cista_core::paths::vaults_dir() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                        let fname = entry.file_name().to_string_lossy().into_owned();
                        if fname.ends_with(".cista") {
                            let name = fname.trim_end_matches(".cista").to_string();
                            let meta = cista_core::config::load_meta(&entry.path()).ok();
                            let last_opened = meta
                                .as_ref()
                                .and_then(|m| m.last_opened)
                                .map(|dt| dt.date().to_string());
                            let entry_count = meta.map(|m| m.entry_count);
                            let size = std::fs::metadata(entry.path())
                                .map(|m| m.len())
                                .unwrap_or(0);
                            self.vaults.push(VaultInfo {
                                name,
                                path: entry.path(),
                                last_opened,
                                entry_count,
                                size,
                            });
                        }
                    }
                }
            }
        }
        self.vaults.sort_by(|a, b| a.name.cmp(&b.name));
        self.vault_list_selected = 0;
    }

    pub fn draw(&mut self, f: &mut Frame) {
        self.poll_task();
        self.expire_status();
        match self.screen {
            Screen::VaultList => draw_vault_list(f, self),
            Screen::Unlock => draw_unlock(f, self),
            Screen::EntryList => draw_entry_list(f, self),
            Screen::EntryDetail => draw_entry_detail(f, self),
            Screen::EntryForm => draw_entry_form(f, self),
            Screen::NewVault => draw_new_vault(f, self),
            Screen::Generate => draw_generate(f, self),
            Screen::Locked => draw_lock_screen(f, self),
            Screen::Confirm => draw_confirm(f, self),
            Screen::Help => draw_help(f, self),
        }

        if let Some(msg) = &self.status_message {
            draw_status(f, &msg.text, false);
        }
        if let Some(err) = &self.status_error {
            draw_status(f, &err.text, true);
        }
        self.draw_busy_overlay(f);
    }

    /// Reaps the result of a background task, if one has finished.
    fn poll_task(&mut self) {
        let mut done: Option<(TaskKind, TaskResult)> = None;
        if let Some(task) = &self.pending {
            match task.rx.try_recv() {
                Ok(result) => done = Some((task.kind, result)),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    done = Some((task.kind, TaskResult::Failed));
                }
            }
        }
        if let Some((kind, result)) = done {
            self.pending = None;
            self.apply_task_result(kind, result);
        }
    }

    fn apply_task_result(&mut self, kind: TaskKind, result: TaskResult) {
        match kind {
            TaskKind::Unlock => match result {
                TaskResult::Unlock {
                    path,
                    password,
                    result: Ok(vault),
                } => {
                    let _ = cista_core::config::record_opened(&path);
                    self.vault_path = Some(path);
                    self.vault = Some(vault);
                    self.master_password = Some(password);
                    self.locked = false;
                    self.last_activity = Instant::now();
                    self.load_entries();
                    self.screen = Screen::EntryList;
                    self.set_status("Vault unlocked");
                }
                TaskResult::Unlock { result: Err(_), .. } => {
                    self.unlock_error = Some("Invalid master password".into());
                }
                _ => {}
            },
            TaskKind::CreateVault => match result {
                TaskResult::CreateVault {
                    name,
                    result: Ok(()),
                } => {
                    self.reset_new_vault_fields();
                    self.load_vaults();
                    self.screen = Screen::VaultList;
                    self.set_status(&format!("Vault '{name}' created"));
                }
                TaskResult::CreateVault { result: Err(e), .. } => {
                    self.set_error(&format!("Failed to create vault: {e}"))
                }
                _ => {}
            },
            TaskKind::SaveEntryAdd | TaskKind::SaveEntryEdit | TaskKind::SaveEntryDelete => {
                match result {
                    TaskResult::SaveVault { result: Ok(()) } => {
                        self.load_entries();
                        match kind {
                            TaskKind::SaveEntryAdd => {
                                self.set_status("Entry added");
                                self.back_from_form();
                            }
                            TaskKind::SaveEntryEdit => {
                                self.set_status("Entry saved");
                                self.back_from_form();
                            }
                            _ => self.set_status("Entry deleted"),
                        }
                    }
                    TaskResult::SaveVault { result: Err(e) } => {
                        self.set_error(&format!("Failed to save changes: {e}"));
                    }
                    _ => {}
                }
            }
        }
    }

    /// Renders a dimmed overlay with a spinner while a background task runs.
    fn draw_busy_overlay(&mut self, f: &mut Frame) {
        use ratatui::{
            layout::{Alignment, Constraint, Direction, Layout},
            style::{Color, Modifier, Style},
            widgets::{Block, BorderType, Borders, Paragraph},
        };

        let Some(task) = &self.pending else { return };
        let elapsed = task.started.elapsed().as_millis() as usize;

        let backdrop = Block::default().style(Style::default().bg(Color::Black));
        f.render_widget(backdrop, f.area());

        let area = centered_rect(46, 22, f.area());
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner = frames[(elapsed / 80) % frames.len()];
        let label = Paragraph::new(format!("{spinner} {}", task.kind.label()))
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        f.render_widget(label, chunks[1]);

        let note = Paragraph::new("Working… please wait")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(note, chunks[2]);

        let modal = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Working")
            .border_style(Style::default().fg(Color::Blue));
        f.render_widget(modal, area);
    }

    /// Dismisses status notifications that have outlived `STATUS_LIFETIME`.
    fn expire_status(&mut self) {
        let expired = |n: &Option<StatusNotice>| {
            n.as_ref()
                .is_some_and(|s| s.at.elapsed() >= STATUS_LIFETIME)
        };
        if expired(&self.status_message) {
            self.status_message = None;
        }
        if expired(&self.status_error) {
            self.status_error = None;
        }
    }

    /// Returns the text buffer that should receive printable characters on the
    /// current screen, if any. Screens without a buffer treat characters as
    /// actions.
    fn active_text_buffer(&mut self) -> Option<&mut String> {
        match self.screen {
            Screen::Unlock => Some(&mut self.unlock_password),
            Screen::EntryList if self.in_search => Some(&mut self.search_query),
            Screen::EntryForm => Some(match self.form_field_idx {
                0 => &mut self.form_fields.name,
                1 => &mut self.form_fields.username,
                2 => &mut self.form_fields.password,
                3 => &mut self.form_fields.password_confirm,
                4 => &mut self.form_fields.url,
                _ => &mut self.form_fields.notes,
            }),
            Screen::NewVault => Some(match self.new_vault_field_idx {
                0 => &mut self.new_vault_fields.name,
                1 => &mut self.new_vault_fields.password,
                _ => &mut self.new_vault_fields.confirm,
            }),
            _ => None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AppSignal {
        self.last_activity = Instant::now();

        // While a background task (unlock/create/save) is running, its spinner
        // modal is up: ignore all input so keys never land in a form hidden
        // behind it.
        if self.pending.is_some() {
            return AppSignal::Continue;
        }

        // 1) Global keys (Esc, Tab, Shift+Tab, Ctrl+s) fire even while a text
        //    field is focused: the user must always be able to cancel, move
        //    between fields or save. Ctrl+letter never leaks into a buffer.
        if let Some(action) = self.mapper.map_global(key) {
            return self.handle_action(action);
        }

        // 2) Printable text goes into the active buffer *before* action
        //    mapping, so characters like `q`, `g`, `d` never trigger actions
        //    while typing. Ctrl/Alt-modified keys are never captured.
        let mut edited = false;
        {
            if let Some(buffer) = self.active_text_buffer() {
                let plain = !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT);
                if plain {
                    match key.code {
                        KeyCode::Char(c) => {
                            buffer.push(c);
                            edited = true;
                        }
                        KeyCode::Backspace => {
                            buffer.pop();
                            edited = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        if edited {
            if self.in_search {
                self.recompute_filtered_entries();
            }
            return AppSignal::Continue;
        }

        // 3) Every other action, only when no field is consuming input.
        if let Some(action) = self.mapper.map(key) {
            self.handle_action(action)
        } else {
            AppSignal::Continue
        }
    }

    fn handle_action(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Lock => self.handle_lock(),
            Action::Up => self.handle_up(),
            Action::Down => self.handle_down(),
            Action::Left => self.handle_left(),
            Action::Right => self.handle_right(),
            Action::PageUp => self.handle_page_up(),
            Action::PageDown => self.handle_page_down(),
            Action::Home => self.handle_home(),
            Action::End => self.handle_end(),
            Action::Enter => self.handle_enter(),
            Action::Back => self.handle_back(),
            Action::Search => self.handle_search(),
            Action::Add => self.handle_add(),
            Action::Generate => self.handle_generate(),
            Action::Delete => self.handle_delete(),
            Action::CopyPassword => self.handle_copy_password(),
            Action::CopyUsername => self.handle_copy_username(),
            Action::CopyUrl => self.handle_copy_url(),
            Action::Edit => self.handle_edit(),
            Action::Reveal => self.handle_reveal(),
            Action::NewVault => self.handle_new_vault(),
            Action::Reroll => self.handle_reroll(),
            Action::TabNext => self.handle_tab_next(),
            Action::TabPrev => self.handle_tab_prev(),
            Action::Save => self.handle_save(),
        }
    }

    fn handle_help(&mut self) -> AppSignal {
        if self.screen == Screen::Help {
            self.screen = self.previous_screen.unwrap_or(Screen::VaultList);
        } else {
            self.previous_screen = Some(self.screen);
            self.screen = Screen::Help;
        }
        AppSignal::Continue
    }

    fn handle_lock(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList || self.screen == Screen::EntryDetail {
            self.lock_vault();
        }
        AppSignal::Continue
    }

    fn lock_vault(&mut self) {
        self.vault = None;
        self.master_password = None;
        self.locked = true;
        self.screen = Screen::Locked;
        self.in_search = false;
        self.search_query.clear();
        self.reset_form_fields();
    }

    fn handle_up(&mut self) -> AppSignal {
        match self.screen {
            Screen::VaultList => {
                if self.vault_list_selected > 0 {
                    self.vault_list_selected -= 1;
                }
            }
            Screen::EntryList => {
                if self.entry_list_selected > 0 {
                    self.entry_list_selected -= 1;
                    self.adjust_page();
                }
            }
            Screen::EntryForm | Screen::NewVault => {
                self.handle_tab_prev();
            }
            Screen::Generate => {
                self.gen_selected = self.gen_selected.saturating_sub(1);
            }
            Screen::Help if self.help_scroll > 0 => {
                self.help_scroll -= 1;
            }
            _ => {}
        }
        AppSignal::Continue
    }

    fn handle_down(&mut self) -> AppSignal {
        match self.screen {
            Screen::VaultList => {
                if self.vault_list_selected + 1 < self.vaults.len() {
                    self.vault_list_selected += 1;
                }
            }
            Screen::EntryList => {
                if self.entry_list_selected + 1 < self.entries.len() {
                    self.entry_list_selected += 1;
                    self.adjust_page();
                }
            }
            Screen::EntryForm | Screen::NewVault => {
                self.handle_tab_next();
            }
            Screen::Generate => {
                self.gen_selected = (self.gen_selected + 1).min(GenOption::ALL.len() - 1);
            }
            Screen::Help => {
                self.help_scroll = self.help_scroll.saturating_add(1);
            }
            _ => {}
        }
        AppSignal::Continue
    }

    /// Left/Right move across the focused generate option. For `Length` the
    /// value changes; for the boolean categories it toggles the state.
    fn handle_left(&mut self) -> AppSignal {
        if self.screen == Screen::Generate {
            let option = GenOption::ALL[self.gen_selected];
            match option {
                GenOption::Length => {
                    self.gen_policy.length = self.gen_policy.length.saturating_sub(1).max(4);
                }
                _ => self.toggle_gen_option(option),
            }
        }
        AppSignal::Continue
    }

    fn handle_right(&mut self) -> AppSignal {
        if self.screen == Screen::Generate {
            let option = GenOption::ALL[self.gen_selected];
            match option {
                GenOption::Length => {
                    self.gen_policy.length = (self.gen_policy.length + 1).min(128);
                }
                _ => self.toggle_gen_option(option),
            }
        }
        AppSignal::Continue
    }

    /// Regenerate the preview password (also bound to 'r').
    fn handle_reroll(&mut self) -> AppSignal {
        if self.screen == Screen::Generate {
            self.do_generate();
        }
        AppSignal::Continue
    }

    fn toggle_gen_option(&mut self, option: GenOption) {
        let p = &mut self.gen_policy;
        match option {
            GenOption::Lowercase => p.include_lowercase = !p.include_lowercase,
            GenOption::Uppercase => p.include_uppercase = !p.include_uppercase,
            GenOption::Digits => p.include_digits = !p.include_digits,
            GenOption::Symbols => p.include_symbols = !p.include_symbols,
            GenOption::ExcludeAmbiguous => p.exclude_ambiguous = !p.exclude_ambiguous,
            GenOption::Length => {}
        }
    }

    fn handle_page_up(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList {
            self.entry_list_selected = self.entry_list_selected.saturating_sub(self.per_page);
            self.adjust_page();
        } else if self.screen == Screen::Help {
            self.help_scroll = self.help_scroll.saturating_sub(15);
        }
        AppSignal::Continue
    }

    fn handle_page_down(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList {
            self.entry_list_selected = (self.entry_list_selected + self.per_page)
                .min(self.entries.len().saturating_sub(1));
            self.adjust_page();
        } else if self.screen == Screen::Help {
            self.help_scroll = self.help_scroll.saturating_add(15);
        }
        AppSignal::Continue
    }

    fn handle_home(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList {
            self.entry_list_selected = 0;
            self.entry_list_page = 0;
        } else if self.screen == Screen::Help {
            self.help_scroll = 0;
        }
        AppSignal::Continue
    }

    fn handle_end(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList {
            self.entry_list_selected = self.entries.len().saturating_sub(1);
            self.adjust_page();
        } else if self.screen == Screen::Help {
            // Set above the max; `draw_help` clamps it to the bottom edge.
            self.help_scroll = u16::MAX;
        }
        AppSignal::Continue
    }

    fn adjust_page(&mut self) {
        self.entry_list_page = self.entry_list_selected / self.per_page.max(1);
    }

    fn handle_enter(&mut self) -> AppSignal {
        match self.screen {
            Screen::VaultList => {
                if let Some(vault) = self.vaults.get(self.vault_list_selected).cloned() {
                    self.vault_path = Some(vault.path);
                    self.screen = Screen::Unlock;
                    self.unlock_password.clear();
                    self.unlock_error = None;
                }
            }
            Screen::Unlock => {
                self.try_unlock();
            }
            Screen::EntryList => {
                if let Some(idx) = self.get_selected_entry_idx() {
                    self.detail_entry_idx = Some(idx);
                    self.show_password = false;
                    self.screen = Screen::EntryDetail;
                }
            }
            Screen::EntryForm | Screen::NewVault => {
                self.handle_tab_next();
            }
            Screen::Generate => {
                self.do_generate();
            }
            Screen::Locked => {
                self.screen = Screen::Unlock;
                self.unlock_password.clear();
            }
            Screen::Confirm => {
                self.confirm_yes();
            }
            _ => {}
        }
        AppSignal::Continue
    }

    fn try_unlock(&mut self) {
        let Some(path) = self.vault_path.clone() else {
            self.unlock_error = Some("No vault selected".into());
            return;
        };
        // `mem::take` moves the plaintext into a secret and empties the buffer,
        // so the plain master password never lingers in `App`.
        let password = Secret::new(SecretString::from(std::mem::take(
            &mut self.unlock_password,
        )));
        self.unlock_error = None;
        self.pending = Some(PendingTask {
            kind: TaskKind::Unlock,
            started: Instant::now(),
            rx: tasks::spawn_unlock(path, password),
        });
    }

    fn load_entries(&mut self) {
        self.all_entries.clear();
        if let Some(vault) = &self.vault {
            for entry in vault.entries() {
                self.all_entries.push(EntryRow {
                    id: entry.id(),
                    name: entry.name().to_string(),
                    username: entry.username().map(|s| s.to_string()),
                    url: entry.url().map(|s| s.to_string()),
                });
            }
        }
        self.recompute_filtered_entries();
    }

    /// Rebuilds `entries` from the pristine `all_entries` list, applying the
    /// current search filter. Never mutates `all_entries`, so clearing or
    /// shortening the query restores every entry.
    fn recompute_filtered_entries(&mut self) {
        self.entries = if self.search_query.is_empty() {
            self.all_entries.clone()
        } else {
            let q = self.search_query.to_lowercase();
            self.all_entries
                .iter()
                .filter(|e| {
                    e.name.to_lowercase().contains(&q)
                        || e.username
                            .as_deref()
                            .map(|u| u.to_lowercase().contains(&q))
                            .unwrap_or(false)
                        || e.url
                            .as_deref()
                            .map(|u| u.to_lowercase().contains(&q))
                            .unwrap_or(false)
                })
                .cloned()
                .collect()
        };
        self.entry_list_selected = 0;
        self.entry_list_page = 0;
    }

    fn get_selected_entry_idx(&self) -> Option<usize> {
        if self.entries.is_empty() {
            None
        } else {
            Some(self.entry_list_selected.min(self.entries.len() - 1))
        }
    }

    fn handle_back(&mut self) -> AppSignal {
        match self.screen {
            Screen::Unlock => {
                self.screen = Screen::VaultList;
                self.unlock_password.zeroize();
            }
            Screen::EntryList if self.in_search => {
                self.search_query.clear();
                self.in_search = false;
                self.recompute_filtered_entries();
            }
            Screen::EntryList => {
                self.screen = Screen::VaultList;
                self.vault = None;
                self.master_password = None;
                self.locked = false;
                self.in_search = false;
                self.search_query.clear();
            }
            Screen::EntryDetail => {
                self.screen = Screen::EntryList;
                self.detail_entry_idx = None;
                self.show_password = false;
            }
            Screen::EntryForm => {
                self.back_from_form();
            }
            Screen::NewVault => {
                self.reset_new_vault_fields();
                self.screen = Screen::VaultList;
            }
            Screen::Generate => {
                self.screen = self.previous_screen.unwrap_or(Screen::VaultList);
                self.gen_result = None;
            }
            Screen::Locked => {
                self.screen = Screen::Unlock;
                self.unlock_password.clear();
            }
            Screen::Confirm => {
                self.screen = self.previous_screen.unwrap_or(Screen::EntryList);
                self.confirm_on_yes = None;
            }
            Screen::Help => {
                self.screen = self.previous_screen.unwrap_or(Screen::VaultList);
            }
            _ => {}
        }
        AppSignal::Continue
    }

    fn back_from_form(&mut self) {
        self.screen = self.previous_screen.unwrap_or(Screen::EntryList);
        self.reset_form_fields();
    }

    fn reset_form_fields(&mut self) {
        self.form_fields.password.zeroize();
        self.form_fields.password_confirm.zeroize();
        self.form_fields = FormFields::default();
        self.form_field_idx = 0;
    }

    fn reset_new_vault_fields(&mut self) {
        self.new_vault_fields.name.zeroize();
        self.new_vault_fields.password.zeroize();
        self.new_vault_fields.confirm.zeroize();
        self.new_vault_fields = NewVaultFields::default();
        self.new_vault_field_idx = 0;
    }

    fn handle_search(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList {
            self.in_search = true;
        }
        AppSignal::Continue
    }

    fn handle_add(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList {
            self.screen = Screen::EntryForm;
            self.previous_screen = Some(Screen::EntryList);
            self.form_mode = FormMode::Add;
            self.form_fields = FormFields::default();
            self.form_field_idx = 0;
        }
        AppSignal::Continue
    }

    fn handle_generate(&mut self) -> AppSignal {
        if self.screen == Screen::Generate {
            return AppSignal::Continue;
        }
        self.previous_screen = Some(self.screen);
        self.screen = Screen::Generate;
        self.gen_policy = GenPolicy::default();
        self.gen_result = None;
        AppSignal::Continue
    }

    fn handle_new_vault(&mut self) -> AppSignal {
        if self.screen == Screen::VaultList {
            self.previous_screen = Some(Screen::VaultList);
            self.screen = Screen::NewVault;
            self.reset_new_vault_fields();
        }
        AppSignal::Continue
    }

    fn handle_delete(&mut self) -> AppSignal {
        match self.screen {
            Screen::VaultList => {
                if let Some(vault) = self.vaults.get(self.vault_list_selected) {
                    self.confirm_message = format!("Delete vault '{}'?", vault.name);
                    self.confirm_on_yes = Some(ConfirmAction::DeleteVault);
                    self.previous_screen = Some(Screen::VaultList);
                    self.screen = Screen::Confirm;
                }
            }
            Screen::EntryList | Screen::EntryDetail => {
                let idx = self
                    .detail_entry_idx
                    .or_else(|| self.get_selected_entry_idx());
                if let Some(idx) = idx {
                    if let Some(entry) = self.entries.get(idx) {
                        self.confirm_message = format!("Delete entry '{}'?", entry.name);
                        self.confirm_on_yes = Some(ConfirmAction::DeleteEntry);
                        self.previous_screen = Some(self.screen);
                        self.screen = Screen::Confirm;
                    }
                }
            }
            _ => {}
        }
        AppSignal::Continue
    }

    fn handle_copy_password(&mut self) -> AppSignal {
        use crate::clipboard::copy_secret_to_clipboard;

        // On the generate screen, copy the freshly generated password (only if
        // one was actually generated — otherwise copy nothing).
        if self.screen == Screen::Generate {
            match &self.gen_result {
                Some(pwd) => match copy_secret_to_clipboard(pwd) {
                    Ok(()) => self.set_status("Password copied to clipboard (15s)"),
                    Err(_) => self.set_error("Clipboard unavailable"),
                },
                None => self.set_error("Generate a password first"),
            }
            return AppSignal::Continue;
        }

        self.copy_from_entry(
            |entry| copy_secret_to_clipboard(entry.password().expose_secret().as_str()),
            "Password copied to clipboard (15s)",
        );
        AppSignal::Continue
    }

    fn handle_copy_username(&mut self) -> AppSignal {
        self.copy_from_entry(
            |entry| {
                use crate::clipboard::copy_secret_to_clipboard;
                match entry.username() {
                    Some(user) => copy_secret_to_clipboard(user),
                    None => Ok(()),
                }
            },
            "Username copied to clipboard (15s)",
        );
        AppSignal::Continue
    }

    fn handle_copy_url(&mut self) -> AppSignal {
        self.copy_from_entry(
            |entry| {
                use crate::clipboard::copy_secret_to_clipboard;
                match entry.url() {
                    Some(url) => copy_secret_to_clipboard(url),
                    None => Ok(()),
                }
            },
            "URL copied to clipboard (15s)",
        );
        AppSignal::Continue
    }

    fn copy_from_entry(
        &mut self,
        f: impl FnOnce(&cista_core::Entry) -> anyhow::Result<()>,
        ok_msg: &str,
    ) {
        if self.screen == Screen::EntryDetail {
            if let Some(idx) = self.detail_entry_idx {
                if self.entries.get(idx).is_some() {
                    if let Some(vault) = &self.vault {
                        if let Some(entry) = vault.find_by_id(self.entries[idx].id) {
                            match f(entry) {
                                Ok(()) => self.set_status(ok_msg),
                                Err(_) => self.set_error("Clipboard unavailable"),
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_edit(&mut self) -> AppSignal {
        if let Screen::EntryDetail = self.screen {
            if let Some(idx) = self.detail_entry_idx {
                if self.entries.get(idx).is_some() {
                    if let Some(vault) = &self.vault {
                        if let Some(entry) = vault.find_by_id(self.entries[idx].id) {
                            self.form_mode = FormMode::Edit;
                            self.form_fields = FormFields {
                                name: entry.name().to_string(),
                                username: entry.username().unwrap_or("").to_string(),
                                password: String::new(),
                                password_confirm: String::new(),
                                url: entry.url().unwrap_or("").to_string(),
                                notes: entry
                                    .notes()
                                    .map(|n| n.expose_secret().as_str().to_string())
                                    .unwrap_or_default(),
                            };
                            self.form_field_idx = 0;
                            self.previous_screen = Some(Screen::EntryDetail);
                            self.screen = Screen::EntryForm;
                        }
                    }
                }
            }
        }
        AppSignal::Continue
    }

    fn handle_reveal(&mut self) -> AppSignal {
        match self.screen {
            Screen::EntryDetail => self.show_password = !self.show_password,
            // Space toggles the focused option on the generate screen.
            Screen::Generate => {
                let option = GenOption::ALL[self.gen_selected];
                self.toggle_gen_option(option);
            }
            _ => {}
        }
        AppSignal::Continue
    }

    fn handle_tab_next(&mut self) -> AppSignal {
        match self.screen {
            Screen::EntryForm => {
                self.form_field_idx = (self.form_field_idx + 1) % 6;
            }
            Screen::NewVault => {
                self.new_vault_field_idx = (self.new_vault_field_idx + 1) % 3;
            }
            _ => {}
        }
        AppSignal::Continue
    }

    fn handle_tab_prev(&mut self) -> AppSignal {
        match self.screen {
            Screen::EntryForm => {
                self.form_field_idx = (self.form_field_idx + 5) % 6;
            }
            Screen::NewVault => {
                self.new_vault_field_idx = (self.new_vault_field_idx + 2) % 3;
            }
            _ => {}
        }
        AppSignal::Continue
    }

    fn handle_save(&mut self) -> AppSignal {
        match self.screen {
            Screen::EntryForm => self.save_entry(),
            Screen::NewVault => self.create_vault(),
            _ => {}
        }
        AppSignal::Continue
    }

    fn confirm_yes(&mut self) {
        if let Some(action) = self.confirm_on_yes {
            match action {
                ConfirmAction::DeleteEntry => {
                    let idx = self
                        .detail_entry_idx
                        .or_else(|| self.get_selected_entry_idx());
                    if let Some(idx) = idx {
                        if let Some(entry) = self.entries.get(idx).cloned() {
                            if let Some(vault) = &mut self.vault {
                                if vault.remove_by_id(entry.id).is_ok() {
                                    self.load_entries();
                                    self.start_save_task(TaskKind::SaveEntryDelete);
                                }
                            }
                        }
                    }
                }
                ConfirmAction::DeleteVault => {
                    if let Some(vault) = self.vaults.get(self.vault_list_selected) {
                        let _ = std::fs::remove_file(&vault.path);
                        self.load_vaults();
                        self.set_status("Vault deleted");
                    }
                }
            }
        }
        self.confirm_on_yes = None;
        self.screen = self.previous_screen.unwrap_or(Screen::EntryList);
        self.detail_entry_idx = None;
        self.show_password = false;
    }

    fn create_vault(&mut self) {
        let name = self.new_vault_fields.name.trim().to_string();
        if name.is_empty() {
            self.set_error("Vault name is required");
            self.reset_new_vault_fields();
            return;
        }
        if self.new_vault_fields.password.is_empty() {
            self.set_error("Password is required");
            self.reset_new_vault_fields();
            return;
        }
        if self.new_vault_fields.password != self.new_vault_fields.confirm {
            self.set_error("Passwords do not match");
            self.reset_new_vault_fields();
            return;
        }

        let path = match cista_core::paths::vaults_dir() {
            Ok(dir) => dir.join(format!("{name}.cista")),
            Err(e) => {
                self.set_error(&e.to_string());
                self.reset_new_vault_fields();
                return;
            }
        };

        let key = std::mem::take(&mut self.new_vault_fields.password);
        self.pending = Some(PendingTask {
            kind: TaskKind::CreateVault,
            started: Instant::now(),
            rx: tasks::spawn_create_vault(name, path, Secret::new(SecretString::from(key))),
        });
    }

    fn save_entry(&mut self) {
        use cista_core::Entry;

        let name = self.form_fields.name.clone();
        let username = self.form_fields.username.clone();
        let notes = self.form_fields.notes.clone();
        let url = self.form_fields.url.clone();
        let password_raw = std::mem::take(&mut self.form_fields.password);
        let confirm_raw = std::mem::take(&mut self.form_fields.password_confirm);

        if name.trim().is_empty() {
            self.set_error("Service name is required");
            return;
        }

        if !password_raw.is_empty() && password_raw != confirm_raw {
            self.set_error("Passwords do not match");
            return;
        }
        // A confirm value with no password (or a password with no confirm) is
        // always a mismatch.
        if confirm_raw.is_empty() && !password_raw.is_empty() {
            self.set_error("Re-type the password to confirm");
            return;
        }

        let password = if password_raw.is_empty() {
            if self.form_mode == FormMode::Add {
                self.set_error("Password is required for new entries");
                return;
            }
            None
        } else {
            // Move the plaintext into a secret so no plain copy remains behind.
            Some(Secret::new(SecretString::from(password_raw)))
        };

        if let Some(vault) = &mut self.vault {
            let result: anyhow::Result<&mut Vault> = match self.form_mode {
                FormMode::Add => {
                    let entry = Entry::new(
                        name.trim().to_string(),
                        if username.is_empty() {
                            None
                        } else {
                            Some(username.clone())
                        },
                        password.unwrap(),
                        if url.is_empty() {
                            None
                        } else {
                            Some(url.clone())
                        },
                        if notes.is_empty() {
                            None
                        } else {
                            Some(notes.clone())
                        },
                    );
                    entry.map_err(anyhow::Error::from).map(|e| {
                        vault.add_entry(e);
                        vault
                    })
                }
                FormMode::Edit => {
                    if let Some(idx) = self.detail_entry_idx {
                        let entry_id = self.entries[idx].id;
                        if let Some(entry) = vault.find_by_id_mut(entry_id) {
                            entry.rename(name.trim().to_string()).ok();
                            entry.set_username(if username.is_empty() {
                                None
                            } else {
                                Some(username.clone())
                            });
                            if let Some(p) = password {
                                entry.set_password(p);
                            }
                            entry.set_url(if url.is_empty() {
                                None
                            } else {
                                Some(url.clone())
                            });
                            entry.set_notes(if notes.is_empty() {
                                None
                            } else {
                                Some(notes.clone())
                            });
                            Ok(vault)
                        } else {
                            Err(anyhow::anyhow!("Entry not found"))
                        }
                    } else {
                        Err(anyhow::anyhow!("No entry selected"))
                    }
                }
            };

            match result {
                Ok(_) => {
                    let kind = match self.form_mode {
                        FormMode::Add => TaskKind::SaveEntryAdd,
                        FormMode::Edit => TaskKind::SaveEntryEdit,
                    };
                    self.start_save_task(kind);
                }
                Err(e) => self.set_error(&e.to_string()),
            }
        }
    }

    /// Snapshot the vault and push the persist (Argon2 seal + write) to a
    /// worker thread. The in-memory change was already applied synchronously,
    /// so the UI stays responsive and the modal just reports the outcome.
    fn start_save_task(&mut self, kind: TaskKind) {
        let (Some(vault), Some(path), Some(password)) = (
            self.vault.clone(),
            self.vault_path.clone(),
            self.master_password.clone(),
        ) else {
            self.set_error("No open vault");
            return;
        };
        self.pending = Some(PendingTask {
            kind,
            started: Instant::now(),
            rx: tasks::spawn_save_vault(path, vault, password),
        });
    }

    fn do_generate(&mut self) {
        use cista_core::password_gen::{generate_password, PasswordPolicy};

        let policy = PasswordPolicy {
            length: self.gen_policy.length,
            include_lowercase: self.gen_policy.include_lowercase,
            include_uppercase: self.gen_policy.include_uppercase,
            include_digits: self.gen_policy.include_digits,
            include_symbols: self.gen_policy.include_symbols,
            exclude_ambiguous: self.gen_policy.exclude_ambiguous,
        };

        match generate_password(&policy) {
            Ok(pwd) => {
                self.gen_result = Some(pwd);
            }
            Err(e) => self.set_error(&e.to_string()),
        }
    }

    fn set_status(&mut self, msg: &str) {
        self.status_message = Some(StatusNotice {
            text: msg.into(),
            at: Instant::now(),
        });
        self.status_error = None;
    }

    fn set_error(&mut self, msg: &str) {
        self.status_error = Some(StatusNotice {
            text: msg.into(),
            at: Instant::now(),
        });
        self.status_message = None;
    }

    pub fn check_auto_lock(&mut self) {
        if self.pending.is_some() {
            return;
        }
        if !self.locked
            && self.auto_lock_seconds > 0
            && self.last_activity.elapsed() >= Duration::from_secs(self.auto_lock_seconds)
        {
            self.lock_vault();
        }
    }
}
