use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Color;
use ratatui::Frame;
use std::path::{Path, PathBuf};
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use crate::keys::{Action, ActionMapper, KeyBindings};
use crate::screens::*;
use crate::tasks::{self, PendingTask, TaskKind, TaskResult};
use crate::widgets::*;
use cista_core::config::Config;
use cista_core::{EntryType, SecretString, Vault};
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
    Dashboard,
    ChangePassword,
}

pub struct App {
    pub screen: Screen,
    pub previous_screen: Option<Screen>,

    /// Vault list screen.
    pub vault_list: VaultListState,

    /// Unlock / password screen.
    pub unlock: UnlockState,

    /// Fingerprint quick-unlock state for the unlock screen.
    pub quick_unlock: QuickUnlockState,

    /// Session (open vault + auto-lock) state.
    pub session: SessionState,

    /// Entry list (dashboard) screen.
    pub entry_list: EntryListState,

    /// Entry detail screen.
    pub entry_detail: EntryDetailState,

    /// Entry form (add/edit) screen.
    pub entry_form: EntryFormState,

    /// New vault form screen.
    pub new_vault: NewVaultState,

    /// Password generator screen.
    pub generate: GenerateState,

    /// Confirm dialog state.
    pub confirm: ConfirmState,

    /// Help screen state.
    pub help: HelpState,

    /// Health dashboard screen.
    pub dashboard: DashboardState,

    /// Change-master-password flow.
    pub change_password: ChangePasswordState,

    // Input
    mapper: ActionMapper,

    // Runtime
    pub status_message: Option<StatusNotice>,
    pub status_error: Option<StatusNotice>,
    pub pending: Option<PendingTask>,
}

pub struct VaultListState {
    pub vaults: Vec<VaultInfo>,
    pub selected: usize,
}

pub struct UnlockState {
    pub password: String,
    pub error: Option<String>,
    /// What the master-password screen is being used for: unlocking to open a
    /// vault, or confirming a password before deleting a vault.
    pub mode: UnlockMode,
}

/// Fingerprint quick-unlock affordance. The master password is still the real
/// gate; the fingerprint only releases the copy stored in the OS keyring.
pub struct QuickUnlockState {
    pub enabled: bool,
    pub detect_cmd: String,
    pub verify_cmd: String,
    /// True once the availability probe has run (reader present + secret
    /// stored), so the unlock screen only offers a finger it can honour.
    pub checked: bool,
    pub available: bool,
    pub has_secret: bool,
}

pub struct SessionState {
    pub vault_path: Option<PathBuf>,
    pub vault: Option<Vault>,
    pub ui_state: cista_core::UiState,
    pub master_password: Option<Secret<SecretString>>,
    pub locked: bool,
    pub last_activity: Instant,
    pub auto_lock_seconds: u64,
}

pub struct EntryListState {
    pub all_entries: Vec<EntryRow>,
    pub entries: Vec<EntryRow>,
    pub selected: usize,
    pub page: usize,
    pub per_page: usize,
    pub search_query: String,
    pub in_search: bool,
    pub sort_mode: SortMode,
}

pub struct EntryDetailState {
    pub entry_idx: Option<usize>,
    pub show_password: bool,
}

pub struct EntryFormState {
    pub mode: FormMode,
    pub fields: FormFields,
    pub field_idx: usize,
}

pub struct NewVaultState {
    pub fields: NewVaultFields,
    pub field_idx: usize,
}

pub struct GenerateState {
    pub policy: GenPolicy,
    pub selected: usize,
    pub result: Option<String>,
    /// True when the generator was opened from the entry form. In that mode a
    /// second Ctrl+g (or Enter after generating) drops the generated password
    /// into the form's password field and returns to the form.
    pub from_form: bool,
    /// Screen the generator was opened from, so Esc/apply always lands back on
    /// it. Kept separate from `previous_screen`, which the entry form relies on
    /// (its own "back" destination), so opening the generator from the form
    /// doesn't clobber the form's exit target.
    pub prev_screen: Option<Screen>,
}

pub struct ConfirmState {
    pub message: String,
    pub on_yes: Option<ConfirmAction>,
}

pub struct HelpState {
    pub scroll: u16,
}

pub struct DashboardState {
    /// Entry ids ordered by health ascending (weakest first). Derived from
    /// `all_entries` when the dashboard is opened.
    pub rows: Vec<uuid::Uuid>,
    /// Index into `rows` of the currently selected entry.
    pub selected: usize,
    /// Top scroll offset for the entry list.
    pub scroll: u16,
}

/// Which field of the change-master-password flow is being filled in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeStage {
    Current,
    New,
    Confirm,
}

pub struct ChangePasswordState {
    pub stage: ChangeStage,
    pub current: String,
    pub new: String,
    pub confirm: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VaultInfo {
    pub name: String,
    pub path: PathBuf,
    pub last_opened: Option<String>,
    pub entry_count: Option<usize>,
    pub size: u64,
    pub created: Option<std::time::SystemTime>,
}

#[derive(Debug, Clone)]
pub struct EntryRow {
    pub id: uuid::Uuid,
    pub name: String,
    pub username: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub favorite: bool,
    pub icon: String,
    pub entry_type: EntryType,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
    pub health: cista_core::health::Health,
}

/// How the entry list is ordered. Pressing `o` in the list cycles through
/// these modes (forward through the paired direction toggles).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    NameAsc,
    NameDesc,
    UpdatedDesc,
    UpdatedAsc,
    CreatedDesc,
    CreatedAsc,
}

impl SortMode {
    /// Human-readable label shown in the table title / footer.
    pub fn label(self) -> &'static str {
        match self {
            SortMode::NameAsc => "Name ↑",
            SortMode::NameDesc => "Name ↓",
            SortMode::UpdatedDesc => "Modified ↓",
            SortMode::UpdatedAsc => "Modified ↑",
            SortMode::CreatedDesc => "Created ↓",
            SortMode::CreatedAsc => "Created ↑",
        }
    }

    /// Next mode in the cycling order.
    pub fn next(self) -> Self {
        match self {
            SortMode::NameAsc => SortMode::NameDesc,
            SortMode::NameDesc => SortMode::UpdatedDesc,
            SortMode::UpdatedDesc => SortMode::UpdatedAsc,
            SortMode::UpdatedAsc => SortMode::CreatedDesc,
            SortMode::CreatedDesc => SortMode::CreatedAsc,
            SortMode::CreatedAsc => SortMode::NameAsc,
        }
    }
}

