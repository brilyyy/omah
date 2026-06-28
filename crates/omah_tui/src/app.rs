use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use omah_lib::{
    config::load_toml_config,
    deps,
    ops::{diff, status, DotStatus, FileChange},
    DotfileConfig, OmahConfig,
};

use crate::{
    dep_flow::DepWorkspace,
    ops::{OpsHandle, OpsMessage},
};

// ── Check types ──────────────────────────────────────────────────────────

pub const CHECK_TYPES: &[(&str, &str, &str)] = &[
    ("none", "No check", "Step always shows as pending"),
    ("bin", "Binary/function", "Skip when binary found in PATH"),
    ("file", "File exists", "Skip when file exists"),
    ("dir", "Dir exists", "Skip when directory exists"),
    ("app", "macOS App", "Skip when app bundle found"),
    ("cmd", "Command exits 0", "Skip when shell command succeeds"),
    ("out", "Output matches", "Skip when stdout matches value"),
    ("skip", "Always skip", "Permanently mark as done"),
];

/// Parse a stored check string into (type, value).
pub fn parse_check(raw: &str) -> (String, String) {
    if raw.is_empty() || raw == "none" {
        return ("none".into(), String::new());
    }
    if raw == "skip" || raw.starts_with("skip:") {
        return ("skip".into(), String::new());
    }
    if let Some(v) = raw.strip_prefix("out:") {
        return ("out".into(), v.to_string());
    }
    for &(prefix, _, _) in CHECK_TYPES {
        if prefix != "none" && prefix != "skip" && prefix != "out" {
            if let Some(v) = raw.strip_prefix(&format!("{prefix}:")) {
                return (prefix.into(), v.to_string());
            }
        }
    }
    // Backward-compat: bare path → file, bare name → bin
    if raw.starts_with('/') || raw.starts_with('~') {
        ("file".into(), raw.to_string())
    } else {
        ("bin".into(), raw.to_string())
    }
}

/// Serialize (type, value) back to stored format.
pub fn serialize_check(check_type: &str, value: &str) -> Option<String> {
    match check_type {
        "none" => None,
        "skip" => Some("skip".into()),
        _ if value.trim().is_empty() => None,
        "out" => Some(format!("out:{}", value.trim())),
        _ => Some(format!("{}:{}", check_type, value.trim())),
    }
}

// ── Tab ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tab {
    Dots = 0,
    Log = 1,
}

impl Tab {
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn label(self) -> &'static str {
        match self {
            Tab::Dots => "Dots",
            Tab::Log => "Log",
        }
    }
    pub fn all() -> &'static [Tab] {
        &[Tab::Dots, Tab::Log]
    }
}

// ── Log entry types ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct LogEntry {
    pub text: String,
    pub kind: LogKind,
}

#[derive(Clone, PartialEq)]
pub enum LogKind {
    Info,
    Success,
    Error,
    Warning,
}

// ── Form types ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct FormState {
    pub title: String,
    pub fields: Vec<FormField>,
    pub focused: usize,
    pub error: Option<String>,
}

#[derive(Clone)]
pub enum FormField {
    Text {
        label: &'static str,
        value: String,
        cursor: usize,
    },
    Toggle {
        label: &'static str,
        value: bool,
    },
    SetupSteps {
        items: Vec<SetupFieldRow>,
        insert_index: usize,
    },
}

#[derive(Clone)]
pub struct SetupFieldRow {
    pub install: String,
    pub check_type: String,
    pub check_value: String,
    pub install_cursor: usize,
    pub check_cursor: usize,
    pub focused_install: bool,
    pub show_check_menu: bool,
    pub check_menu_index: usize,
}

impl SetupFieldRow {
    pub fn check_display(&self) -> String {
        match self.check_type.as_str() {
            "none" => String::new(),
            "skip" => "skip".into(),
            _ => {
                if self.check_value.is_empty() {
                    self.check_type.clone()
                } else {
                    format!("{}:{}", self.check_type, self.check_value)
                }
            }
        }
    }
}

// ── Modal state ──────────────────────────────────────────────────────────

pub enum ModalState {
    AddForm(FormState),
    EditForm(FormState, String), // original dot name
    DepFlow(DepWorkspace),
    Error(String),
    RemoveConfirm(String), // dot name
    Confirm {
        message: String,
        action: ConfirmAction,
    },
    HelpOverlay(HelpContext),
    Settings,
}

#[derive(Clone, Copy)]
pub enum HelpContext {
    Dots,
    Log,
    Form,
    Detail,
    Settings,
    CheckSelector,
}

pub enum ConfirmAction {
    Backup(usize),
    Restore(usize),
    RunBackupAll,
    RunRestoreAll,
}

/// Modal-specific action result.
enum ModalAction {
    /// Modal remains open
    Stay,
    /// Close without saving
    Close,
    /// Save form data then close
    Save,
}

// ── Settings form ────────────────────────────────────────────────────────

pub struct SettingsForm {
    pub vault_path: String,
    pub vault_path_cursor: usize,
    pub os_index: usize,
    pub pkg_manager_index: usize,
    pub focused: usize, // 0=vault_path, 1=os, 2=pkg_manager
    pub dirty: bool,
}

