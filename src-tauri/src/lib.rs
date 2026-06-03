use omah_core::{
    backup, diff, get_default_config_path, load_toml_config, restore, save_toml_config, status,
    DotStatus, FileChange, OmahConfig,
};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tracing::{error, info, instrument};

// ── App settings ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AppSettings {
    #[serde(default)]
    pub run_in_tray: bool,
    #[serde(default)]
    pub auto_update: bool,
}

fn app_settings_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    std::path::PathBuf::from(home).join(".config/omah/app-settings.json")
}

fn load_app_settings() -> AppSettings {
    let path = app_settings_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn persist_app_settings(settings: &AppSettings) -> Result<(), String> {
    let path = app_settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

// ── Update check ─────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
}

fn semver_gt(remote: &str, current: &str) -> bool {
    fn parse(v: &str) -> [u32; 3] {
        let parts: Vec<u32> = v.split('.').filter_map(|x| x.parse().ok()).collect();
        [
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        ]
    }
    parse(remote) > parse(current)
}

async fn fetch_update_info() -> Result<Option<UpdateInfo>, String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("omah/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let resp: serde_json::Value = client
        .get("https://api.github.com/repos/brilyyy/omah/releases/latest")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let tag = resp["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v');

    let current = env!("CARGO_PKG_VERSION");

    if semver_gt(tag, current) {
        let url = resp["html_url"]
            .as_str()
            .unwrap_or("https://github.com/brilyyy/omah/releases")
            .to_string();
        Ok(Some(UpdateInfo {
            version: tag.to_string(),
            url,
        }))
    } else {
        Ok(None)
    }
}

// ── Tray ─────────────────────────────────────────────────────────────────────

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let show = MenuItem::with_id(app, "show", "Show omah", true, None::<&str>)?;
    let check_upd =
        MenuItem::with_id(app, "check_update", "Check for Updates…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit omah", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show,
            &PredefinedMenuItem::separator(app)?,
            &check_upd,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("omah — dotfile manager")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "check_update" => {
                show_main_window(app);
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.emit("tray-check-update", ());
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

// ── Config helper ─────────────────────────────────────────────────────────────

fn load_config() -> Result<OmahConfig, String> {
    let path = get_default_config_path().map_err(|e| e.to_string())?;
    if !path.exists() {
        omah_core::init_setup().map_err(|e| e.to_string())?;
    }
    load_toml_config(&path).map_err(|e| e.to_string())
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct RunResult {
    pub success: bool,
    pub output: String,
}

#[derive(Clone, Serialize)]
struct SetupStepOutputEvent {
    run_id: String,
    line: String,
    is_stderr: bool,
    done: bool,
    success: Option<bool>,
}

// ── Commands — app settings ───────────────────────────────────────────────────

#[tauri::command]
fn get_app_settings() -> AppSettings {
    load_app_settings()
}

#[tauri::command]
fn save_app_settings(settings: AppSettings) -> Result<(), String> {
    persist_app_settings(&settings)
}

// ── Commands — update ─────────────────────────────────────────────────────────

#[tauri::command]
async fn check_update() -> Result<Option<UpdateInfo>, String> {
    fetch_update_info().await
}

// ── Commands — dotfiles ───────────────────────────────────────────────────────

#[tauri::command]
#[instrument]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
#[instrument]
fn get_config() -> Result<OmahConfig, String> {
    info!("get_config");
    load_config()
}

#[tauri::command]
#[instrument]
fn get_status() -> Result<Vec<DotStatus>, String> {
    info!("get_status");
    let config = load_config()?;
    status(&config).map_err(|e| {
        error!("{e}");
        e.to_string()
    })
}

#[tauri::command]
#[instrument]
fn backup_all() -> Result<(), String> {
    info!("backup_all");
    let config = load_config()?;
    backup(&config).map_err(|e| {
        error!("{e}");
        e.to_string()
    })
}

#[tauri::command]
#[instrument]
fn restore_all() -> Result<(), String> {
    info!("restore_all");
    let config = load_config()?;
    restore(&config).map_err(|e| {
        error!("{e}");
        e.to_string()
    })
}

#[tauri::command]
#[instrument]
fn backup_one(name: String) -> Result<(), String> {
    info!("backup_one: {name}");
    let config = load_config()?;
    let dot = config
        .dots
        .iter()
        .find(|d| d.name == name)
        .ok_or_else(|| format!("Dotfile '{name}' not found"))?
        .clone();
    let single = OmahConfig {
        dots: vec![dot],
        ..config
    };
    backup(&single).map_err(|e| {
        error!("{e}");
        e.to_string()
    })
}

#[tauri::command]
#[instrument]
fn restore_one(name: String) -> Result<(), String> {
    info!("restore_one: {name}");
    let config = load_config()?;
    let dot = config
        .dots
        .iter()
        .find(|d| d.name == name)
        .ok_or_else(|| format!("Dotfile '{name}' not found"))?
        .clone();
    let single = OmahConfig {
        dots: vec![dot],
        ..config
    };
    restore(&single).map_err(|e| {
        error!("{e}");
        e.to_string()
    })
}

#[tauri::command]
#[instrument]
fn save_config(config: OmahConfig) -> Result<(), String> {
    info!("save_config");
    let path = get_default_config_path().map_err(|e| e.to_string())?;
    save_toml_config(&config, &path).map_err(|e| {
        error!("{e}");
        e.to_string()
    })
}

#[tauri::command]
#[instrument]
fn get_diff() -> Result<Vec<FileChange>, String> {
    info!("get_diff");
    let config = load_config()?;
    diff(&config).map_err(|e| {
        error!("{e}");
        e.to_string()
    })
}

#[tauri::command]
#[instrument(skip(command))]
async fn run_setup_step(command: String) -> Result<RunResult, String> {
    info!("run_setup_step");
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .await
        .map_err(|e| {
            error!("{e}");
            format!("Failed to spawn process: {e}")
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let combined = match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n{stderr}"),
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => String::new(),
    };

    Ok(RunResult {
        success: output.status.success(),
        output: combined,
    })
}

// ── Streaming helpers ─────────────────────────────────────────────────────────

fn emit_line(
    window: &tauri::WebviewWindow,
    run_id: &str,
    line: impl Into<String>,
    is_stderr: bool,
) {
    let _ = window.emit(
        "setup_step_output",
        SetupStepOutputEvent {
            run_id: run_id.to_string(),
            line: line.into(),
            is_stderr,
            done: false,
            success: None,
        },
    );
}

fn emit_done(window: &tauri::WebviewWindow, run_id: &str, success: bool) {
    let _ = window.emit(
        "setup_step_output",
        SetupStepOutputEvent {
            run_id: run_id.to_string(),
            line: String::new(),
            is_stderr: false,
            done: true,
            success: Some(success),
        },
    );
}

async fn stream_command(
    window: &tauri::WebviewWindow,
    run_id: &str,
    command: &str,
) -> Result<bool, String> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::sync::mpsc;

    let mut child = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn process: {e}"))?;

    let stdout = child.stdout.take().expect("stdout not captured");
    let stderr = child.stderr.take().expect("stderr not captured");

    let (tx, mut rx) = mpsc::channel::<(String, bool)>(256);
    let tx_out = tx.clone();
    let tx_err = tx.clone();
    drop(tx);

    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = tx_out.send((line, false)).await;
        }
    });

    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = tx_err.send((line, true)).await;
        }
    });

    while let Some((line, is_stderr)) = rx.recv().await {
        emit_line(window, run_id, line, is_stderr);
    }

    child
        .wait()
        .await
        .map(|s| s.success())
        .map_err(|e| e.to_string())
}

