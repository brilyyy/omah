use std::sync::mpsc;
use std::thread;

use anyhow::Result;
use omah_lib::OmahConfig;

/// Messages from background operations to the UI event loop.
pub enum OpsMessage {
    Log(String),
    Progress(u64, u64),
    Done(Result<()>),
}

/// Handle for polling a background operation.
pub struct OpsHandle {
    pub receiver: mpsc::Receiver<OpsMessage>,
}

pub fn start_backup(config: OmahConfig, dry_run: bool) -> OpsHandle {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(OpsMessage::Log("Backup started.".into()));
        match omah_lib::ops::backup(&config, dry_run) {
            Ok(()) => {
                let _ = tx.send(OpsMessage::Log("✓ Backup complete.".into()));
                let _ = tx.send(OpsMessage::Done(Ok(())));
            }
            Err(e) => {
                let _ = tx.send(OpsMessage::Log(format!("✗ Backup failed: {e}")));
                let _ = tx.send(OpsMessage::Done(Err(e)));
            }
        }
    });
    OpsHandle { receiver: rx }
}

pub fn start_restore(config: OmahConfig, dry_run: bool) -> OpsHandle {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(OpsMessage::Log("Restore started.".into()));
        match omah_lib::ops::restore(&config, dry_run) {
            Ok(()) => {
                let _ = tx.send(OpsMessage::Log("✓ Restore complete.".into()));
                let _ = tx.send(OpsMessage::Done(Ok(())));
            }
            Err(e) => {
                let _ = tx.send(OpsMessage::Log(format!("✗ Restore failed: {e}")));
                let _ = tx.send(OpsMessage::Done(Err(e)));
            }
        }
    });
    OpsHandle { receiver: rx }
}

/// Execute a shell command and report progress messages.
pub fn run_install_cmd(cmd: &str) -> Result<()> {
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .status()?;
    if !status.success() {
        anyhow::bail!("Command exited with status {status}");
    }
    Ok(())
}
