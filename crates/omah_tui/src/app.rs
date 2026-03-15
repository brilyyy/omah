use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use omah_core::{backup, load_toml_config, restore, status, DotStatus, OmahConfig};

// ── Screens ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum ConfirmKind {
    RestoreOne,
    RestoreAll,
}

pub enum Screen {
    Splash(Instant),
    List,
    AddForm,
    Confirm(ConfirmKind),
    Edit,
    Settings,
}

// ── Add-form state ─────────────────────────────────────────────────────────

pub struct FormState {
    pub field: usize, // 0 = name, 1 = source, 2 = symlink
    pub name: String,
    pub source: String,
    pub symlink: bool,
}

impl FormState {
    pub fn new() -> Self {
        Self { field: 0, name: String::new(), source: String::new(), symlink: false }
    }
    pub fn reset(&mut self) {
        *self = FormState::new();
    }
}

// ── Edit-screen state ──────────────────────────────────────────────────────

pub struct StepEdit {
    pub check: String,
    pub install: String,
}

pub struct StepFormState {
    pub field: usize, // 0 = check, 1 = install
    pub check: String,
    pub install: String,
}

impl StepFormState {
    pub fn new() -> Self {
        Self { field: 0, check: String::new(), install: String::new() }
    }
}

/// Focus positions in the edit screen:
/// 0 = Name, 1 = Source, 2 = Symlink, 3 = Deps, 4 = Setup steps, 5 = Excludes
pub struct EditState {
    pub dot_idx: usize,
    pub name: String,
    pub source: String,
    pub symlink: bool,
    pub deps: String,    // space-separated
    pub steps: Vec<StepEdit>,
    pub excludes: Vec<String>,
    pub focus: usize,
    pub step_sel: usize,
    pub exclude_sel: usize,
    pub step_form: Option<StepFormState>,
    pub exclude_input: Option<String>, // input buffer for new exclude pattern
}

impl Default for EditState {
    fn default() -> Self {
        Self {
            dot_idx: 0,
            name: String::new(),
            source: String::new(),
            symlink: false,
            deps: String::new(),
            steps: Vec::new(),
            excludes: Vec::new(),
            focus: 0,
            step_sel: 0,
            exclude_sel: 0,
            step_form: None,
            exclude_input: None,
        }
    }
}

// ── Settings-screen state ──────────────────────────────────────────────────

/// Focus: 0 = OS, 1 = Package Manager
pub struct SettingsState {
    pub os: String,
    pub pkg_manager: String,
    pub focus: usize,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self { os: "auto".to_string(), pkg_manager: "auto".to_string(), focus: 0 }
    }
}

// ── App ────────────────────────────────────────────────────────────────────

pub struct App {
    pub config_path: PathBuf,
    pub config: OmahConfig,
    pub statuses: Vec<DotStatus>,
    pub selected: usize,
    pub screen: Screen,
    pub form: FormState,
    pub edit: EditState,
    pub settings: SettingsState,
    pub message: Option<(String, bool)>, // (text, is_error)
}

impl App {
    pub fn new(config_path: PathBuf) -> anyhow::Result<Self> {
        let config = load_toml_config(&config_path)?;
        let statuses = status(&config).unwrap_or_default();
        Ok(Self {
            config_path,
            config,
            statuses,
            selected: 0,
            screen: Screen::Splash(Instant::now()),
            form: FormState::new(),
            edit: EditState::default(),
            settings: SettingsState::default(),
            message: None,
        })
    }

    pub fn reload(&mut self) {
        match load_toml_config(&self.config_path) {
            Ok(cfg) => {
                self.config = cfg;
                match status(&self.config) {
                    Ok(s) => self.statuses = s,
                    Err(e) => self.message = Some((e.to_string(), true)),
                }
            }
            Err(e) => self.message = Some((e.to_string(), true)),
        }
        if !self.statuses.is_empty() {
            self.selected = self.selected.min(self.statuses.len() - 1);
        } else {
            self.selected = 0;
        }
    }

    // ── Backup / restore ───────────────────────────────────────────────────

    pub fn backup_selected(&mut self) {
        if self.config.dots.is_empty() {
            return;
        }
        let dot = self.config.dots[self.selected].clone();
        let name = dot.name.clone();
        let single = OmahConfig {
            vault_path: self.config.vault_path.clone(),
            dots: vec![dot],
            git: None,
            os: None,
            pkg_manager: None,
        };
        match backup(&single) {
            Ok(()) => {
                self.reload();
                self.message = Some((format!("Backed up '{name}'"), false));
            }
            Err(e) => self.message = Some((e.to_string(), true)),
        }
    }

    pub fn backup_all(&mut self) {
        match backup(&self.config) {
            Ok(()) => {
                self.reload();
                self.message = Some(("All dotfiles backed up".into(), false));
            }
            Err(e) => self.message = Some((e.to_string(), true)),
        }
    }

    pub fn restore_selected(&mut self) {
        if self.config.dots.is_empty() {
            return;
        }
        let dot = self.config.dots[self.selected].clone();
        let name = dot.name.clone();
        let single = OmahConfig {
            vault_path: self.config.vault_path.clone(),
            dots: vec![dot],
            git: None,
            os: None,
            pkg_manager: None,
        };
        match restore(&single) {
            Ok(()) => {
                self.reload();
                self.message = Some((format!("Restored '{name}'"), false));
            }
            Err(e) => self.message = Some((e.to_string(), true)),
        }
    }

    pub fn restore_all(&mut self) {
        match restore(&self.config) {
            Ok(()) => {
                self.reload();
                self.message = Some(("All dotfiles restored".into(), false));
            }
            Err(e) => self.message = Some((e.to_string(), true)),
        }
    }