// ── Streaming commands ────────────────────────────────────────────────────────

#[tauri::command]
#[instrument(skip(window, command))]
async fn run_setup_step_stream(
    window: tauri::WebviewWindow,
    run_id: String,
    command: String,
) -> Result<(), String> {
    info!("run_setup_step_stream");
    let success = stream_command(&window, &run_id, &command).await?;
    emit_done(&window, &run_id, success);
    Ok(())
}

#[tauri::command]
#[instrument(skip(window))]
async fn install_missing_deps(
    window: tauri::WebviewWindow,
    run_id: String,
    name: String,
) -> Result<(), String> {
    info!("install_missing_deps: {name}");
    let config = load_config()?;
    let dot = config
        .dots
        .iter()
        .find(|d| d.name == name)
        .ok_or_else(|| format!("Dotfile '{name}' not found"))?;

    let missing = omah_core::missing_deps(dot);
    if missing.is_empty() {
        emit_line(
            &window,
            &run_id,
            "✓ All dependencies are already installed.",
            false,
        );
        emit_done(&window, &run_id, true);
        return Ok(());
    }

    let pm = omah_core::resolve_pkg_manager(config.pkg_manager.as_deref()).ok_or_else(|| {
        "No package manager found in PATH (tried brew, apt-get, pacman, dnf, zypper)".to_string()
    })?;
    let command = omah_core::install_command(&pm, &missing);

    emit_line(&window, &run_id, format!("$ {command}"), false);
    let success = stream_command(&window, &run_id, &command).await?;
    emit_done(&window, &run_id, success);
    Ok(())
}

