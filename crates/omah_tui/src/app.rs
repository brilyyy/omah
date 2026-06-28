use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use omah_lib::{
    config::load_toml_config,
    ops::{diff, status, DotStatus, FileChange},
    OmahConfig,
};

use crate::{
    dep_flow::DepWorkspace,
    ops::{OpsHandle, OpsMessage},
};

// ── Tab ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tab {
    Status = 0,
    Diff = 1,
    Details = 2,
    Log = 3,
}

impl Tab {
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn label(self) -> &'static str {
        match self {
            Tab::Status => "Status",
            Tab::Diff => "Diff",
            Tab::Details => "Details",
            Tab::Log => "Log",
        }
    }
    pub fn all() -> &'static [Tab] {
        &[Tab::Status, Tab::Diff, Tab::Details, Tab::Log]
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
    pub check: String,
    pub install_cursor: usize,
    pub check_cursor: usize,
    pub focused_install: bool,
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
}

pub enum ConfirmAction {
    Backup(usize),
    Restore(usize),
    RunBackupAll,
    RunRestoreAll,
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

    // Log
    pub log_entries: Vec<LogEntry>,

    // Modal
    pub modal: Option<ModalState>,

    // Background operations
    pub ops_handle: Option<OpsHandle>,
}

impl App {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            config: None,
            active_tab: Tab::Status,
            should_quit: false,
            tick_counter: 0,
            selected_index: 0,
            statuses: vec![],
            changes: vec![],
            log_entries: vec![],
            modal: None,
            ops_handle: None,
        }
    }

    // ── Data loading ────────────────────────────────────────────────────

    pub fn load_config(&mut self) {
        match load_toml_config(&self.config_path) {
            Ok(cfg) => {
                self.config = Some(cfg);
                self.load_status();
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
    }

    // ── Modal open helpers ─────────────────────────────────────────────

    pub fn open_error(&mut self, msg: String) {
        self.modal = Some(ModalState::Error(msg));
    }

    pub fn open_confirm_remove(&mut self, name: String) {
        self.modal = Some(ModalState::RemoveConfirm(name));
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
                            .map(|s| SetupFieldRow {
                                install: s.install.clone(),
                                check: s.check.clone().unwrap_or_default(),
                                install_cursor: s.install.len(),
                                check_cursor: s.check.as_ref().map(|c| c.len()).unwrap_or(0),
                                focused_install: true,
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
            // No deps to resolve, proceed directly
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

        // Global shortcuts
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('Q') if key.modifiers == KeyModifiers::CONTROL => {
                self.should_quit = true;
            }
            KeyCode::Char('1') => self.active_tab = Tab::Status,
            KeyCode::Char('2') => self.active_tab = Tab::Diff,
            KeyCode::Char('3') => self.active_tab = Tab::Details,
            KeyCode::Char('4') => self.active_tab = Tab::Log,
            KeyCode::Tab => {
                self.active_tab = match self.active_tab {
                    Tab::Status => Tab::Diff,
                    Tab::Diff => Tab::Details,
                    Tab::Details => Tab::Log,
                    Tab::Log => Tab::Status,
                }
            }
            KeyCode::BackTab => {
                self.active_tab = match self.active_tab {
                    Tab::Status => Tab::Log,
                    Tab::Diff => Tab::Status,
                    Tab::Details => Tab::Diff,
                    Tab::Log => Tab::Details,
                }
            }

            // Tab-specific
            KeyCode::Up | KeyCode::Char('k') => {
                if self.active_tab == Tab::Status || self.active_tab == Tab::Details {
                    self.selected_index = self.selected_index.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.active_tab == Tab::Status || self.active_tab == Tab::Details {
                    let max = self.statuses.len().saturating_sub(1);
                    self.selected_index = self.selected_index.min(max).saturating_add(1).min(max);
                }
            }
            KeyCode::Enter => {
                match self.active_tab {
                    Tab::Status | Tab::Details => {
                        self.active_tab = Tab::Details;
                    }
                    Tab::Diff => {}
                    Tab::Log => {}
                }
            }

            // Status / Detail actions
            KeyCode::Char('a') => self.open_add_form(),
            KeyCode::Char('e') => {
                if !self.statuses.is_empty() {
                    self.open_edit_form(self.selected_index);
                }
            }
            KeyCode::Char('x') => {
                if let Some(s) = self.statuses.get(self.selected_index) {
                    self.open_confirm_remove(s.name.clone());
                }
            }
            KeyCode::Char('b') => {
                if !self.statuses.is_empty() {
                    let idx = self.selected_index.min(self.statuses.len().saturating_sub(1));
                    self.active_tab = Tab::Log;
                    self.start_backup_dot(idx);
                }
            }
            KeyCode::Char('r') => {
                if !self.statuses.is_empty() {
                    let idx = self.selected_index.min(self.statuses.len().saturating_sub(1));
                    self.active_tab = Tab::Log;
                    self.open_dep_flow(idx);
                }
            }
            KeyCode::Char('d') => {
                self.load_diff();
                self.active_tab = Tab::Diff;
            }
            KeyCode::Char('i') => {
                if !self.statuses.is_empty() {
                    self.active_tab = Tab::Details;
                }
            }

            _ => {}
        }
    }

    // ── Modal key handling ─────────────────────────────────────────────

    fn handle_modal_key(&mut self, key: KeyEvent) {
        // Take the modal to avoid borrow issues
        let mut modal = None;
        std::mem::swap(&mut modal, &mut self.modal);

        let (new_modal, action) = match modal {
            Some(ModalState::AddForm(mut f)) => {
                let action = self.handle_form_key(&mut f, key);
                (Some(ModalState::AddForm(f)), action)
            }
            Some(ModalState::EditForm(mut f, name)) => {
                let action = self.handle_form_key(&mut f, key);
                (Some(ModalState::EditForm(f, name)), action)
            }
            Some(ModalState::DepFlow(mut ws)) => {
                let (new_m, action) = self.handle_dep_flow_key(&mut ws, key);
                (new_m.map(|m| ModalState::DepFlow(m)), action)
            }
            Some(ModalState::Error(_)) => {
                // Any key dismisses error
                (None, ModalAction::Close)
            }
            Some(ModalState::RemoveConfirm(name)) => {
                let action = match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.remove_dot_by_name(&name);
                        self.load_status();
                        self.add_log(format!("Removed {name}"), LogKind::Info);
                        ModalAction::Close
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        ModalAction::Close
                    }
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
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        ModalAction::Close
                    }
                    _ => ModalAction::Stay,
                };
                (None, act)
            }
            None => (None, ModalAction::Close),
        };

        if matches!(action, ModalAction::Stay) {
            self.modal = new_modal;
        } else {
            self.modal = new_modal;
            if matches!(action, ModalAction::Close) {
                self.modal = None;
            }
        }
    }

    fn handle_form_key(&self, form: &mut FormState, key: KeyEvent) -> ModalAction {
        match key.code {
            KeyCode::Esc => return ModalAction::Close,
            KeyCode::Enter => {
                // Save
                return ModalAction::Close; // caller handles save
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
                    KeyCode::Char(c) if !c.is_control() => {
                        let pos = *cursor;
                        value.insert(pos, c);
                        *cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if *cursor > 0 {
                            let pos = *cursor - 1;
                            value.remove(pos);
                            *cursor = pos;
                        }
                    }
                    KeyCode::Delete => {
                        if *cursor < value.len() {
                            value.remove(*cursor);
                        }
                    }
                    KeyCode::Left => {
                        *cursor = cursor.saturating_sub(1);
                    }
                    KeyCode::Right => {
                        *cursor = (*cursor + 1).min(value.len());
                    }
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

    fn handle_setup_key(items: &mut Vec<SetupFieldRow>, insert_index: &mut usize, key: KeyEvent) {
        // Find focused item
        let Some(focused_item) = items.get_mut(*insert_index) else {
            // No items yet, create first on Enter
            if key.code == KeyCode::Enter {
                items.push(SetupFieldRow {
                    install: String::new(),
                    check: String::new(),
                    install_cursor: 0,
                    check_cursor: 0,
                    focused_install: true,
                });
                *insert_index = items.len();
            }
            return;
        };

        match key.code {
            KeyCode::Tab => {
                // Toggle between install/check within item, or move to next item
                if focused_item.focused_install {
                    focused_item.focused_install = false;
                } else if *insert_index < items.len() {
                    *insert_index += 1;
                    if *insert_index >= items.len() {
                        // Add new empty row
                        items.push(SetupFieldRow {
                            install: String::new(),
                            check: String::new(),
                            install_cursor: 0,
                            check_cursor: 0,
                            focused_install: true,
                        });
                    }
                }
            }
            KeyCode::BackTab => {
                if !focused_item.focused_install {
                    focused_item.focused_install = true;
                } else if *insert_index > 0 {
                    *insert_index -= 1;
                }
            }
            KeyCode::Enter => {
                // Add new step after current
                items.insert(*insert_index + 1, SetupFieldRow {
                    install: String::new(),
                    check: String::new(),
                    install_cursor: 0,
                    check_cursor: 0,
                    focused_install: true,
                });
                *insert_index += 1;
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
                    // Add new row
                    items.push(SetupFieldRow {
                        install: String::new(),
                        check: String::new(),
                        install_cursor: 0,
                        check_cursor: 0,
                        focused_install: true,
                    });
                    *insert_index = items.len() - 1;
                }
            }
            _ => {
                // Text input
                let target = if focused_item.focused_install {
                    &mut focused_item.install
                } else {
                    &mut focused_item.check
                };
                let target_cursor = if focused_item.focused_install {
                    &mut focused_item.install_cursor
                } else {
                    &mut focused_item.check_cursor
                };
                match key.code {
                    KeyCode::Char(c) if !c.is_control() => {
                        let pos = *target_cursor;
                        target.insert(pos, c);
                        *target_cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if *target_cursor > 0 {
                            let pos = *target_cursor - 1;
                            target.remove(pos);
                            *target_cursor = pos;
                        }
                    }
                    KeyCode::Delete => {
                        if *target_cursor < target.len() {
                            target.remove(*target_cursor);
                        }
                    }
                    KeyCode::Left => {
                        *target_cursor = target_cursor.saturating_sub(1);
                    }
                    KeyCode::Right => {
                        *target_cursor = (*target_cursor + 1).min(target.len());
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_dep_flow_key(&mut self, ws: &mut DepWorkspace, key: KeyEvent) -> (Option<DepWorkspace>, ModalAction) {
        match key.code {
            KeyCode::Esc => return (None, ModalAction::Close),
            KeyCode::Char(' ') => {
                // Toggle all checked state
                let all_checked = ws.missing_deps.iter().all(|d| d.checked)
                    && ws.setup_steps.iter().all(|s| s.checked);
                let new_state = !all_checked;
                for dep in &mut ws.missing_deps {
                    dep.checked = new_state;
                }
                for step in &mut ws.setup_steps {
                    step.checked = new_state;
                }
            }
            KeyCode::Char('a') => {
                // Select all
                for dep in &mut ws.missing_deps {
                    dep.checked = true;
                }
                for step in &mut ws.setup_steps {
                    step.checked = true;
                }
            }
            KeyCode::Enter => {
                // Execute all checked items in a thread
                let mut ws_clone = ws.clone();
                std::thread::spawn(move || {
                    // Run deps first, then setup steps
                    for dep in ws_clone.missing_deps.iter_mut() {
                        if dep.checked && !dep.installed {
                            dep.installed = true;
                            let pm = ws_clone.pkg_manager.clone();
                            let cmd = pm
                                .as_ref()
                                .map(|pm| omah_lib::deps::install_command(pm, &[dep.pkg.clone()]))
                                .unwrap_or_default();
                            if !cmd.is_empty() {
                                let _ = std::process::Command::new("sh")
                                    .arg("-c")
                                    .arg(&cmd)
                                    .status();
                            }
                        }
                    }
                    for step in ws_clone.setup_steps.iter_mut() {
                        if step.checked && !step.done {
                            step.done = true;
                            let _ = std::process::Command::new("sh")
                                .arg("-c")
                                .arg(&step.install)
                                .status();
                        }
                    }
                });
                // Close modal and start restore
                let dot_name = ws.dot_name.clone();
                let idx = self.statuses.iter().position(|s| s.name == dot_name);
                if let Some(i) = idx {
                    self.proceed_restore_dot(i);
                }
            }
            KeyCode::Char('s') => {
                // Skip — proceed to restore without deps
                let dot_name = ws.dot_name.clone();
                let idx = self.statuses.iter().position(|s| s.name == dot_name);
                if let Some(i) = idx {
                    self.proceed_restore_dot(i);
                }
            }
            _ => {}
        }
        (None, ModalAction::Close)
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

        // Save config
        if let Err(e) = omah_lib::config::save_toml_config(config, &self.config_path) {
            self.add_log(format!("✗ Failed to save config: {e}"), LogKind::Error);
        }
    }

    fn execute_confirm_action(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::Backup(i) => self.start_backup_dot(i),
            ConfirmAction::Restore(i) => self.proceed_restore_dot(i),
            ConfirmAction::RunBackupAll => self.start_backup_dot(usize::MAX),
            ConfirmAction::RunRestoreAll => self.proceed_restore_dot(usize::MAX),
        }
    }

    /// Save form data to config (called from ui when form submits).
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
                    .map(|row| omah_lib::SetupStep {
                        install: row.install,
                        check: if row.check.is_empty() { None } else { Some(row.check) },
                    })
                    .collect(),
            )
        };

        let dot = omah_lib::DotfileConfig {
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
            // Edit: replace existing
            if let Some(pos) = config.dots.iter().position(|d| d.name == orig) {
                config.dots[pos] = dot;
            }
        } else {
            // Add: append
            config.dots.push(dot);
        }

        match omah_lib::config::save_toml_config(config, &self.config_path) {
            Ok(()) => {
                self.add_log("✓ Config saved", LogKind::Success);
                self.load_status();
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

/// Modal-specific action result.
enum ModalAction {
    /// Modal remains open
    Stay,
    /// Modal should close (set self.modal = None)
    Close,
}