impl SettingsForm {
    pub const OS_OPTIONS: &'static [&'static str] = &["auto", "macos", "linux"];
    pub const PKG_OPTIONS: &'static [&'static str] = &["auto", "brew", "apt-get", "pacman", "dnf", "zypper"];

    pub fn new(config: &OmahConfig) -> Self {
        let os_index = Self::OS_OPTIONS
            .iter()
            .position(|o| Some(*o) == config.os.as_deref())
            .unwrap_or(0);
        let pkg_index = Self::PKG_OPTIONS
            .iter()
            .position(|p| Some(*p) == config.pkg_manager.as_deref())
            .unwrap_or(0);
        Self {
            vault_path: config.vault_path.clone(),
            vault_path_cursor: config.vault_path.len(),
            os_index,
            pkg_manager_index: pkg_index,
            focused: 0,
            dirty: false,
        }
    }
}

// ── Step execution state ─────────────────────────────────────────────────

pub struct StepExecState {
    pub running: bool,
    pub done: bool,
    pub success: bool,
    pub output: Vec<String>,
}

impl Default for StepExecState {
    fn default() -> Self {
        Self { running: false, done: false, success: false, output: vec![] }
    }
}

// ── App state ────────────────────────────────────────────────────────────

pub struct App {
    // Config
    pub config_path: PathBuf,
    pub config: Option<OmahConfig>,

    // Navigation
    pub active_tab: Tab,
    pub should_quit: bool,
    pub tick_counter: u64,
    pub selected_index: usize,

    // Data
    pub statuses: Vec<DotStatus>,
    pub changes: Vec<FileChange>,

    // Search
    pub search: String,
    pub search_cursor: usize,
    pub search_focused: bool,

    // Detail expand
    pub detail_expanded: Option<usize>, // which dot index is expanded

    // Log
    pub log_entries: Vec<LogEntry>,

    // Modal
    pub modal: Option<ModalState>,

    // Background operations
    pub ops_handle: Option<OpsHandle>,

    // Settings
    pub settings_form: Option<SettingsForm>,

    // Step execution (inline in detail panel)
    pub step_exec: Option<(String, StepExecState)>, // (dot_name, state)

    // Receiver for setup step output
    pub setup_rx: Option<std::sync::mpsc::Receiver<String>>,
}