#[tauri::command]
#[instrument(skip(window))]
async fn run_pending_setups(
    window: tauri::WebviewWindow,
    run_id: String,
    name: String,
) -> Result<(), String> {
    info!("run_pending_setups: {name}");
    let config = load_config()?;
    let dot = config
        .dots
        .iter()
        .find(|d| d.name == name)
        .ok_or_else(|| format!("Dotfile '{name}' not found"))?;

    let pending: Vec<String> = omah_core::pending_setup_steps(dot)
        .into_iter()
        .map(|s| s.install.clone())
        .collect();

    if pending.is_empty() {
        emit_line(
            &window,
            &run_id,
            "✓ All setup steps are already done.",
            false,
        );
        emit_done(&window, &run_id, true);
        return Ok(());
    }

    let total = pending.len();
    let mut all_ok = true;

    for (i, cmd) in pending.iter().enumerate() {
        emit_line(
            &window,
            &run_id,
            format!("─── step {}/{total} ───", i + 1),
            false,
        );
        emit_line(&window, &run_id, format!("$ {cmd}"), false);

        let success = stream_command(&window, &run_id, cmd).await?;
        if !success {
            all_ok = false;
            emit_line(
                &window,
                &run_id,
                format!("✗ step {} failed — stopping", i + 1),
                true,
            );
            break;
        }
    }

    emit_done(&window, &run_id, all_ok);
    Ok(())
}

// ── App entry ─────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,omah_desktop=debug".into()),
        )
        .with_target(false)
        .init();

    tracing::info!("omah desktop v{}", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            setup_tray(app.handle())?;

            // Hide to tray on close when run_in_tray is enabled
            if let Some(window) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if load_app_settings().run_in_tray {
                            if let Some(w) = handle.get_webview_window("main") {
                                let _ = w.hide();
                            }
                            api.prevent_close();
                        }
                    }
                });
            }

            // Background update check on startup
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // Small delay so the window finishes loading first
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                if load_app_settings().auto_update {
                    match fetch_update_info().await {
                        Ok(Some(info)) => {
                            if let Some(w) = handle.get_webview_window("main") {
                                let _ = w.emit("update-available", info);
                            }
                        }
                        Ok(None) => {}
                        Err(e) => tracing::warn!("update check failed: {e}"),
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_version,
            get_config,
            save_config,
            get_status,
            backup_all,
            restore_all,
            backup_one,
            restore_one,
            get_diff,
            run_setup_step,
            run_setup_step_stream,
            install_missing_deps,
            run_pending_setups,
            get_app_settings,
            save_app_settings,
            check_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