    // ── Add form ───────────────────────────────────────────────────────────

    pub fn save_new_dot(&mut self) {
        let name = self.form.name.trim().to_string();
        let source = self.form.source.trim().to_string();
        if name.is_empty() || source.is_empty() {
            self.message = Some(("Name and source are required".into(), true));
            return;
        }
        let mut entry = format!("\n[[dots]]\nname = {:?}\nsource = {:?}\n", name, source);
        if self.form.symlink {
            entry.push_str("symlink = true\n");
        }
        let result = OpenOptions::new()
            .append(true)
            .open(&self.config_path)
            .and_then(|mut f| f.write_all(entry.as_bytes()));

        match result {
            Ok(()) => {
                self.form.reset();
                self.screen = Screen::List;
                self.reload();
                self.message = Some((format!("Added '{name}'"), false));
            }
            Err(e) => self.message = Some((e.to_string(), true)),
        }
    }

    // ── Edit screen ────────────────────────────────────────────────────────

    pub fn open_edit(&mut self, idx: usize) {
        let dot = &self.config.dots[idx];
        let deps = dot.deps.as_deref().unwrap_or(&[]).join(" ");
        let steps = dot
            .setup
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|s| StepEdit {
                check: s.check.clone().unwrap_or_default(),
                install: s.install.clone(),
            })
            .collect();
        let excludes = dot.exclude.clone().unwrap_or_default();
        self.edit = EditState {
            dot_idx: idx,
            name: dot.name.clone(),
            source: dot.source.clone(),
            symlink: dot.symlink.unwrap_or(false),
            deps,
            steps,
            excludes,
            focus: 0,
            step_sel: 0,
            exclude_sel: 0,
            step_form: None,
            exclude_input: None,
        };
        self.screen = Screen::Edit;
        self.message = None;
    }

    pub fn save_edit(&mut self) {
        use toml_edit::{Array, DocumentMut, InlineTable};

        let result = (|| -> anyhow::Result<()> {
            let content = fs::read_to_string(&self.config_path)?;
            let mut doc: DocumentMut = content.parse()?;

            let dots = doc
                .get_mut("dots")
                .and_then(|i| i.as_array_of_tables_mut())
                .ok_or_else(|| anyhow::anyhow!("[[dots]] not found in config"))?;

            let entry = dots
                .get_mut(self.edit.dot_idx)
                .ok_or_else(|| anyhow::anyhow!("dot index out of bounds"))?;

            entry["name"] = toml_edit::value(self.edit.name.trim());
            entry["source"] = toml_edit::value(self.edit.source.trim());

            if self.edit.symlink {
                entry["symlink"] = toml_edit::value(true);
            } else {
                entry.remove("symlink");
            }

            let dep_list: Vec<&str> = self.edit.deps.split_whitespace().collect();
            if dep_list.is_empty() {
                entry.remove("deps");
            } else {
                let mut arr = Array::new();
                for d in dep_list {
                    arr.push(d);
                }
                entry["deps"] = toml_edit::value(arr);
            }

            let valid_steps: Vec<&StepEdit> =
                self.edit.steps.iter().filter(|s| !s.install.trim().is_empty()).collect();

            if valid_steps.is_empty() {
                entry.remove("setup");
            } else {
                let mut setup_arr = Array::new();
                for step in valid_steps {
                    let mut tbl = InlineTable::new();
                    if !step.check.trim().is_empty() {
                        tbl.insert("check", step.check.trim().into());
                    }
                    tbl.insert("install", step.install.trim().into());
                    setup_arr.push(tbl);
                }
                entry["setup"] = toml_edit::value(setup_arr);
            }

            let excl_list: Vec<&str> =
                self.edit.excludes.iter().map(|s| s.as_str()).collect();
            if excl_list.is_empty() {
                entry.remove("exclude");
            } else {
                let mut arr = Array::new();
                for e in excl_list {
                    arr.push(e);
                }
                entry["exclude"] = toml_edit::value(arr);
            }

            fs::write(&self.config_path, doc.to_string())?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                let name = self.edit.name.clone();
                self.screen = Screen::List;
                self.reload();
                self.message = Some((format!("Saved '{name}'"), false));
            }
            Err(e) => self.message = Some((e.to_string(), true)),
        }
    }

    // ── Settings screen ────────────────────────────────────────────────────

    pub fn open_settings(&mut self) {
        self.settings.os =
            self.config.os.clone().filter(|s| !s.is_empty()).unwrap_or_else(|| "auto".to_string());
        self.settings.pkg_manager = self
            .config
            .pkg_manager
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "auto".to_string());
        self.settings.focus = 0;
        self.screen = Screen::Settings;
        self.message = None;
    }

    pub fn save_settings(&mut self) {
        use toml_edit::DocumentMut;

        let result = (|| -> anyhow::Result<()> {
            let content = fs::read_to_string(&self.config_path)?;
            let mut doc: DocumentMut = content.parse()?;

            let os = self.settings.os.trim();
            if os.is_empty() || os == "auto" {
                doc.remove("os");
            } else {
                doc["os"] = toml_edit::value(os);
            }

            let pm = self.settings.pkg_manager.trim();
            if pm.is_empty() || pm == "auto" {
                doc.remove("pkg_manager");
            } else {
                doc["pkg_manager"] = toml_edit::value(pm);
            }

            fs::write(&self.config_path, doc.to_string())?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.screen = Screen::List;
                self.reload();
                self.message = Some(("Settings saved".into(), false));
            }
            Err(e) => self.message = Some((e.to_string(), true)),
        }
    }
}