/// Case-insensitive name comparison for sorting.
fn name_cmp(a: &EntryRow, b: &EntryRow) -> std::cmp::Ordering {
    a.name.to_lowercase().cmp(&b.name.to_lowercase())
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
    pub icon: String,
    pub entry_type: EntryType,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockMode {
    Open,
    Delete,
}

impl App {
    pub fn new(keybindings: KeyBindings) -> Self {
        let mapper = ActionMapper::new(keybindings);
        let config = Config::load().unwrap_or_default();
        let auto_lock_seconds = config.auto_lock_seconds;
        let quick_unlock_cfg = config.quick_unlock;
        let mut app = Self {
            screen: Screen::VaultList,
            previous_screen: None,
            vault_list: VaultListState {
                vaults: Vec::new(),
                selected: 0,
            },
            unlock: UnlockState {
                password: String::new(),
                error: None,
                mode: UnlockMode::Open,
            },
            quick_unlock: QuickUnlockState {
                enabled: quick_unlock_cfg.enabled,
                detect_cmd: quick_unlock_cfg.detect_cmd,
                verify_cmd: quick_unlock_cfg.verify_cmd,
                checked: false,
                available: false,
                has_secret: false,
            },
            session: SessionState {
                vault_path: None,
                vault: None,
                ui_state: cista_core::UiState::new(),
                master_password: None,
                locked: false,
                last_activity: Instant::now(),
                auto_lock_seconds,
            },
            entry_list: EntryListState {
                all_entries: Vec::new(),
                entries: Vec::new(),
                selected: 0,
                page: 0,
                per_page: 20,
                search_query: String::new(),
                in_search: false,
                sort_mode: SortMode::NameAsc,
            },
            entry_detail: EntryDetailState {
                entry_idx: None,
                show_password: false,
            },
            entry_form: EntryFormState {
                mode: FormMode::Add,
                fields: FormFields::default(),
                field_idx: 0,
            },
            new_vault: NewVaultState {
                fields: NewVaultFields::default(),
                field_idx: 0,
            },
            generate: GenerateState {
                policy: GenPolicy::default(),
                selected: 0,
                result: None,
                from_form: false,
                prev_screen: None,
            },
            confirm: ConfirmState {
                message: String::new(),
                on_yes: None,
            },
            help: HelpState { scroll: 0 },
            dashboard: DashboardState {
                rows: Vec::new(),
                selected: 0,
                scroll: 0,
            },
            change_password: ChangePasswordState {
                stage: ChangeStage::Current,
                current: String::new(),
                new: String::new(),
                confirm: String::new(),
                error: None,
            },
            mapper,
            status_message: None,
            status_error: None,
            pending: None,
        };
        app.load_vaults();
        app
    }

    fn load_vaults(&mut self) {
        self.vault_list.vaults.clear();
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
                            let created = std::fs::metadata(entry.path())
                                .and_then(|m| m.created())
                                .ok();
                            self.vault_list.vaults.push(VaultInfo {
                                name,
                                path: entry.path(),
                                last_opened,
                                entry_count,
                                size,
                                created,
                            });
                        }
                    }
                }
            }
        }
        self.vault_list.vaults.sort_by(|a, b| a.name.cmp(&b.name));
        self.vault_list.selected = 0;
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
            Screen::Dashboard => draw_dashboard(f, self),
            Screen::ChangePassword => draw_change_password(f, self),
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
                    self.persist_quick_unlock_secret(&path, &password);
                    self.unlock_opened(path, vault, password, "Vault unlocked");
                }
                TaskResult::Unlock { result: Err(_), .. } => {
                    self.unlock.error = Some("Invalid master password".into());
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
            TaskKind::DeleteVault => match result {
                TaskResult::DeleteVault { result: Ok(()) } => {
                    self.unlock.mode = UnlockMode::Open;
                    if let Some(path) = &self.session.vault_path {
                        cista_core::UiState::remove_for_vault(path);
                        if let Some(account) = crate::quick_unlock::vault_account(path) {
                            crate::quick_unlock::delete_secret(&account);
                        }
                    }
                    self.load_vaults();
                    self.screen = Screen::VaultList;
                    self.set_status("Vault deleted");
                }
                TaskResult::DeleteVault { result: Err(_) } => {
                    self.unlock.error = Some("Invalid master password".into());
                    self.unlock.password.clear();
                    self.screen = Screen::Unlock;
                }
                _ => {}
            },
            TaskKind::VerifyPassword => match result {
                TaskResult::VerifyPassword { result: Ok(()) } => {
                    self.change_password.current.clear();
                    self.change_password.stage = ChangeStage::New;
                    self.change_password.error = None;
                }
                TaskResult::VerifyPassword { result: Err(_) } => {
                    self.change_password.current.clear();
                    self.change_password.error = Some("Invalid master password".into());
                }
                _ => {}
            },
            TaskKind::ChangePassword => match result {
                TaskResult::ChangePassword {
                    result: Ok(()),
                    password,
                } => {
                    if let Some(path) = self.session.vault_path.clone() {
                        self.persist_quick_unlock_secret(&path, &password);
                    }
                    self.session.master_password = Some(password);
                    let target = self.previous_screen.unwrap_or(Screen::EntryList);
                    self.reset_change_password();
                    self.screen = target;
                    self.set_status("Master password changed");
                }
                TaskResult::ChangePassword { result: Err(e), .. } => {
                    self.set_error(&format!("Failed to change master password: {e}"));
                    self.cancel_change_password();
                }
                _ => {}
            },
            TaskKind::QuickUnlockCheck => if let TaskResult::QuickUnlockCheck {
                available,
                has_secret,
            } = result
            {
                self.quick_unlock.checked = true;
                self.quick_unlock.available = available;
                self.quick_unlock.has_secret = has_secret;
            },
            TaskKind::QuickUnlockVerify => {
                if let TaskResult::QuickUnlockVerify {
                    path,
                    password,
                    result,
                } = result
                {
                    match (password, result) {
                        (Some(password), Ok(vault)) => {
                            self.unlock_opened(path, vault, password, "Vault unlocked (fingerprint)");
                        }
                        (None, _) => {
                            self.unlock.error = Some("Fingerprint not accepted".into());
                        }
                        (Some(_), Err(_)) => {
                            self.unlock.error = Some(
                                "Fingerprint accepted, but the stored password is stale; type it here"
                                    .into(),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Shared post-unlock steps for both the typed-password and the
    /// fingerprint paths.
    fn unlock_opened(
        &mut self,
        path: PathBuf,
        vault: Vault,
        password: Secret<SecretString>,
        status: &str,
    ) {
        let _ = cista_core::config::record_opened(&path);
        self.session.vault_path = Some(path.clone());
        self.session.ui_state = cista_core::UiState::load_for_vault(&path);
        self.session.vault = Some(vault);
        self.session.master_password = Some(password);
        self.session.locked = false;
        self.session.last_activity = Instant::now();
        self.load_entries();
        self.screen = Screen::EntryList;
        self.set_status(status);
    }

    /// After a successful (typed-password or fingerprint) unlock, cache the
    /// master password in the OS keyring so quick unlock can use it later.
    /// Best-effort: without a Secret Service this silently degrades.
    fn persist_quick_unlock_secret(&mut self, path: &Path, password: &Secret<SecretString>) {
        if !self.quick_unlock.enabled {
            return;
        }
        let Some(account) = crate::quick_unlock::vault_account(path) else {
            return;
        };
        if crate::quick_unlock::store_secret(&account, password) {
            self.quick_unlock.checked = true;
            self.quick_unlock.has_secret = true;
        } else {
            crate::log::log_error(&format!("quick unlock: could not store secret for {path:?}"));
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

        // Soft, dimmed backdrop: a dark grey tint rather than a solid black
        // wall, so the running screen stays faintly visible behind the modal
        // instead of being completely blanked out.
        let backdrop = Block::default().style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        );
        f.render_widget(backdrop, f.area());

        let area = centered_rect(46, 18, f.area());
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
            Screen::Unlock => Some(&mut self.unlock.password),
            Screen::EntryList if self.entry_list.in_search => {
                Some(&mut self.entry_list.search_query)
            }
            Screen::EntryForm => Some(match self.entry_form.field_idx {
                0 => &mut self.entry_form.fields.name,
                1 => &mut self.entry_form.fields.username,
                2 => &mut self.entry_form.fields.password,
                3 => &mut self.entry_form.fields.password_confirm,
                4 => &mut self.entry_form.fields.url,
                5 => &mut self.entry_form.fields.notes,
                6 => &mut self.entry_form.fields.icon,
                _ => &mut self.entry_form.fields.name,
            }),
            Screen::NewVault => Some(match self.new_vault.field_idx {
                0 => &mut self.new_vault.fields.name,
                1 => &mut self.new_vault.fields.password,
                _ => &mut self.new_vault.fields.confirm,
            }),
            Screen::ChangePassword => Some(match self.change_password.stage {
                ChangeStage::Current => &mut self.change_password.current,
                ChangeStage::New => &mut self.change_password.new,
                ChangeStage::Confirm => &mut self.change_password.confirm,
            }),
            _ => None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AppSignal {
        self.session.last_activity = Instant::now();

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
            if self.entry_list.in_search {
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

    /// Dispatch a key action to the active screen. Centralizes the
    /// `match self.screen` that used to be repeated across every `handle_*`,
    /// so each screen's behaviour lives in one `update_*` method.
    fn handle_action(&mut self, action: Action) -> AppSignal {
        match self.screen {
            Screen::VaultList => self.update_vault_list(action),
            Screen::Unlock => self.update_unlock(action),
            Screen::EntryList => self.update_entry_list(action),
            Screen::EntryDetail => self.update_entry_detail(action),
            Screen::EntryForm => self.update_entry_form(action),
            Screen::NewVault => self.update_new_vault(action),
            Screen::Generate => self.update_generate(action),
            Screen::Locked => self.update_locked(action),
            Screen::Confirm => self.update_confirm(action),
            Screen::Help => self.update_help(action),
            Screen::Dashboard => self.update_dashboard(action),
            Screen::ChangePassword => self.update_change_password(action),
        }
    }

    fn update_vault_list(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Generate => self.handle_generate(),
            Action::Up => {
                if self.vault_list.selected > 0 {
                    self.vault_list.selected -= 1;
                }
                AppSignal::Continue
            }
            Action::Down => {
                if self.vault_list.selected + 1 < self.vault_list.vaults.len() {
                    self.vault_list.selected += 1;
                }
                AppSignal::Continue
            }
            Action::Enter => self.handle_enter(),
            Action::Back => self.handle_back(),
            Action::NewVault => self.handle_new_vault(),
            Action::Delete => self.handle_delete(),
            _ => AppSignal::Continue,
        }
    }

    fn update_unlock(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Generate => self.handle_generate(),
            Action::Enter => self.handle_enter(),
            Action::Back => self.handle_back(),
            Action::QuickUnlock => self.quick_unlock_action(),
            _ => AppSignal::Continue,
        }
    }

    fn update_entry_list(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Lock => self.handle_lock(),
            Action::Generate => self.handle_generate(),
            Action::Up => {
                if self.entry_list.selected > 0 {
                    self.entry_list.selected -= 1;
                    self.adjust_page();
                }
                AppSignal::Continue
            }
            Action::Down => {
                if self.entry_list.selected + 1 < self.entry_list.entries.len() {
                    self.entry_list.selected += 1;
                    self.adjust_page();
                }
                AppSignal::Continue
            }
            Action::PageUp => self.handle_page_up(),
            Action::PageDown => self.handle_page_down(),
            Action::Home => self.handle_home(),
            Action::End => self.handle_end(),
            Action::Enter => self.handle_enter(),
            Action::Back => self.handle_back(),
            Action::Search => self.handle_search(),
            Action::Add => self.handle_add(),
            Action::Delete => self.handle_delete(),
            Action::CopyPassword => self.handle_copy_password(),
            Action::CopyUsername => self.handle_copy_username(),
            Action::CopyUrl => self.handle_copy_url(),
            Action::Sort => self.handle_sort(),
            Action::Dashboard => self.handle_dashboard(),
            Action::ChangePassword => self.handle_change_password(),
            Action::ToggleFavorite => self.handle_toggle_favorite(),
            _ => AppSignal::Continue,
        }
    }

    fn update_entry_detail(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Lock => self.handle_lock(),
            Action::Generate => self.handle_generate(),
            Action::Enter => self.handle_enter(),
            Action::Back => self.handle_back(),
            Action::Delete => self.handle_delete(),
            Action::CopyPassword => self.handle_copy_password(),
            Action::CopyUsername => self.handle_copy_username(),
            Action::CopyUrl => self.handle_copy_url(),
            Action::Edit => self.handle_edit(),
            Action::Reveal => self.handle_reveal(),
            _ => AppSignal::Continue,
        }
    }

    fn update_entry_form(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Generate => self.handle_generate(),
            Action::Up | Action::TabPrev => self.handle_tab_prev(),
            Action::Down | Action::TabNext => self.handle_tab_next(),
            Action::Enter => self.handle_enter(),
            Action::Back => self.handle_back(),
            Action::Save => self.handle_save(),
            _ => AppSignal::Continue,
        }
    }

    fn update_new_vault(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Generate => self.handle_generate(),
            Action::Up | Action::TabPrev => self.handle_tab_prev(),
            Action::Down | Action::TabNext => self.handle_tab_next(),
            Action::Enter => self.handle_enter(),
            Action::Back => self.handle_back(),
            Action::Save => self.handle_save(),
            _ => AppSignal::Continue,
        }
    }

    fn update_generate(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Generate => self.handle_generate(),
            Action::Up => {
                self.generate.selected = self.generate.selected.saturating_sub(1);
                AppSignal::Continue
            }
            Action::Down => {
                self.generate.selected = (self.generate.selected + 1).min(GenOption::ALL.len() - 1);
                AppSignal::Continue
            }
            Action::Left => self.handle_left(),
            Action::Right => self.handle_right(),
            Action::Enter => self.handle_enter(),
            Action::Back => self.handle_back(),
            Action::Reroll => self.handle_reroll(),
            Action::Reveal => self.handle_reveal(),
            Action::CopyPassword => self.handle_copy_password(),
            _ => AppSignal::Continue,
        }
    }

    fn update_locked(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Generate => self.handle_generate(),
            Action::Enter => self.handle_enter(),
            Action::Back => self.handle_back(),
            _ => AppSignal::Continue,
        }
    }

    fn update_confirm(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Generate => self.handle_generate(),
            Action::Enter => self.handle_enter(),
            Action::Back => self.handle_back(),
            _ => AppSignal::Continue,
        }
    }

    fn update_help(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Generate => self.handle_generate(),
            Action::Up => {
                self.help.scroll = self.help.scroll.saturating_sub(1);
                AppSignal::Continue
            }
            Action::Down => {
                self.help.scroll = self.help.scroll.saturating_add(1);
                AppSignal::Continue
            }
            Action::PageUp => {
                self.help.scroll = self.help.scroll.saturating_sub(15);
                AppSignal::Continue
            }
            Action::PageDown => {
                self.help.scroll = self.help.scroll.saturating_add(15);
                AppSignal::Continue
            }
            Action::Home => {
                self.help.scroll = 0;
                AppSignal::Continue
            }
            Action::End => {
                self.help.scroll = u16::MAX;
                AppSignal::Continue
            }
            Action::Back => self.handle_back(),
            _ => AppSignal::Continue,
        }
    }

    fn update_dashboard(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Generate => self.handle_generate(),
            Action::Up => {
                self.dashboard.selected = self.dashboard.selected.saturating_sub(1);
                AppSignal::Continue
            }
            Action::Down => {
                if !self.dashboard.rows.is_empty()
                    && self.dashboard.selected + 1 < self.dashboard.rows.len()
                {
                    self.dashboard.selected += 1;
                }
                AppSignal::Continue
            }
            Action::PageUp => {
                self.dashboard.selected = self.dashboard.selected.saturating_sub(15);
                AppSignal::Continue
            }
            Action::PageDown => {
                if !self.dashboard.rows.is_empty() {
                    self.dashboard.selected = (self.dashboard.selected + 15)
                        .min(self.dashboard.rows.len() - 1);
                }
                AppSignal::Continue
            }
            Action::Home => {
                self.dashboard.selected = 0;
                AppSignal::Continue
            }
            Action::End => {
                self.dashboard.selected = self.dashboard.rows.len().saturating_sub(1);
                AppSignal::Continue
            }
            Action::Enter => self.open_dashboard_detail(),
            Action::Back => self.handle_back(),
            _ => AppSignal::Continue,
        }
    }

    fn open_dashboard_detail(&mut self) -> AppSignal {
        let Some(id) = self.dashboard.rows.get(self.dashboard.selected).copied() else {
            return AppSignal::Continue;
        };
        if let Some(idx) = self
            .entry_list
            .entries
            .iter()
            .position(|e| e.id == id)
        {
            self.entry_detail.entry_idx = Some(idx);
            self.entry_detail.show_password = false;
            self.screen = Screen::EntryDetail;
        }
        AppSignal::Continue
    }

    fn update_change_password(&mut self, action: Action) -> AppSignal {
        match action {
            Action::Quit => AppSignal::Quit,
            Action::Help => self.handle_help(),
            Action::Generate => self.handle_generate(),
            Action::Enter => self.advance_change_password(),
            Action::Back => self.cancel_change_password(),
            _ => AppSignal::Continue,
        }
    }

    /// Entry point from the entry list: opens the change-master-password flow
    /// on top of the current screen.
    fn handle_change_password(&mut self) -> AppSignal {
        if self.session.vault.is_none() {
            return AppSignal::Continue;
        }
        self.previous_screen = Some(self.screen);
        self.reset_change_password();
        self.screen = Screen::ChangePassword;
        AppSignal::Continue
    }

    fn reset_change_password(&mut self) {
        self.change_password.stage = ChangeStage::Current;
        self.change_password.current.clear();
        self.change_password.new.clear();
        self.change_password.confirm.clear();
        self.change_password.error = None;
    }

    fn advance_change_password(&mut self) -> AppSignal {
        match self.change_password.stage {
            ChangeStage::Current => {
                if self.change_password.current.is_empty() {
                    self.change_password.error = Some("Enter your current master password".into());
                    return AppSignal::Continue;
                }
                let Some(path) = self.session.vault_path.clone() else {
                    return AppSignal::Continue;
                };
                // `mem::take` so the plaintext never lingers in the state.
                let password = Secret::new(SecretString::from(std::mem::take(
                    &mut self.change_password.current,
                )));
                self.pending = Some(PendingTask {
                    kind: TaskKind::VerifyPassword,
                    started: Instant::now(),
                    rx: tasks::spawn_verify_password(path, password),
                });
            }
            ChangeStage::New => {
                if self.change_password.new.is_empty() {
                    self.change_password.error = Some("New master password is required".into());
                    return AppSignal::Continue;
                }
                self.change_password.stage = ChangeStage::Confirm;
                self.change_password.error = None;
            }
            ChangeStage::Confirm => {
                if self.change_password.new != self.change_password.confirm {
                    // Back to the new-password step; both buffers are cleared so
                    // no partial secret lingers.
                    self.change_password.new.clear();
                    self.change_password.confirm.clear();
                    self.change_password.stage = ChangeStage::New;
                    self.change_password.error = Some("Passwords do not match".into());
                    return AppSignal::Continue;
                }
                self.finish_change_password();
            }
        }
        AppSignal::Continue
    }

    /// Re-seals the vault with the new master password in a background task.
    fn finish_change_password(&mut self) {
        let Some(path) = self.session.vault_path.clone() else {
            return;
        };
        let Some(vault) = self.session.vault.clone() else {
            return;
        };
        let new_password = Secret::new(SecretString::from(std::mem::take(
            &mut self.change_password.confirm,
        )));
        self.change_password.new.clear();
        self.pending = Some(PendingTask {
            kind: TaskKind::ChangePassword,
            started: Instant::now(),
            rx: tasks::spawn_change_password(path, vault, new_password),
        });
    }

    fn cancel_change_password(&mut self) -> AppSignal {
        let target = self.previous_screen.unwrap_or(Screen::EntryList);
        self.reset_change_password();
        self.screen = target;
        AppSignal::Continue
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
        self.session.vault = None;
        self.session.master_password = None;
        self.session.vault_path = None;
        self.session.ui_state = cista_core::UiState::new();
        self.entry_list.all_entries.clear();
        self.entry_list.entries.clear();
        self.session.locked = true;
        self.screen = Screen::Locked;
        self.entry_list.in_search = false;
        self.entry_list.search_query.clear();
        self.reset_form_fields();
    }

    /// Left/Right move across the focused generate option. For `Length` the
    /// value changes; for the boolean categories it toggles the state.
    fn handle_left(&mut self) -> AppSignal {
        if self.screen == Screen::Generate {
            let option = GenOption::ALL[self.generate.selected];
            match option {
                GenOption::Length => {
                    self.generate.policy.length =
                        self.generate.policy.length.saturating_sub(1).max(4);
                }
                _ => self.toggle_gen_option(option),
            }
        }
        AppSignal::Continue
    }

    fn handle_right(&mut self) -> AppSignal {
        if self.screen == Screen::Generate {
            let option = GenOption::ALL[self.generate.selected];
            match option {
                GenOption::Length => {
                    self.generate.policy.length = (self.generate.policy.length + 1).min(128);
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

    /// Cycle the entry list sort mode ('o'). Resets to the first page so the
    /// newly ordered list is seen from its start.
    fn handle_sort(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList {
            self.entry_list.sort_mode = self.entry_list.sort_mode.next();
            let label = self.entry_list.sort_mode.label();
            self.recompute_filtered_entries();
            self.set_status(&format!("Sorted by {label}"));
        }
        AppSignal::Continue
    }

    /// Toggle the favourite flag on the selected entry ('f'). Updates the
    /// in-memory rows and the plaintext UI state, then persists metadata.
    fn handle_toggle_favorite(&mut self) -> AppSignal {
        if self.screen != Screen::EntryList {
            return AppSignal::Continue;
        }
        let Some(idx) = self.get_selected_entry_idx() else {
            return AppSignal::Continue;
        };
        let entry_id = self.entry_list.entries[idx].id;
        let new_val = self.session.ui_state.toggle_favourite(entry_id);

        // Update both the pristine list and the visible list.
        for e in self.entry_list.all_entries.iter_mut() {
            if e.id == entry_id {
                e.favorite = new_val;
            }
        }
        for e in self.entry_list.entries.iter_mut() {
            if e.id == entry_id {
                e.favorite = new_val;
            }
        }

        // Persist the plaintext metadata (fast, no re-seal).
        self.save_ui_state();

        // Keep favorites at the top.
        self.recompute_filtered_entries();
        self.set_status(if new_val {
            "Marked as favourite"
        } else {
            "Removed from favourites"
        });
        AppSignal::Continue
    }

    /// Persist the current UI state to the XDG state directory. Safe to call
    /// even when no vault is open (it becomes a no-op).
    fn save_ui_state(&self) {
        if let Some(path) = &self.session.vault_path {
            let _ = self.session.ui_state.save_for_vault(path);
        }
    }

    fn toggle_gen_option(&mut self, option: GenOption) {
        let p = &mut self.generate.policy;
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
            self.entry_list.selected = self
                .entry_list
                .selected
                .saturating_sub(self.entry_list.per_page);
            self.adjust_page();
        } else if self.screen == Screen::Help {
            self.help.scroll = self.help.scroll.saturating_sub(15);
        }
        AppSignal::Continue
    }

    fn handle_page_down(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList {
            self.entry_list.selected = (self.entry_list.selected + self.entry_list.per_page)
                .min(self.entry_list.entries.len().saturating_sub(1));
            self.adjust_page();
        } else if self.screen == Screen::Help {
            self.help.scroll = self.help.scroll.saturating_add(15);
        }
        AppSignal::Continue
    }

    fn handle_home(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList {
            self.entry_list.selected = 0;
            self.entry_list.page = 0;
        } else if self.screen == Screen::Help {
            self.help.scroll = 0;
        }
        AppSignal::Continue
    }

    fn handle_end(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList {
            self.entry_list.selected = self.entry_list.entries.len().saturating_sub(1);
            self.adjust_page();
        } else if self.screen == Screen::Help {
            // Set above the max; `draw_help` clamps it to the bottom edge.
            self.help.scroll = u16::MAX;
        }
        AppSignal::Continue
    }

    fn adjust_page(&mut self) {
        self.entry_list.page = self.entry_list.selected / self.entry_list.per_page.max(1);
    }

    fn handle_enter(&mut self) -> AppSignal {
        match self.screen {
            Screen::VaultList => {
                if let Some(vault) = self
                    .vault_list
                    .vaults
                    .get(self.vault_list.selected)
                    .cloned()
                {
                    self.session.vault_path = Some(vault.path);
                    self.unlock.mode = UnlockMode::Open;
                    self.screen = Screen::Unlock;
                    self.unlock.password.clear();
                    self.unlock.error = None;
                    self.start_quick_unlock_check();
                }
            }
            Screen::Unlock => {
                if self.unlock.mode == UnlockMode::Delete {
                    self.try_delete_vault();
                } else {
                    self.try_unlock();
                }
            }
            Screen::EntryList => {
                if let Some(idx) = self.get_selected_entry_idx() {
                    self.entry_detail.entry_idx = Some(idx);
                    self.entry_detail.show_password = false;
                    self.screen = Screen::EntryDetail;
                }
            }
            Screen::EntryForm => match self.entry_form.field_idx {
                6 => self.cycle_icon(),
                7 => self.cycle_entry_type(),
                _ => {
                    self.handle_tab_next();
                }
            },
            Screen::NewVault => {
                self.handle_tab_next();
            }
            Screen::Locked => {
                self.screen = Screen::Unlock;
                self.unlock.password.clear();
            }
            Screen::Confirm => {
                self.confirm_yes();
            }
            Screen::Generate => {
                self.do_generate();
            }
            _ => {}
        }
        AppSignal::Continue
    }

    fn try_unlock(&mut self) {
        let Some(path) = self.session.vault_path.clone() else {
            self.unlock.error = Some("No vault selected".into());
            return;
        };
        // `mem::take` moves the plaintext into a secret and empties the buffer,
        // so the plain master password never lingers in `App`.
        let password = Secret::new(SecretString::from(std::mem::take(
            &mut self.unlock.password,
        )));
        self.unlock.error = None;
        self.pending = Some(PendingTask {
            kind: TaskKind::Unlock,
            started: Instant::now(),
            rx: tasks::spawn_unlock(path, password),
        });
    }

    /// Kicks off the fingerprint availability probe when entering the unlock
    /// screen. Runs only when quick unlock is enabled; the result decides
    /// whether the lock screen offers `[F] Fingerprint unlock`.
    fn start_quick_unlock_check(&mut self) {
        if !self.quick_unlock.enabled {
            return;
        }
        let Some(path) = self.session.vault_path.clone() else {
            return;
        };
        let Some(account) = crate::quick_unlock::vault_account(&path) else {
            return;
        };
        self.quick_unlock.checked = false;
        self.quick_unlock.available = false;
        self.quick_unlock.has_secret = false;
        self.pending = Some(PendingTask {
            kind: TaskKind::QuickUnlockCheck,
            started: Instant::now(),
            rx: tasks::spawn_quick_unlock_check(self.quick_unlock.detect_cmd.clone(), account),
        });
    }

    /// Attempts a fingerprint unlock: verifies the print, then opens the vault
    /// with the stored master password. Runs entirely in the worker thread so
    /// the argon2 work never freezes the event loop.
    fn quick_unlock_action(&mut self) -> AppSignal {
        if !self.quick_unlock.enabled
            || !self.quick_unlock.checked
            || !self.quick_unlock.available
            || !self.quick_unlock.has_secret
        {
            return AppSignal::Continue;
        }
        let Some(path) = self.session.vault_path.clone() else {
            return AppSignal::Continue;
        };
        let Some(account) = crate::quick_unlock::vault_account(&path) else {
            return AppSignal::Continue;
        };
        self.unlock.error = None;
        self.pending = Some(PendingTask {
            kind: TaskKind::QuickUnlockVerify,
            started: Instant::now(),
            rx: tasks::spawn_quick_unlock_verify(
                path,
                self.quick_unlock.verify_cmd.clone(),
                account,
            ),
        });
        AppSignal::Continue
    }

    /// Verifies the vault's master password in a background task and, only if
    /// correct, deletes the vault file. Mirrors [`Self::try_unlock`] so the
    /// same password screen drives both flows.
    fn try_delete_vault(&mut self) {
        let Some(path) = self.session.vault_path.clone() else {
            self.unlock.error = Some("No vault selected".into());
            return;
        };
        // `mem::take` so the plaintext master password never lingers in `App`.
        let password = Secret::new(SecretString::from(std::mem::take(
            &mut self.unlock.password,
        )));
        self.unlock.error = None;
        self.pending = Some(PendingTask {
            kind: TaskKind::DeleteVault,
            started: Instant::now(),
            rx: tasks::spawn_delete_vault(path, password),
        });
    }

    fn load_entries(&mut self) {
        self.entry_list.all_entries.clear();
        if let Some(vault) = &self.session.vault {
            let now = time::OffsetDateTime::now_utc();
            // First pass: gather (password, age_days) pairs so the health check
            // can count reuse across the whole vault before we build rows.
            let pairs: Vec<(&str, i64)> = vault
                .entries()
                .iter()
                .map(|e| {
                    let age_days = (now - e.updated_at()).whole_days().max(0);
                    (e.password().expose_secret().as_str(), age_days)
                })
                .collect();
            let healths = cista_core::health::assess_all(pairs);

            for (i, entry) in vault.entries().iter().enumerate() {
                self.entry_list.all_entries.push(EntryRow {
                    id: entry.id(),
                    name: entry.name().to_string(),
                    username: entry.username().map(|s| s.to_string()),
                    url: entry.url().map(|s| s.to_string()),
                    notes: entry
                        .notes()
                        .map(|n| n.expose_secret().as_str().to_string()),
                    favorite: self.session.ui_state.is_favourite(entry.id()),
                    icon: self.session.ui_state.display_icon(entry.id()),
                    entry_type: self.session.ui_state.entry_type(entry.id()),
                    created_at: entry.created_at(),
                    updated_at: entry.updated_at(),
                    health: healths[i].clone(),
                });
            }
        }
        self.recompute_filtered_entries();
    }

    /// Rebuilds `entries` from the pristine `all_entries` list, applying the
    /// current search filter and sort order. Never mutates `all_entries`, so
    /// clearing or shortening the query restores every entry.
    fn recompute_filtered_entries(&mut self) {
        let mut entries: Vec<EntryRow> = if self.entry_list.search_query.is_empty() {
            self.entry_list.all_entries.clone()
        } else {
            let q = self.entry_list.search_query.to_lowercase();
            self.entry_list
                .all_entries
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
                        || e.notes
                            .as_deref()
                            .map(|n| n.to_lowercase().contains(&q))
                            .unwrap_or(false)
                })
                .cloned()
                .collect()
        };

        // Favorites always come first; within each group the requested sort
        // order applies. `sort_by` is stable, so equal-category rows keep
        // their relative order from the search filter.
        entries.sort_by(|a, b| {
            b.favorite
                .cmp(&a.favorite)
                .then_with(|| match self.entry_list.sort_mode {
                    SortMode::NameAsc => name_cmp(a, b),
                    SortMode::NameDesc => name_cmp(b, a),
                    SortMode::UpdatedDesc => b.updated_at.cmp(&a.updated_at),
                    SortMode::UpdatedAsc => a.updated_at.cmp(&b.updated_at),
                    SortMode::CreatedDesc => b.created_at.cmp(&a.created_at),
                    SortMode::CreatedAsc => a.created_at.cmp(&b.created_at),
                })
        });

        self.entry_list.entries = entries;
        self.entry_list.selected = 0;
        self.entry_list.page = 0;
    }

    fn get_selected_entry_idx(&self) -> Option<usize> {
        if self.entry_list.entries.is_empty() {
            None
        } else {
            Some(
                self.entry_list
                    .selected
                    .min(self.entry_list.entries.len() - 1),
            )
        }
    }

    /// Health summary across all entries, e.g. `2 weak · 1 reused`. Empty
    /// beyond a healthy vault, so the header stays tidy.
    pub fn health_summary(&self) -> Option<String> {
        let mut weak = 0usize;
        let mut reused = 0usize;
        for e in &self.entry_list.all_entries {
            if e.health.score < 60 {
                weak += 1;
            }
            if e.health.used_in > 1 {
                reused += 1;
            }
        }
        if weak == 0 && reused == 0 {
            return None;
        }
        let mut parts = Vec::new();
        if weak > 0 {
            parts.push(format!("{weak} weak"));
        }
        if reused > 0 {
            parts.push(format!("{reused} reused"));
        }
        Some(parts.join(" · "))
    }

    /// Color for a health score: red for weak, yellow for middling, green for
    /// strong.
    pub fn health_color(score: u8) -> Color {
        if score < 60 {
            Color::Red
        } else if score < 85 {
            Color::Yellow
        } else {
            Color::Green
        }
    }

    fn handle_dashboard(&mut self) -> AppSignal {
        self.previous_screen = Some(self.screen);
        let mut rows: Vec<_> = self.entry_list.all_entries.iter().collect();
        rows.sort_by_key(|e| e.health.score);
        self.dashboard.rows = rows.iter().map(|e| e.id).collect();
        self.dashboard.selected = 0;
        self.dashboard.scroll = 0;
        self.screen = Screen::Dashboard;
        AppSignal::Continue
    }

    fn handle_back(&mut self) -> AppSignal {
        match self.screen {
            Screen::Unlock => {
                self.unlock.mode = UnlockMode::Open;
                self.screen = Screen::VaultList;
                self.unlock.password.zeroize();
            }
            Screen::EntryList if self.entry_list.in_search => {
                self.entry_list.search_query.clear();
                self.entry_list.in_search = false;
                self.recompute_filtered_entries();
            }
            Screen::EntryList => {
                self.screen = Screen::VaultList;
                self.session.vault = None;
                self.session.master_password = None;
                self.session.locked = false;
                self.entry_list.in_search = false;
                self.entry_list.search_query.clear();
            }
            Screen::EntryDetail => {
                self.screen = Screen::EntryList;
                self.entry_detail.entry_idx = None;
                self.entry_detail.show_password = false;
            }
            Screen::EntryForm => {
                self.back_from_form();
            }
            Screen::NewVault => {
                self.reset_new_vault_fields();
                self.screen = Screen::VaultList;
            }
            Screen::Generate => {
                self.screen = self.generate.prev_screen.unwrap_or(Screen::VaultList);
                self.generate.result = None;
                self.generate.from_form = false;
            }
            Screen::Locked => {
                self.screen = Screen::Unlock;
                self.unlock.password.clear();
            }
            Screen::Confirm => {
                self.screen = self.previous_screen.unwrap_or(Screen::EntryList);
                self.confirm.on_yes = None;
            }
            Screen::Help => {
                self.screen = self.previous_screen.unwrap_or(Screen::VaultList);
            }
            Screen::Dashboard => {
                self.screen = Screen::EntryList;
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
        self.entry_form.fields.password.zeroize();
        self.entry_form.fields.password_confirm.zeroize();
        self.entry_form.fields = FormFields::default();
        self.entry_form.field_idx = 0;
    }

    fn reset_new_vault_fields(&mut self) {
        self.new_vault.fields.name.zeroize();
        self.new_vault.fields.password.zeroize();
        self.new_vault.fields.confirm.zeroize();
        self.new_vault.fields = NewVaultFields::default();
        self.new_vault.field_idx = 0;
    }

    fn handle_search(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList {
            self.entry_list.in_search = true;
        }
        AppSignal::Continue
    }

    fn handle_add(&mut self) -> AppSignal {
        if self.screen == Screen::EntryList {
            self.screen = Screen::EntryForm;
            self.previous_screen = Some(Screen::EntryList);
            self.entry_form.mode = FormMode::Add;
            self.entry_form.fields = FormFields::default();
            self.entry_form.field_idx = 0;
        }
        AppSignal::Continue
    }

    fn handle_generate(&mut self) -> AppSignal {
        if self.screen == Screen::Generate {
            // Second Ctrl+g while the generator is open: if it was opened from
            // the entry form, drop the generated password into the form and go
            // back. Otherwise (standalone generator) there is nothing else to
            // do on a second activate.
            if self.generate.from_form {
                if let Some(pwd) = self.generate.result.clone() {
                    self.entry_form.fields.password = pwd;
                    self.entry_form.fields.password_confirm =
                        self.entry_form.fields.password.clone();
                    self.generate.from_form = false;
                    self.generate.result = None;
                    self.screen = self.generate.prev_screen.unwrap_or(Screen::EntryForm);
                    self.set_status("Generated password applied to form");
                } else {
                    self.set_error("Generate a password first");
                }
            }
            return AppSignal::Continue;
        }
        self.generate.from_form = self.screen == Screen::EntryForm;
        self.generate.prev_screen = Some(self.screen);
        self.screen = Screen::Generate;
        self.generate.policy = GenPolicy::default();
        self.generate.result = None;
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
                if let Some(vault) = self.vault_list.vaults.get(self.vault_list.selected) {
                    // Deleting a vault is destructive, so the master-password
                    // screen is reused (same as before unlocking): ask for the
                    // vault's password and only delete when it verifies.
                    self.session.vault_path = Some(vault.path.clone());
                    self.unlock.mode = UnlockMode::Delete;
                    self.unlock.password.clear();
                    self.unlock.error = None;
                    self.screen = Screen::Unlock;
                }
            }
            Screen::EntryList | Screen::EntryDetail => {
                let idx = self
                    .entry_detail
                    .entry_idx
                    .or_else(|| self.get_selected_entry_idx());
                if let Some(idx) = idx {
                    if let Some(entry) = self.entry_list.entries.get(idx) {
                        self.confirm.message = format!("Delete entry '{}'?", entry.name);
                        self.confirm.on_yes = Some(ConfirmAction::DeleteEntry);
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
            match &self.generate.result {
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

    /// Applies `f` to the currently selected entry. Works from both the entry
    /// list (selected/hovered row) and the entry detail screen, so a plain key
    /// can copy the password (or username/URL) of the row under the cursor
    /// without opening it first.
    fn copy_from_entry(
        &mut self,
        f: impl FnOnce(&cista_core::Entry) -> anyhow::Result<()>,
        ok_msg: &str,
    ) {
        if self.screen != Screen::EntryList && self.screen != Screen::EntryDetail {
            return;
        }
        let idx = match self.screen {
            Screen::EntryDetail => self.entry_detail.entry_idx,
            _ => self.get_selected_entry_idx(),
        };
        let Some(idx) = idx else { return };
        if self.entry_list.entries.get(idx).is_none() {
            return;
        }
        if let Some(vault) = &self.session.vault {
            if let Some(entry) = vault.find_by_id(self.entry_list.entries[idx].id) {
                match f(entry) {
                    Ok(()) => self.set_status(ok_msg),
                    Err(_) => self.set_error("Clipboard unavailable"),
                }
            }
        }
    }

    fn handle_edit(&mut self) -> AppSignal {
        if let Screen::EntryDetail = self.screen {
            if let Some(idx) = self.entry_detail.entry_idx {
                if self.entry_list.entries.get(idx).is_some() {
                    if let Some(vault) = &self.session.vault {
                        if let Some(entry) = vault.find_by_id(self.entry_list.entries[idx].id) {
                            self.entry_form.mode = FormMode::Edit;
                            self.entry_form.fields = FormFields {
                                name: entry.name().to_string(),
                                username: entry.username().unwrap_or("").to_string(),
                                password: String::new(),
                                password_confirm: String::new(),
                                url: entry.url().unwrap_or("").to_string(),
                                notes: entry
                                    .notes()
                                    .map(|n| n.expose_secret().as_str().to_string())
                                    .unwrap_or_default(),
                                icon: self
                                    .session
                                    .ui_state
                                    .icon(entry.id())
                                    .unwrap_or("")
                                    .to_string(),
                                entry_type: self.session.ui_state.entry_type(entry.id()),
                            };
                            self.entry_form.field_idx = 0;
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
            Screen::EntryDetail => {
                self.entry_detail.show_password = !self.entry_detail.show_password
            }
            // Space toggles the focused option on the generate screen.
            Screen::Generate => {
                let option = GenOption::ALL[self.generate.selected];
                self.toggle_gen_option(option);
            }
            _ => {}
        }
        AppSignal::Continue
    }

    fn handle_tab_next(&mut self) -> AppSignal {
        match self.screen {
            Screen::EntryForm => {
                self.entry_form.field_idx = (self.entry_form.field_idx + 1) % 8;
            }
            Screen::NewVault => {
                self.new_vault.field_idx = (self.new_vault.field_idx + 1) % 3;
            }
            _ => {}
        }
        AppSignal::Continue
    }

    fn handle_tab_prev(&mut self) -> AppSignal {
        match self.screen {
            Screen::EntryForm => {
                self.entry_form.field_idx = (self.entry_form.field_idx + 7) % 8;
            }
            Screen::NewVault => {
                self.new_vault.field_idx = (self.new_vault.field_idx + 2) % 3;
            }
            _ => {}
        }
        AppSignal::Continue
    }

    /// Cycle the entry type selector when pressed while the type field is
    /// focused. Walks through every variant in [`EntryType::ALL`], wrapping
    /// around at the end.
    fn cycle_entry_type(&mut self) {
        self.entry_form.fields.entry_type = self.entry_form.fields.entry_type.next();
    }

    /// Palette of preset icons offered in the entry form. The empty string is
    /// "no custom icon", which falls back to the entry type's default.
    fn icon_palette() -> &'static [&'static str] {
        &[
            "", "🔑", "💳", "📝", "🪪", "🏦", "✉️", "📶", "🖥️", "🪙", "🛍️", "🎮", "📞", "☁️", "🛡️",
            "🔒",
        ]
    }

    /// Cycle the entry form's icon field through the preset palette when
    /// pressed while the icon field is focused.
    fn cycle_icon(&mut self) {
        let palette = Self::icon_palette();
        let current = self.entry_form.fields.icon.as_str();
        let next = match palette.iter().position(|i| *i == current) {
            Some(idx) => palette[(idx + 1) % palette.len()],
            None => palette[0],
        };
        self.entry_form.fields.icon = next.to_string();
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
        if let Some(action) = self.confirm.on_yes {
            match action {
                ConfirmAction::DeleteEntry => {
                    let idx = self
                        .entry_detail
                        .entry_idx
                        .or_else(|| self.get_selected_entry_idx());
                    if let Some(idx) = idx {
                        if let Some(entry) = self.entry_list.entries.get(idx).cloned() {
                            if let Some(vault) = &mut self.session.vault {
                                if vault.remove_by_id(entry.id).is_ok() {
                                    self.session.ui_state.remove_entry(entry.id);
                                    self.save_ui_state();
                                    self.load_entries();
                                    self.start_save_task(TaskKind::SaveEntryDelete);
                                }
                            }
                        }
                    }
                }
            }
        }
        self.confirm.on_yes = None;
        self.screen = self.previous_screen.unwrap_or(Screen::EntryList);
        self.entry_detail.entry_idx = None;
        self.entry_detail.show_password = false;
    }

    fn create_vault(&mut self) {
        let name = self.new_vault.fields.name.trim().to_string();
        if name.is_empty() {
            self.set_error("Vault name is required");
            self.reset_new_vault_fields();
            return;
        }
        if self.new_vault.fields.password.is_empty() {
            self.set_error("Password is required");
            self.reset_new_vault_fields();
            return;
        }
        if self.new_vault.fields.password != self.new_vault.fields.confirm {
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

        let key = std::mem::take(&mut self.new_vault.fields.password);
        self.pending = Some(PendingTask {
            kind: TaskKind::CreateVault,
            started: Instant::now(),
            rx: tasks::spawn_create_vault(name, path, Secret::new(SecretString::from(key))),
        });
    }

    fn save_entry(&mut self) {
        use cista_core::Entry;

        let name = self.entry_form.fields.name.clone();
        let username = self.entry_form.fields.username.clone();
        let notes = self.entry_form.fields.notes.clone();
        let url = self.entry_form.fields.url.clone();
        let icon = self.entry_form.fields.icon.clone();
        let entry_type = self.entry_form.fields.entry_type;
        let password_raw = std::mem::take(&mut self.entry_form.fields.password);
        let confirm_raw = std::mem::take(&mut self.entry_form.fields.password_confirm);

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
            if self.entry_form.mode == FormMode::Add {
                self.set_error("Password is required for new entries");
                return;
            }
            None
        } else {
            // Move the plaintext into a secret so no plain copy remains behind.
            Some(Secret::new(SecretString::from(password_raw)))
        };

        if let Some(vault) = &mut self.session.vault {
            let result: anyhow::Result<&mut Vault> = match self.entry_form.mode {
                FormMode::Add => {
                    let created = Entry::new(
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
                    )
                    .map_err(anyhow::Error::from);
                    match created {
                        Ok(e) => {
                            let id = e.id();
                            vault.add_entry(e);
                            self.session.ui_state.set_entry_type(id, entry_type);
                            self.session.ui_state.set_icon(id, Some(icon.clone()));
                            Ok(vault)
                        }
                        Err(err) => Err(err),
                    }
                }
                FormMode::Edit => {
                    if let Some(idx) = self.entry_detail.entry_idx {
                        let entry_id = self.entry_list.entries[idx].id;
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
                            self.session.ui_state.set_entry_type(entry_id, entry_type);
                            self.session.ui_state.set_icon(entry_id, Some(icon.clone()));
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
                    self.save_ui_state();
                    let kind = match self.entry_form.mode {
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
            self.session.vault.clone(),
            self.session.vault_path.clone(),
            self.session.master_password.clone(),
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
            length: self.generate.policy.length,
            include_lowercase: self.generate.policy.include_lowercase,
            include_uppercase: self.generate.policy.include_uppercase,
            include_digits: self.generate.policy.include_digits,
            include_symbols: self.generate.policy.include_symbols,
            exclude_ambiguous: self.generate.policy.exclude_ambiguous,
        };

        match generate_password(&policy) {
            Ok(pwd) => {
                self.generate.result = Some(pwd);
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
        if !self.session.locked
            && self.session.auto_lock_seconds > 0
            && self.session.last_activity.elapsed()
                >= Duration::from_secs(self.session.auto_lock_seconds)
        {
            self.lock_vault();
        }
    }
}