impl App {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            config: None,
            active_tab: Tab::Dots,
            should_quit: false,
            tick_counter: 0,
            selected_index: 0,
            statuses: vec![],
            changes: vec![],
            search: String::new(),
            search_cursor: 0,
            search_focused: false,
            detail_expanded: None,
            log_entries: vec![],
            modal: None,
            ops_handle: None,
            settings_form: None,
            step_exec: None,
            setup_rx: None,
        }
    }

    // ── Data loading ────────────────────────────────────────────────────

    pub fn load_config(&mut self) {
        match load_toml_config(&self.config_path) {
            Ok(cfg) => {
                self.config = Some(cfg);
                self.load_status();
                self.load_diff();
            }
            Err(e) => {
                self.log_entries.push(LogEntry {
                    text: format!("✗ Failed to load config: {e}"),
                    kind: LogKind::Error,
                });
            }
        }
    }

    pub fn load_status(&mut self) {
        if let Some(ref config) = self.config.clone() {
            match status(config) {
                Ok(s) => {
                    self.statuses = s;
                    if self.selected_index >= self.statuses.len() {
                        self.selected_index = self.statuses.len().saturating_sub(1);
                    }
                }
                Err(e) => {
                    self.log_entries.push(LogEntry {
                        text: format!("✗ Failed to load status: {e}"),
                        kind: LogKind::Error,
                    });
                }
            }
        }
    }

    pub fn load_diff(&mut self) {
        if let Some(ref config) = self.config.clone() {
            match diff(config) {
                Ok(c) => self.changes = c,
                Err(e) => {
                    self.log_entries.push(LogEntry {
                        text: format!("✗ Failed to diff: {e}"),
                        kind: LogKind::Error,
                    });
                }
            }
        }
    }

    pub fn filtered_statuses(&self) -> Vec<(usize, &DotStatus)> {
        if self.search.is_empty() {
            self.statuses.iter().enumerate().collect()
        } else {
            let q = self.search.to_lowercase();
            self.statuses
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.name.to_lowercase().contains(&q)
                        || s.source.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub fn diff_map(&self) -> std::collections::HashMap<&str, Vec<&FileChange>> {
        let mut map = std::collections::HashMap::new();
        for c in &self.changes {
            map.entry(c.dot_name.as_str()).or_insert_with(Vec::new).push(c);
        }
        map
    }

    // ── Logging ────────────────────────────────────────────────────────

    pub fn add_log(&mut self, text: impl Into<String>, kind: LogKind) {
        self.log_entries.push(LogEntry {
            text: text.into(),
            kind,
        });
    }

    // ── Operation polling ──────────────────────────────────────────────

    pub fn poll_ops(&mut self) {
        let Some(handle) = self.ops_handle.take() else { return };
        loop {
            match handle.receiver.try_recv() {
                Ok(OpsMessage::Log(text)) => {
                    self.add_log(text, LogKind::Info);
                }
                Ok(OpsMessage::Progress(_, _)) => {}
                Ok(OpsMessage::Done(result)) => {
                    self.modal = None;
                    match result {
                        Ok(()) => {
                            self.add_log("✓ Operation completed", LogKind::Success);
                            self.load_status();
                            self.load_diff();
                        }
                        Err(e) => {
                            self.add_log(format!("✗ Operation failed: {e}"), LogKind::Error);
                        }
                    }
                    break;
                }
                Err(TryRecvError::Empty) => {
                    self.ops_handle = Some(handle);
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    self.add_log("⚠ Operation thread disconnected", LogKind::Warning);
                    break;
                }
            }
        }
    }

    // ── Tick ────────────────────────────────────────────────────────────

    pub fn tick(&mut self) {
        self.tick_counter = self.tick_counter.wrapping_add(1);
        self.poll_ops();
        self.poll_setup();
    }

    fn poll_setup(&mut self) {
        let Some(rx) = self.setup_rx.as_ref() else { return };
        let mut done = false;
        let mut output_lines: Vec<String> = Vec::new();

        loop {
            match rx.try_recv() {
                Ok(line) => {
                    if line.starts_with('✓') || line.starts_with('✗') {
                        done = true;
                    }
                    output_lines.push(line);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    done = true;
                    break;
                }
            }
        }

        if !output_lines.is_empty() {
            if let Some((_, ref mut state)) = self.step_exec {
                state.output.extend(output_lines);
                if done {
                    state.running = false;
                    state.done = true;
                    state.success = state.output.last().map(|l| l.starts_with('✓')).unwrap_or(false);
                    self.setup_rx = None;
                    self.load_status();
                }
            }
        }
    }

    // ── Modal open helpers ─────────────────────────────────────────────

    pub fn open_error(&mut self, msg: String) {
        self.modal = Some(ModalState::Error(msg));
    }

    pub fn open_confirm_remove(&mut self, name: String) {
        self.modal = Some(ModalState::RemoveConfirm(name));
    }

    pub fn open_help(&mut self, ctx: HelpContext) {
        self.modal = Some(ModalState::HelpOverlay(ctx));
    }

    pub fn open_settings(&mut self) {
        if let Some(ref config) = self.config {
            self.settings_form = Some(SettingsForm::new(config));
            self.modal = Some(ModalState::Settings);
        }
    }

    pub fn open_add_form(&mut self) {
        let fields = vec![
            FormField::Text {
                label: "Name",
                value: String::new(),
                cursor: 0,
            },
            FormField::Text {
                label: "Source",
                value: String::new(),
                cursor: 0,
            },
            FormField::Toggle {
                label: "Symlink",
                value: false,
            },
            FormField::Text {
                label: "Deps",
                value: String::new(),
                cursor: 0,
            },
            FormField::Text {
                label: "Exclude",
                value: String::new(),
                cursor: 0,
            },
            FormField::SetupSteps {
                items: vec![],
                insert_index: 0,
            },
        ];
        self.modal = Some(ModalState::AddForm(FormState {
            title: " Add Dotfile ".into(),
            fields,
            focused: 0,
            error: None,
        }));
    }

    pub fn open_edit_form(&mut self, idx: usize) {
        let Some(ref dot) = self.config.as_ref().and_then(|c| c.dots.get(idx)) else {
            return;
        };
        let name = dot.name.clone();
        let fields = vec![
            FormField::Text {
                label: "Name",
                value: dot.name.clone(),
                cursor: dot.name.len(),
            },
            FormField::Text {
                label: "Source",
                value: dot.source.clone(),
                cursor: dot.source.len(),
            },
            FormField::Toggle {
                label: "Symlink",
                value: dot.symlink.unwrap_or(false),
            },
            FormField::Text {
                label: "Deps",
                value: dot.deps.as_ref().map(|d| d.join(", ")).unwrap_or_default(),
                cursor: 0,
            },
            FormField::Text {
                label: "Exclude",
                value: dot.exclude.as_ref().map(|e| e.join(", ")).unwrap_or_default(),
                cursor: 0,
            },
            FormField::SetupSteps {
                items: dot
                    .setup
                    .as_ref()
                    .map(|steps| {
                        steps
                            .iter()
                            .map(|s| {
                                let (check_type, check_value) = if s.check.is_none() {
                                    ("none".into(), String::new())
                                } else {
                                    let raw = s.check.as_deref().unwrap_or("");
                                    parse_check(raw)
                                };
                                SetupFieldRow {
                                    install: s.install.clone(),
                                    check_type,
                                    check_value,
                                    install_cursor: s.install.len(),
                                    check_cursor: 0,
                                    focused_install: true,
                                    show_check_menu: false,
                                    check_menu_index: 0,
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                insert_index: 0,
            },
        ];
        self.modal = Some(ModalState::EditForm(
            FormState {
                title: format!(" Edit {name} "),
                fields,
                focused: 0,
                error: None,
            },
            name,
        ));
    }

    pub fn open_dep_flow(&mut self, idx: usize) {
        let Some(ref dot) = self.config.as_ref().and_then(|c| c.dots.get(idx)) else {
            return;
        };
        let ws = DepWorkspace::new(dot);
        if ws.total_count == 0 {
            self.proceed_restore_dot(idx);
            return;
        }
        self.modal = Some(ModalState::DepFlow(ws));
    }

    // ── Key handling ───────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyEvent) {
        // If modal active, route to modal handler
        if self.modal.is_some() {
            self.handle_modal_key(key);
            return;
        }

        // '?' opens help anywhere
        if key.code == KeyCode::Char('?') {
            let ctx = match self.active_tab {
                Tab::Dots => {
                    if self.search_focused {
                        HelpContext::Dots
                    } else if self.detail_expanded.is_some() {
                        HelpContext::Detail
                    } else {
                        HelpContext::Dots
                    }
                }
                Tab::Log => HelpContext::Log,
            };
            self.open_help(ctx);
            return;
        }

        // Global shortcuts
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc if !self.search_focused => {
                self.should_quit = true;
            }
            KeyCode::Char('Q') if key.modifiers == KeyModifiers::CONTROL => {
                self.should_quit = true;
            }
            KeyCode::Char('1') => self.active_tab = Tab::Dots,
            KeyCode::Char('2') => self.active_tab = Tab::Log,
            KeyCode::Tab => {
                self.active_tab = match self.active_tab {
                    Tab::Dots => Tab::Log,
                    Tab::Log => Tab::Dots,
                }
            }
            _ => {}
        }

        // Search focus mode
        if self.search_focused {
            self.handle_search_key(key);
            return;
        }

        // Dots tab specific
        if self.active_tab == Tab::Dots {
            self.handle_dots_key(key);
        } else if self.active_tab == Tab::Log {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {}
                KeyCode::Down | KeyCode::Char('j') => {}
                _ => {}
            }
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.search_focused = false;
                self.search.clear();
                self.search_cursor = 0;
            }
            KeyCode::Enter => {
                self.search_focused = false;
            }
            KeyCode::Char(c) if !c.is_control() && key.modifiers.is_empty() => {
                let pos = self.search_cursor;
                self.search.insert(pos, c);
                self.search_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.search_cursor > 0 {
                    self.search.remove(self.search_cursor - 1);
                    self.search_cursor -= 1;
                }
            }
            KeyCode::Delete => {
                if self.search_cursor < self.search.len() {
                    self.search.remove(self.search_cursor);
                }
            }
            KeyCode::Left => {
                self.search_cursor = self.search_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                self.search_cursor = (self.search_cursor + 1).min(self.search.len());
            }
            KeyCode::Home => self.search_cursor = 0,
            KeyCode::End => self.search_cursor = self.search.len(),
            _ => {}
        }
    }

    fn handle_dots_key(&mut self, key: KeyEvent) {
        // Collect filtered results as owned data to avoid borrow conflicts
        let filtered: Vec<(usize, String)> = self
            .filtered_statuses()
            .into_iter()
            .map(|(i, s)| (i, s.name.clone()))
            .collect();
        let max = filtered.len().saturating_sub(1);

        // If detail is expanded, handle detail-specific keys first
        if self.detail_expanded.is_some() {
            match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    self.detail_expanded = None;
                    return;
                }
                KeyCode::Char('i') => {
                    // Install missing deps for expanded dot
                    if let Some(exp_idx) = self.detail_expanded {
                        let config = self.config.clone();
                        if let Some(ref cfg) = config {
                            if let Some(dot) = cfg.dots.get(exp_idx) {
                                if let Some(ref deps) = dot.deps {
                                    let missing: Vec<String> = deps
                                        .iter()
                                        .filter(|d| !deps::is_installed(d))
                                        .cloned()
                                        .collect();
                                    if !missing.is_empty() {
                                        self.add_log(
                                            format!("Installing {} missing dep(s)…", missing.len()),
                                            LogKind::Info,
                                        );
                                        let mut dot_cfg = cfg.clone();
                                        dot_cfg.dots.retain(|d| d.name == dot.name);
                                        self.ops_handle = Some(crate::ops::start_restore(dot_cfg, false));
                                    }
                                }
                            }
                        }
                    }
                    return;
                }
                KeyCode::Char('s') => {
                    if let Some(exp_idx) = self.detail_expanded {
                        self.skip_first_pending_setup(exp_idx);
                    }
                    return;
                }
                KeyCode::Char('r') => {
                    if let Some(exp_idx) = self.detail_expanded {
                        self.run_pending_setup(exp_idx);
                    }
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('/') => {
                self.search_focused = true;
                self.search_cursor = self.search.len();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_index = max.min(self.selected_index + 1);
            }
            KeyCode::Home => self.selected_index = 0,
            KeyCode::End => self.selected_index = max,
            KeyCode::Enter => {
                if let Some((actual_idx, _)) = filtered.get(self.selected_index) {
                    let idx = *actual_idx;
                    self.detail_expanded = if self.detail_expanded == Some(idx) {
                        None
                    } else {
                        Some(idx)
                    };
                }
            }
            KeyCode::Char('a') => self.open_add_form(),
            KeyCode::Char('e') => {
                if let Some((actual_idx, _)) = filtered.get(self.selected_index) {
                    self.open_edit_form(*actual_idx);
                }
            }
            KeyCode::Char('x') => {
                if let Some((_, name)) = filtered.get(self.selected_index) {
                    self.open_confirm_remove(name.clone());
                }
            }
            KeyCode::Char('b') => {
                if let Some((actual_idx, _)) = filtered.get(self.selected_index) {
                    self.active_tab = Tab::Log;
                    self.detail_expanded = None;
                    self.start_backup_dot(*actual_idx);
                }
            }
            KeyCode::Char('r') => {
                if let Some((actual_idx, _)) = filtered.get(self.selected_index) {
                    self.active_tab = Tab::Log;
                    self.detail_expanded = None;
                    self.open_dep_flow(*actual_idx);
                }
            }
            KeyCode::Char('B') => {
                self.active_tab = Tab::Log;
                self.detail_expanded = None;
                self.add_log("Backup all dotfiles…", LogKind::Info);
                let config = self.config.clone();
                if let Some(cfg) = config {
                    self.ops_handle = Some(crate::ops::start_backup(cfg, false));
                }
            }
            KeyCode::Char('R') => {
                self.active_tab = Tab::Log;
                self.detail_expanded = None;
                self.add_log("Restore all dotfiles…", LogKind::Info);
                let config = self.config.clone();
                if let Some(cfg) = config {
                    self.ops_handle = Some(crate::ops::start_restore(cfg, false));
                }
            }
            KeyCode::Char('S') => {
                self.open_settings();
            }
            _ => {}
        }
    }

    // ── Modal key handling ─────────────────────────────────────────────

    fn handle_modal_key(&mut self, key: KeyEvent) {
        // '?' in any modal opens help
        if key.code == KeyCode::Char('?') && !self.is_text_input_focused() {
            let ctx = match &self.modal {
                Some(ModalState::AddForm(_)) | Some(ModalState::EditForm(_, _)) => HelpContext::Form,
                Some(ModalState::Settings) => HelpContext::Settings,
                _ => HelpContext::Dots,
            };
            self.modal = Some(ModalState::HelpOverlay(ctx));
            return;
        }

        let mut modal = None;
        std::mem::swap(&mut modal, &mut self.modal);

        let (new_modal, action) = match modal {
            Some(ModalState::AddForm(mut f)) => {
                let action = self.handle_form_key(&mut f, key);
                if matches!(action, ModalAction::Save) {
                    self.save_form(&f, None);
                }
                (Some(ModalState::AddForm(f)), action)
            }
            Some(ModalState::EditForm(mut f, name)) => {
                let action = self.handle_form_key(&mut f, key);
                if matches!(action, ModalAction::Save) {
                    self.save_form(&f, Some(&name));
                }
                (Some(ModalState::EditForm(f, name)), action)
            }
            Some(ModalState::DepFlow(mut ws)) => {
                let (new_m, action) = self.handle_dep_flow_key(&mut ws, key);
                (new_m.map(|m| ModalState::DepFlow(m)), action)
            }
            Some(ModalState::Error(_)) => {
                (None, ModalAction::Close)
            }
            Some(ModalState::RemoveConfirm(name)) => {
                let action = match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.remove_dot_by_name(&name);
                        self.load_status();
                        self.load_diff();
                        self.add_log(format!("Removed {name}"), LogKind::Info);
                        ModalAction::Close
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => ModalAction::Close,
                    _ => ModalAction::Stay,
                };
                (None, action)
            }
            Some(ModalState::Confirm { message: _, action }) => {
                let act = match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.execute_confirm_action(action);
                        ModalAction::Close
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => ModalAction::Close,
                    _ => ModalAction::Stay,
                };
                (None, act)
            }
            Some(ModalState::HelpOverlay(_)) => {
                (None, ModalAction::Close)
            }
            Some(ModalState::Settings) => {
                let action = self.handle_settings_key(key);
                (Some(ModalState::Settings), action)
            }
            None => (None, ModalAction::Close),
        };

        if matches!(action, ModalAction::Stay) {
            self.modal = new_modal;
        } else {
            self.modal = None;
        }
    }

    fn is_text_input_focused(&self) -> bool {
        match &self.modal {
            Some(ModalState::AddForm(f)) | Some(ModalState::EditForm(f, _)) => {
                matches!(
                    f.fields.get(f.focused),
                    Some(FormField::Text { .. } | FormField::SetupSteps { .. })
                )
            }
            Some(ModalState::Settings) => {
                self.settings_form.as_ref().map(|s| s.focused == 0).unwrap_or(false)
            }
            _ => false,
        }
    }

    // ── Form key handling ──────────────────────────────────────────────

    fn handle_form_key(&self, form: &mut FormState, key: KeyEvent) -> ModalAction {
        // Check if any setup step has its check menu open
        let focused = form.focused;
        let show_menu = match form.fields.get(focused) {
            Some(FormField::SetupSteps { items, insert_index }) => {
                items.get(*insert_index).map(|item| item.show_check_menu).unwrap_or(false)
            }
            _ => false,
        };
        if show_menu {
            if let Some(FormField::SetupSteps { items, insert_index }) = form.fields.get_mut(focused) {
                Self::handle_check_menu_key(items, insert_index, key);
                return ModalAction::Stay;
            }
        }

        match key.code {
            KeyCode::Esc => return ModalAction::Close,
            KeyCode::Enter => {
                let is_setup = matches!(
                    form.fields.get(form.focused),
                    Some(FormField::SetupSteps { .. })
                );
                if is_setup {
                    // Fall through to field routing (adds step)
                } else {
                    return ModalAction::Save;
                }
            }
            KeyCode::Tab => {
                form.focused = (form.focused + 1) % form.fields.len();
                return ModalAction::Stay;
            }
            KeyCode::BackTab => {
                form.focused = if form.focused == 0 {
                    form.fields.len().saturating_sub(1)
                } else {
                    form.focused - 1
                };
                return ModalAction::Stay;
            }
            _ => {}
        }

        // Route key to focused field
        if let Some(field) = form.fields.get_mut(form.focused) {
            match field {
                FormField::Text { value, cursor, .. } => match key.code {
                    KeyCode::Char(c) if !c.is_control() && key.modifiers.is_empty() => {
                        value.insert(*cursor, c);
                        *cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if *cursor > 0 {
                            value.remove(*cursor - 1);
                            *cursor -= 1;
                        }
                    }
                    KeyCode::Delete => {
                        if *cursor < value.len() {
                            value.remove(*cursor);
                        }
                    }
                    KeyCode::Left => *cursor = cursor.saturating_sub(1),
                    KeyCode::Right => *cursor = (*cursor + 1).min(value.len()),
                    KeyCode::Home => *cursor = 0,
                    KeyCode::End => *cursor = value.len(),
                    _ => {}
                },
                FormField::Toggle { value, .. } => {
                    if key.code == KeyCode::Char(' ') || key.code == KeyCode::Enter {
                        *value = !*value;
                    }
                }
                FormField::SetupSteps { items, insert_index } => {
                    Self::handle_setup_key(items, insert_index, key);
                }
            }
        }

        ModalAction::Stay
    }

    // ── Settings key handling ──────────────────────────────────────────

    fn handle_settings_key(&mut self, key: KeyEvent) -> ModalAction {
        let Some(ref mut sf) = self.settings_form else {
            return ModalAction::Close;
        };
        match key.code {
            KeyCode::Esc => return ModalAction::Close,
            KeyCode::Enter => {
                // Save settings
                self.save_settings();
                return ModalAction::Close;
            }
            KeyCode::Tab => {
                sf.focused = (sf.focused + 1) % 3;
                sf.dirty = true;
                return ModalAction::Stay;
            }
            KeyCode::BackTab => {
                sf.focused = if sf.focused == 0 { 2 } else { sf.focused - 1 };
                sf.dirty = true;
                return ModalAction::Stay;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                match sf.focused {
                    1 => {
                        sf.os_index = sf.os_index.saturating_sub(1);
                        sf.dirty = true;
                    }
                    2 => {
                        sf.pkg_manager_index = sf.pkg_manager_index.saturating_sub(1);
                        sf.dirty = true;
                    }
                    _ => {}
                }
                return ModalAction::Stay;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                match sf.focused {
                    1 => {
                        sf.os_index = (sf.os_index + 1).min(SettingsForm::OS_OPTIONS.len() - 1);
                        sf.dirty = true;
                    }
                    2 => {
                        sf.pkg_manager_index = (sf.pkg_manager_index + 1).min(SettingsForm::PKG_OPTIONS.len() - 1);
                        sf.dirty = true;
                    }
                    _ => {}
                }
                return ModalAction::Stay;
            }
            _ => {}
        }

        // Vault path text input
        if sf.focused == 0 {
            match key.code {
                KeyCode::Char(c) if !c.is_control() && key.modifiers.is_empty() => {
                    sf.vault_path.insert(sf.vault_path_cursor, c);
                    sf.vault_path_cursor += 1;
                    sf.dirty = true;
                }
                KeyCode::Backspace => {
                    if sf.vault_path_cursor > 0 {
                        sf.vault_path.remove(sf.vault_path_cursor - 1);
                        sf.vault_path_cursor -= 1;
                        sf.dirty = true;
                    }
                }
                KeyCode::Delete => {
                    if sf.vault_path_cursor < sf.vault_path.len() {
                        sf.vault_path.remove(sf.vault_path_cursor);
                        sf.dirty = true;
                    }
                }
                KeyCode::Left => sf.vault_path_cursor = sf.vault_path_cursor.saturating_sub(1),
                KeyCode::Right => sf.vault_path_cursor = (sf.vault_path_cursor + 1).min(sf.vault_path.len()),
                KeyCode::Home => sf.vault_path_cursor = 0,
                KeyCode::End => sf.vault_path_cursor = sf.vault_path.len(),
                _ => {}
            }
        }

        ModalAction::Stay
    }

    fn save_settings(&mut self) {
        let Some(ref sf) = self.settings_form else { return };
        let os = SettingsForm::OS_OPTIONS[sf.os_index].to_string();
        let pkg = SettingsForm::PKG_OPTIONS[sf.pkg_manager_index].to_string();
        if let Some(ref mut config) = self.config {
            config.vault_path = sf.vault_path.clone();
            config.os = if os == "auto" { None } else { Some(os) };
            config.pkg_manager = if pkg == "auto" { None } else { Some(pkg) };
            match omah_lib::config::save_toml_config(config, &self.config_path) {
                Ok(()) => self.add_log("✓ Settings saved", LogKind::Success),
                Err(e) => self.add_log(format!("✗ Failed to save settings: {e}"), LogKind::Error),
            }
        }
        self.settings_form = None;
    }

    // ── Setup step key handling ────────────────────────────────────────

    fn handle_setup_key(items: &mut Vec<SetupFieldRow>, insert_index: &mut usize, key: KeyEvent) {
        if items.is_empty() {
            if key.code == KeyCode::Enter {
                items.push(SetupFieldRow {
                    install: String::new(),
                    check_type: "none".into(),
                    check_value: String::new(),
                    install_cursor: 0,
                    check_cursor: 0,
                    focused_install: true,
                    show_check_menu: false,
                    check_menu_index: 0,
                });
                *insert_index = 0;
            }
            return;
        }

        let Some(focused_item) = items.get_mut(*insert_index) else {
            *insert_index = items.len().saturating_sub(1);
            return;
        };

        match key.code {
            KeyCode::Tab => {
                if focused_item.focused_install {
                    focused_item.focused_install = false;
                } else {
                    // Move to next row or create new
                    *insert_index += 1;
                    if *insert_index >= items.len() {
                        items.push(SetupFieldRow {
                            install: String::new(),
                            check_type: "none".into(),
                            check_value: String::new(),
                            install_cursor: 0,
                            check_cursor: 0,
                            focused_install: true,
                            show_check_menu: false,
                            check_menu_index: 0,
                        });
                    }
                }
            }
            KeyCode::BackTab => {
                if !focused_item.focused_install {
                    focused_item.focused_install = true;
                    focused_item.show_check_menu = false;
                } else if *insert_index > 0 {
                    *insert_index -= 1;
                    if let Some(prev) = items.get_mut(*insert_index) {
                        prev.focused_install = false;
                    }
                }
            }
            KeyCode::Enter => {
                if !focused_item.focused_install {
                    // Open check type selector
                    focused_item.show_check_menu = true;
                    focused_item.check_menu_index = CHECK_TYPES
                        .iter()
                        .position(|(t, _, _)| *t == focused_item.check_type)
                        .unwrap_or(0);
                } else {
                    // Insert new step after current
                    items.insert(*insert_index + 1, SetupFieldRow {
                        install: String::new(),
                        check_type: "none".into(),
                        check_value: String::new(),
                        install_cursor: 0,
                        check_cursor: 0,
                        focused_install: true,
                        show_check_menu: false,
                        check_menu_index: 0,
                    });
                    *insert_index += 1;
                }
            }
            KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                // Delete current step
                if items.len() > 1 {
                    items.remove(*insert_index);
                    *insert_index = (*insert_index).min(items.len().saturating_sub(1));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if *insert_index > 0 {
                    *insert_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if *insert_index + 1 < items.len() {
                    *insert_index += 1;
                } else {
                    items.push(SetupFieldRow {
                        install: String::new(),
                        check_type: "none".into(),
                        check_value: String::new(),
                        install_cursor: 0,
                        check_cursor: 0,
                        focused_install: true,
                        show_check_menu: false,
                        check_menu_index: 0,
                    });
                    *insert_index = items.len() - 1;
                }
            }
            _ => {
                // Text input for active field
                let (target, target_cursor) = if focused_item.focused_install {
                    (&mut focused_item.install, &mut focused_item.install_cursor)
                } else {
                    (&mut focused_item.check_value, &mut focused_item.check_cursor)
                };
                match key.code {
                    KeyCode::Char(c) if !c.is_control() && key.modifiers.is_empty() => {
                        target.insert(*target_cursor, c);
                        *target_cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if *target_cursor > 0 {
                            target.remove(*target_cursor - 1);
                            *target_cursor -= 1;
                        }
                    }
                    KeyCode::Delete => {
                        if *target_cursor < target.len() {
                            target.remove(*target_cursor);
                        }
                    }
                    KeyCode::Left => *target_cursor = target_cursor.saturating_sub(1),
                    KeyCode::Right => *target_cursor = (*target_cursor + 1).min(target.len()),
                    KeyCode::Home => *target_cursor = 0,
                    KeyCode::End => *target_cursor = target.len(),
                    _ => {}
                }
            }
        }
    }

    fn handle_check_menu_key(items: &mut Vec<SetupFieldRow>, insert_index: &usize, key: KeyEvent) {
        let Some(item) = items.get_mut(*insert_index) else { return };
        match key.code {
            KeyCode::Esc => {
                item.show_check_menu = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                item.check_menu_index = item.check_menu_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                item.check_menu_index = (item.check_menu_index + 1).min(CHECK_TYPES.len() - 1);
            }
            KeyCode::Enter => {
                let (ct, _, _) = CHECK_TYPES[item.check_menu_index];
                item.check_type = ct.to_string();
                item.check_value.clear();
                item.show_check_menu = false;
            }
            _ => {}
        }
    }

    // ── Dep flow key handling ─────────────────────────────────────────

    fn handle_dep_flow_key(
        &mut self,
        ws: &mut crate::dep_flow::DepWorkspace,
        key: KeyEvent,
    ) -> (Option<crate::dep_flow::DepWorkspace>, ModalAction) {
        match key.code {
            KeyCode::Char(' ') => {
                // Toggle all checked/unchecked
                let all_checked = ws.missing_deps.iter().all(|d| d.checked)
                    && ws.setup_steps.iter().all(|s| s.checked);
                for dep in &mut ws.missing_deps {
                    dep.checked = !all_checked;
                }
                for step in &mut ws.setup_steps {
                    step.checked = !all_checked;
                }
                (Some(ws.clone()), ModalAction::Stay)
            }
            KeyCode::Char('a') => {
                for dep in &mut ws.missing_deps {
                    dep.checked = true;
                }
                for step in &mut ws.setup_steps {
                    step.checked = true;
                }
                (Some(ws.clone()), ModalAction::Stay)
            }
            KeyCode::Char('s') | KeyCode::Esc => {
                (None, ModalAction::Close)
            }
            KeyCode::Enter => {
                // Execute all checked items, then proceed with restore
                let dot_idx = self
                    .config
                    .as_ref()
                    .and_then(|c| c.dots.iter().position(|d| d.name == ws.dot_name));
                if let Some(idx) = dot_idx {
                    self.proceed_restore_dot(idx);
                }
                (None, ModalAction::Close)
            }
            _ => (Some(ws.clone()), ModalAction::Stay),
        }
    }

    // ── Inline step execution ──────────────────────────────────────────

    fn run_pending_setup(&mut self, dot_idx: usize) {
        let Some(ref dot) = self.config.as_ref().and_then(|c| c.dots.get(dot_idx)) else {
            return;
        };
        let pending = deps::pending_setup_steps(dot);
        let Some(first) = pending.first() else { return };

        let cmd = first.install.clone();
        let dot_name = dot.name.clone();
        self.add_log(format!("Running: {cmd}"), LogKind::Info);
        self.step_exec = Some((
            dot_name.clone(),
            StepExecState { running: true, done: false, success: false, output: vec![] },
        ));

        // Spawn thread to run command, send output back via channel
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let cmd_clone = cmd.clone();
        let step_exec_sender = tx.clone();
        std::thread::spawn(move || {
            match std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd_clone)
                .output()
            {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    for line in stdout.lines() {
                        let _ = step_exec_sender.send(line.to_string());
                    }
                    for line in stderr.lines() {
                        let _ = step_exec_sender.send(line.to_string());
                    }
                    if out.status.success() {
                        let _ = step_exec_sender.send("✓ Done".to_string());
                    } else {
                        let _ = step_exec_sender.send(format!("✗ Exit: {}", out.status));
                    }
                }
                Err(e) => {
                    let _ = step_exec_sender.send(format!("✗ Failed: {e}"));
                }
            }
        });

        // Store receiver for polling via tick
        self.setup_rx = Some(rx);
    }

    fn skip_first_pending_setup(&mut self, dot_idx: usize) {
        let step_install;
        let skipped;
        {
            let Some(ref mut config) = self.config else { return };
            let Some(dot) = config.dots.get_mut(dot_idx) else { return };
            let Some(ref mut steps) = dot.setup else { return };
            skipped = steps.iter_mut().find(|step| {
                let temp_dot = DotfileConfig {
                    name: dot.name.clone(),
                    source: dot.source.clone(),
                    id: dot.id.clone(),
                    symlink: dot.symlink,
                    deps: dot.deps.clone(),
                    setup: Some(vec![(*step).clone()]),
                    exclude: dot.exclude.clone(),
                };
                !deps::pending_setup_steps(&temp_dot).is_empty()
            });
            if let Some(step) = skipped {
                step.check = Some("skip".into());
                step_install = step.install.clone();
            } else {
                return;
            }
            let _ = omah_lib::config::save_toml_config(config, &self.config_path);
        }
        self.add_log(format!("Skipped: {step_install}"), LogKind::Info);
        self.load_status();
    }

    // ── Actions ────────────────────────────────────────────────────────

    pub fn start_backup_dot(&mut self, idx: usize) {
        let Some(ref config) = self.config.clone() else { return };
        let dot = config.dots.get(idx).map(|d| d.name.clone());

        let mut cfg = config.clone();
        if let Some(ref name) = dot {
            cfg.dots.retain(|d| d.name == *name);
        }

        self.add_log(format!("Backup '{}'…", dot.as_deref().unwrap_or("all")), LogKind::Info);
        self.ops_handle = Some(crate::ops::start_backup(cfg, false));
    }

    fn proceed_restore_dot(&mut self, idx: usize) {
        let Some(ref config) = self.config.clone() else { return };
        let dot = config.dots.get(idx).map(|d| d.name.clone());

        let mut cfg = config.clone();
        if let Some(ref name) = dot {
            cfg.dots.retain(|d| d.name == *name);
        }

        self.add_log(format!("Restore '{}'…", dot.as_deref().unwrap_or("all")), LogKind::Info);
        self.ops_handle = Some(crate::ops::start_restore(cfg, false));
    }

    fn remove_dot_by_name(&mut self, name: &str) {
        let Some(ref mut config) = self.config else { return };
        config.dots.retain(|d| d.name != name);
        if let Err(e) = omah_lib::config::save_toml_config(config, &self.config_path) {
            self.add_log(format!("✗ Failed to save config: {e}"), LogKind::Error);
        }
    }

    fn execute_confirm_action(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::Backup(i) => self.start_backup_dot(i),
            ConfirmAction::Restore(i) => self.proceed_restore_dot(i),
            ConfirmAction::RunBackupAll => {
                if let Some(ref config) = self.config {
                    self.ops_handle = Some(crate::ops::start_backup(config.clone(), false));
                }
            }
            ConfirmAction::RunRestoreAll => {
                if let Some(ref config) = self.config {
                    self.ops_handle = Some(crate::ops::start_restore(config.clone(), false));
                }
            }
        }
    }

    /// Save form data to config.
    pub fn save_form(&mut self, form: &FormState, original_name: Option<&str>) {
        let name = Self::get_field_value(form, 0);
        let source = Self::get_field_value(form, 1);
        let symlink = Self::get_toggle_value(form, 2);
        let deps_str = Self::get_field_value(form, 3);
        let exclude_str = Self::get_field_value(form, 4);
        let setup_items = Self::get_setup_items(form);

        if name.is_empty() {
            return;
        }

        let deps = if deps_str.is_empty() {
            None
        } else {
            Some(deps_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        };

        let exclude = if exclude_str.is_empty() {
            None
        } else {
            Some(exclude_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        };

        let setup = if setup_items.is_empty() {
            None
        } else {
            Some(
                setup_items
                    .into_iter()
                    .filter(|row| !row.install.is_empty())
                    .map(|row| omah_lib::SetupStep {
                        install: row.install,
                        check: serialize_check(&row.check_type, &row.check_value),
                    })
                    .collect(),
            )
        };

        let dot = DotfileConfig {
            name,
            source,
            id: Some(nanoid::nanoid!(8)),
            symlink: Some(symlink),
            deps,
            setup,
            exclude,
        };

        let Some(ref mut config) = self.config else { return };

        if let Some(orig) = original_name {
            if let Some(pos) = config.dots.iter().position(|d| d.name == orig) {
                config.dots[pos] = dot;
            }
        } else {
            config.dots.push(dot);
        }

        match omah_lib::config::save_toml_config(config, &self.config_path) {
            Ok(()) => {
                self.add_log("✓ Config saved", LogKind::Success);
                self.load_status();
                self.load_diff();
            }
            Err(e) => {
                self.add_log(format!("✗ Failed to save config: {e}"), LogKind::Error);
            }
        }
    }

    fn get_field_value(form: &FormState, idx: usize) -> String {
        match form.fields.get(idx) {
            Some(FormField::Text { value, .. }) => value.clone(),
            _ => String::new(),
        }
    }

    fn get_toggle_value(form: &FormState, idx: usize) -> bool {
        match form.fields.get(idx) {
            Some(FormField::Toggle { value, .. }) => *value,
            _ => false,
        }
    }

    fn get_setup_items(form: &FormState) -> Vec<SetupFieldRow> {
        for field in &form.fields {
            if let FormField::SetupSteps { items, .. } = field {
                return items
                    .iter()
                    .filter(|r| !r.install.is_empty())
                    .cloned()
                    .collect();
            }
        }
        vec![]
    }
}
