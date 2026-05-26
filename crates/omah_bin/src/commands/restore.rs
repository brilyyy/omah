use std::collections::HashSet;
use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;
use omah_lib::{
    config::load_toml_config,
    deps::{install_command, missing_deps, pending_setup_steps, resolve_pkg_manager},
    ops::restore,
};

pub fn run(config_path: &Path, name: Option<&str>) -> Result<()> {
    let mut config = load_toml_config(config_path)?;

    // If a name is given, narrow config to just that dotfile
    if let Some(n) = name {
        config.dots.retain(|d| d.name == n);
        if config.dots.is_empty() {
            anyhow::bail!("Dotfile '{}' not found in config", n);
        }
        // Single-dotfile restore: skip the setup-steps prompt
        restore(&config)?;
        println!("Restore complete ← {}", config.vault_path);
        return Ok(());
    }

    // Collect all missing deps (deduped) and all pending setup steps
    let all_missing: Vec<String> = {
        let mut seen = HashSet::new();
        config
            .dots
            .iter()
            .flat_map(|dot| missing_deps(dot))
            .filter(|d| seen.insert(d.clone()))
            .collect()
    };

    // (dot_name, install_cmd) for each pending setup step
    let all_setup: Vec<(String, String)> = config
        .dots
        .iter()
        .flat_map(|dot| {
            pending_setup_steps(dot)
                .into_iter()
                .map(|s| (dot.name.clone(), s.install.clone()))
                .collect::<Vec<_>>()
        })
        .collect();

    // Build the ordered action list presented to the user
    let mut actions: Vec<(String, String)> = Vec::new();

    let pm = resolve_pkg_manager(config.pkg_manager.as_deref());

    if !all_missing.is_empty() {
        match pm {
            Some(ref pm) => {
                let cmd = install_command(pm, &all_missing);
                actions.push(("install deps".to_string(), cmd));
            }
            None => {
                eprintln!(
                    "Warning: missing deps [{}] but no known package manager found.",
                    all_missing.join(", ")
                );
            }
        }
    }

    for (dot_name, cmd) in &all_setup {
        actions.push((format!("setup  {dot_name}"), cmd.clone()));
    }

    if !actions.is_empty() {
        println!("The following steps are required before restore:\n");
        for (i, (label, cmd)) in actions.iter().enumerate() {
            println!("  [{}]  {}:  {}", i + 1, label, cmd);
        }

        print!("\nRun all? [y/N] ");
        io::stdout().flush()?;

        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;

        if answer.trim().eq_ignore_ascii_case("y") {
            let total = actions.len();
            for (i, (label, cmd)) in actions.iter().enumerate() {
                println!("\n[{}/{}] {}:", i + 1, total, label);
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd)
                    .status()?;
                if !status.success() {
                    eprintln!("Warning: step exited with {status}");
                }
            }
            println!();
        } else {
            // Ask before proceeding without the pre-restore steps
            print!("Continue restore without running setup steps? [y/N] ");
            io::stdout().flush()?;
            let mut confirm = String::new();
            io::stdin().read_line(&mut confirm)?;
            if !confirm.trim().eq_ignore_ascii_case("y") {
                println!("Aborted.");
                return Ok(());
            }
            println!();
        }
    }

    restore(&config)?;
    println!("Restore complete ← {}", config.vault_path);
    Ok(())
}
