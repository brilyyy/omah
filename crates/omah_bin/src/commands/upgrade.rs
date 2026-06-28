use std::io::{self, Write};

use anyhow::{Context, Result};

fn current_target() -> Option<&'static str> {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("aarch64", "macos") => Some("aarch64-apple-darwin"),
        ("x86_64", "linux") => Some("x86_64-unknown-linux-musl"),
        ("aarch64", "linux") => Some("aarch64-unknown-linux-musl"),
        _ => None,
    }
}

pub fn run() -> Result<()> {
    let latest = match crate::upgrade_check::check_for_upgrade() {
        Some(tag) => tag,
        None => {
            println!(
                "omah {} is up to date",
                omah_lib::upgrade::CURRENT_VERSION
            );
            return Ok(());
        }
    };

    println!("New release {latest} available. Upgrading...");

    let target = current_target().context(
        "No prebuilt binary for your platform. Use 'cargo install omah' instead.",
    )?;

    let tmp_dir = std::env::temp_dir().join("omah-upgrade");
    std::fs::create_dir_all(&tmp_dir)
        .context("Failed to create temp directory")?;

    let tarball = tmp_dir.join("omah.tar.gz");
    let url = format!(
        "https://github.com/brilyyy/omah/releases/download/{latest}/omah-{latest}-{target}.tar.gz"
    );

    print!("  Downloading... ");
    io::stdout().flush()?;
    let status = std::process::Command::new("curl")
        .args([
            "-sSL",
            "--connect-timeout",
            "10",
            "--max-time",
            "60",
            "-o",
            tarball.to_str().unwrap(),
            &url,
        ])
        .status()
        .context("Failed to run curl. Is curl installed?")?;
    anyhow::ensure!(status.success(), "Download failed for {url}");
    println!("done");

    print!("  Extracting... ");
    io::stdout().flush()?;
    let status = std::process::Command::new("tar")
        .args([
            "xzf",
            tarball.to_str().unwrap(),
            "-C",
            tmp_dir.to_str().unwrap(),
        ])
        .status()
        .context("Failed to run tar")?;
    anyhow::ensure!(status.success(), "Extraction failed");
    println!("done");

    let extracted = tmp_dir.join("omah");
    anyhow::ensure!(
        extracted.is_file(),
        "Extracted binary not found in tarball"
    );

    let current_exe = std::env::current_exe()
        .context("Could not determine current binary path")?
        .canonicalize()
        .context("Could not resolve current binary path")?;

    print!("  Installing... ");
    io::stdout().flush()?;
    std::fs::copy(&extracted, &current_exe)
        .context("Failed to install. Try: sudo omah upgrade")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            &current_exe,
            std::fs::Permissions::from_mode(0o755),
        )?;
    }
    println!("done");

    let _ = std::fs::remove_dir_all(&tmp_dir);

    println!();
    println!(
        "  omah {latest} installed to {}",
        current_exe.display()
    );
    println!("  Run 'omah --version' to verify.");

    Ok(())
}
