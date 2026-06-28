use crate::ops::run_install_cmd;
use omah_lib::{
    deps::{install_command, resolve_pkg_manager},
    DotfileConfig,
};

/// Orchestrates missing dep installation and setup step execution.
#[derive(Clone)]
pub struct DepWorkspace {
    pub dot_name: String,
    pub missing_deps: Vec<DepItem>,
    pub setup_steps: Vec<SetupItem>,
    pub pkg_manager: Option<String>,
    pub install_cmd: Option<String>,
    /// Index of currently running item (None = idle)
    pub running_index: Option<usize>,
    /// Total steps completed successfully
    pub done_count: usize,
    pub total_count: usize,
    pub all_done: bool,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct DepItem {
    pub pkg: String,
    pub checked: bool,
    pub installed: bool,
}

#[derive(Clone)]
pub struct SetupItem {
    pub install: String,
    pub check: Option<String>,
    pub checked: bool,
    pub done: bool,
}

impl DepWorkspace {
    pub fn new(dot: &DotfileConfig) -> Self {
        let missing: Vec<String> = omah_lib::deps::missing_deps(dot);
        let pm = resolve_pkg_manager(None);
        let install_cmd = pm.as_ref().map(|pm| install_command(pm, &missing));

        let dep_items: Vec<DepItem> = missing
            .iter()
            .map(|p| DepItem {
                pkg: p.clone(),
                checked: true,
                installed: false,
            })
            .collect();

        let pending = omah_lib::deps::pending_setup_steps(dot);
        let setup_items: Vec<SetupItem> = pending
            .iter()
            .map(|s| SetupItem {
                install: s.install.clone(),
                check: s.check.clone(),
                checked: true,
                done: false,
            })
            .collect();

        let total = dep_items.len() + setup_items.len();

        Self {
            dot_name: dot.name.clone(),
            missing_deps: dep_items,
            setup_steps: setup_items,
            pkg_manager: pm,
            install_cmd,
            running_index: None,
            done_count: 0,
            total_count: total,
            all_done: false,
            error: None,
        }
    }

    /// Run one install step synchronously.
    /// Call from a background thread, not the event loop.
    #[allow(dead_code)]
    pub fn execute_next(&mut self) -> String {
        // Find first unchecked, not-done item that's checked
        if let Some(dep) = self
            .missing_deps
            .iter_mut()
            .find(|d| d.checked && !d.installed)
        {
            dep.installed = true; // mark to avoid re-run
            let cmd = self
                .pkg_manager
                .as_ref()
                .map(|pm| install_command(pm, &[dep.pkg.clone()]))
                .unwrap_or_default();
            match run_install_cmd(&cmd) {
                Ok(()) => {
                    self.done_count += 1;
                    format!("✓ {} installed", dep.pkg)
                }
                Err(e) => {
                    self.error = Some(format!("Failed: {}: {e}", dep.pkg));
                    format!("✗ {} failed: {e}", dep.pkg)
                }
            }
        } else if let Some(step) = self
            .setup_steps
            .iter_mut()
            .find(|s| s.checked && !s.done)
        {
            step.done = true;
            match run_install_cmd(&step.install) {
                Ok(()) => {
                    self.done_count += 1;
                    format!("✓ {} done", step.install)
                }
                Err(e) => {
                    self.error = Some(format!("Failed: {}: {e}", step.install));
                    format!("✗ {} failed: {e}", step.install)
                }
            }
        } else {
            self.all_done = true;
            "All steps complete.".to_string()
        }
    }
}
