use omah_core::{
    backup, diff, get_default_config_path, load_toml_config, restore, save_toml_config, status,
    DotStatus, FileChange, OmahConfig,
};
use serde::Serialize;
use tauri::Emitter;
use tracing::{error, info, instrument};

// ── Config helper ───────────────────────────────────────────────────────────

fn load_config() -> Result<OmahConfig, String> {
    let path = get_default_config_path().map_err(|e| e.to_string())?;
    load_toml_config(&path).map_err(|e| e.to_string())
}

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct RunResult {
    pub success: bool,
    /// Combined stdout + stderr output of the process.
    pub output: String,
}

/// Payload emitted for each line during a streamed setup step run.
#[derive(Clone, Serialize)]
struct SetupStepOutputEvent {
    run_id: String,
    line: String,
    is_stderr: bool,
    done: bool,
    success: Option<bool>,
}

// ── Commands ────────────────────────────────────────────────────────────────

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

/// Run an arbitrary shell command via `sh -c` and return its output.
/// Used to execute setup steps defined in the user's config.
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

// ── Streaming helpers ────────────────────────────────────────────────────────

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

/// Spawn a shell command, stream each stdout/stderr line as events, and return
/// whether the process exited successfully. Does NOT emit a `done` event.
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

// ── Streaming commands ───────────────────────────────────────────────────────

/// Run a setup step and stream each output line as a Tauri event.
/// The frontend subscribes to `setup_step_output` events filtered by `run_id`.
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

/// Install all missing deps for a dotfile and stream the output.
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

/// Run all pending setup steps for a dotfile in sequence, streaming output.
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
        // Step header separator
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

// ── App entry ───────────────────────────────────────────────────────────────

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
