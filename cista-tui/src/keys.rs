use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub quit: KeyBinding,
    pub help: KeyBinding,
    pub lock: KeyBinding,
    pub up: KeyBinding,
    pub down: KeyBinding,
    pub up_alt: KeyBinding,
    pub down_alt: KeyBinding,
    pub page_up: KeyBinding,
    pub page_down: KeyBinding,
    pub home: KeyBinding,
    pub end: KeyBinding,
    pub enter: KeyBinding,
    pub back: KeyBinding,
    pub search: KeyBinding,
    pub add: KeyBinding,
    pub generate: KeyBinding,
    pub delete: KeyBinding,
    pub copy_password: KeyBinding,
    pub copy_username: KeyBinding,
    pub copy_url: KeyBinding,
    pub edit: KeyBinding,
    pub reveal: KeyBinding,
    pub new_vault: KeyBinding,
    pub left: KeyBinding,
    pub right: KeyBinding,
    pub reroll: KeyBinding,
    pub tab_next: KeyBinding,
    pub tab_prev: KeyBinding,
    pub save: KeyBinding,
}

#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn matches(&self, key: KeyEvent) -> bool {
        key.code == self.code && key.modifiers == self.modifiers
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        use KeyCode::*;

        Self {
            quit: kb(Char('q'), KeyModifiers::NONE),
            help: kb(Char('?'), KeyModifiers::NONE),
            lock: kb(Char('L'), KeyModifiers::SHIFT),
            up: kb(Up, KeyModifiers::NONE),
            down: kb(Down, KeyModifiers::NONE),
            up_alt: kb(Char('k'), KeyModifiers::NONE),
            down_alt: kb(Char('j'), KeyModifiers::NONE),
            page_up: kb(PageUp, KeyModifiers::NONE),
            page_down: kb(PageDown, KeyModifiers::NONE),
            home: kb(Home, KeyModifiers::NONE),
            end: kb(End, KeyModifiers::NONE),
            enter: kb(Enter, KeyModifiers::NONE),
            back: kb(Esc, KeyModifiers::NONE),
            search: kb(Char('/'), KeyModifiers::NONE),
            add: kb(Char('a'), KeyModifiers::NONE),
            generate: kb(Char('g'), KeyModifiers::NONE),
            delete: kb(Char('d'), KeyModifiers::NONE),
            copy_password: kb(Char('c'), KeyModifiers::NONE),
            copy_username: kb(Char('u'), KeyModifiers::NONE),
            copy_url: kb(Char('l'), KeyModifiers::NONE),
            edit: kb(Char('e'), KeyModifiers::NONE),
            reveal: kb(Char(' '), KeyModifiers::NONE),
            new_vault: kb(Char('n'), KeyModifiers::NONE),
            left: kb(Left, KeyModifiers::NONE),
            right: kb(Right, KeyModifiers::NONE),
            reroll: kb(Char('r'), KeyModifiers::NONE),
            tab_next: kb(Tab, KeyModifiers::NONE),
            tab_prev: kb(BackTab, KeyModifiers::NONE),
            save: kb(Char('s'), KeyModifiers::CONTROL),
        }
    }
}

fn kb(code: KeyCode, modifiers: KeyModifiers) -> KeyBinding {
    KeyBinding { code, modifiers }
}

pub struct ActionMapper {
    bindings: KeyBindings,
}

/// A key->action entry in the binding tables.
type Binding = (Action, fn(&KeyBindings) -> &KeyBinding);

/// Actions that also fire while a text field is focused (see `map_global`).
const GLOBAL: &[Binding] = &[
    (Action::Back, |b| &b.back),
    (Action::TabNext, |b| &b.tab_next),
    (Action::TabPrev, |b| &b.tab_prev),
    (Action::Save, |b| &b.save),
];

/// All remaining actions, resolved only when no text field consumed the key.
/// Some actions have a secondary key (e.g. `j`/`k` for up/down); those appear
/// as separate entries reusing the same action.
const BOUND: &[Binding] = &[
    (Action::Quit, |b| &b.quit),
    (Action::Help, |b| &b.help),
    (Action::Lock, |b| &b.lock),
    (Action::Up, |b| &b.up),
    (Action::Up, |b| &b.up_alt),
    (Action::Down, |b| &b.down),
    (Action::Down, |b| &b.down_alt),
    (Action::Left, |b| &b.left),
    (Action::Right, |b| &b.right),
    (Action::PageUp, |b| &b.page_up),
    (Action::PageDown, |b| &b.page_down),
    (Action::Home, |b| &b.home),
    (Action::End, |b| &b.end),
    (Action::Enter, |b| &b.enter),
    (Action::Search, |b| &b.search),
    (Action::Add, |b| &b.add),
    (Action::Generate, |b| &b.generate),
    (Action::Delete, |b| &b.delete),
    (Action::CopyPassword, |b| &b.copy_password),
    (Action::CopyUsername, |b| &b.copy_username),
    (Action::CopyUrl, |b| &b.copy_url),
    (Action::Edit, |b| &b.edit),
    (Action::Reveal, |b| &b.reveal),
    (Action::NewVault, |b| &b.new_vault),
    (Action::Reroll, |b| &b.reroll),
];

impl ActionMapper {
    pub fn new(bindings: KeyBindings) -> Self {
        Self { bindings }
    }

    /// Keys that work on every screen, including while a text field is
    /// focused: cancel/back (Esc), field navigation (Tab / Shift+Tab) and
    /// save (Ctrl+s). These never collide with plain characters, so they are
    /// resolved *before* text capture.
    pub fn map_global(&self, key: KeyEvent) -> Option<Action> {
        resolve(key, GLOBAL, &self.bindings)
    }

    /// All remaining actions, applied only when no text field consumed the
    /// key as input.
    pub fn map(&self, key: KeyEvent) -> Option<Action> {
        resolve(key, BOUND, &self.bindings)
    }
}

/// Finds the first `(action, key)` pair whose key matches `key`. Walking a
/// table (rather than an if/else chain) keeps every binding a single
/// declarative row.
fn resolve(key: KeyEvent, table: &[Binding], bindings: &KeyBindings) -> Option<Action> {
    table
        .iter()
        .find(|(_, keyfn)| keyfn(bindings).matches(key))
        .map(|(action, _)| *action)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Help,
    Lock,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Enter,
    Back,
    Search,
    Add,
    Generate,
    Delete,
    CopyPassword,
    CopyUsername,
    CopyUrl,
    Edit,
    Reveal,
    NewVault,
    Reroll,
    TabNext,
    TabPrev,
    Save,
}
