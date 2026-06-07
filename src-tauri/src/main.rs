// Prevents console window on Windows in release (CLI won't work on Windows
// release builds, but this project only releases macOS/Linux).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod commands;

use clap::Parser;
use cli::{Cli, Commands};
use owo_colors::OwoColorize;
use std::io::{self, IsTerminal, Write};
use std::thread::sleep;
use std::time::Duration;

fn print_banner() {
    const FRAMES: &[(&str, (u8, u8, u8))] = &[
        (" ██████╗ ███╗   ███╗  █████╗  ██╗  ██╗", (0, 100, 160)),
        ("██╔═══██╗████╗ ████║ ██╔══██╗ ██║  ██║", (0, 130, 190)),
        ("██║   ██║██╔████╔██║ ███████║ ███████║", (0, 160, 215)),
        ("██║   ██║██║╚██╔╝██║ ██╔══██║ ██╔══██║", (0, 190, 235)),
        ("╚██████╔╝██║ ╚═╝ ██║ ██║  ██║ ██║  ██║", (0, 215, 248)),
        (" ╚═════╝ ╚═╝     ╚═╝ ╚═╝  ╚═╝ ╚═╝  ╚═╝", (0, 235, 255)),
    ];

    let mut stdout = io::stdout();
    let is_tty = stdout.is_terminal();

    if is_tty {
        for (line, _) in FRAMES {
            println!("{}", line.truecolor(30, 50, 65));
        }
        stdout.flush().ok();
        sleep(Duration::from_millis(25));

        print!("\x1b[{}A", FRAMES.len());
        stdout.flush().ok();
        for (line, (r, g, b)) in FRAMES {
            print!("\x1b[2K");
            println!("{}", line.truecolor(*r, *g, *b).bold());
            stdout.flush().ok();
            sleep(Duration::from_millis(22));
        }
    } else {
        for (line, (r, g, b)) in FRAMES {
            println!("{}", line.truecolor(*r, *g, *b).bold());
        }
    }

    println!(
        "  {}  {}",
        "omah".bold(),
        "— panggonan kanggo nyimpen backup".dimmed()
    );
    println!();
}

/// True only when an explicit subcommand or flag is provided.
/// No args always opens the desktop GUI (whether from terminal or app launcher).
fn is_cli_mode() -> bool {
    let args: Vec<String> = std::env::args().collect();

    // macOS Finder/Dock always injects -psn_*
    if args.iter().any(|a| a.starts_with("-psn")) {
        return false;
    }

    args.len() > 1
}

fn cli_main() -> anyhow::Result<()> {
    use clap::CommandFactory;

    if std::env::args_os().len() == 1 {
        print_banner();
        let _ = Cli::command().print_help();
        println!();
        return Ok(());
    }

    let cli = Cli::parse();

    if matches!(cli.command, Commands::Init) {
        print_banner();
    }

    let config_path = match cli.config {
        Some(p) => p,
        None => omah_lib::config::get_default_config_path()?,
    };

    match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Backup { no_exclude, name } => {
            commands::backup::run(&config_path, no_exclude, name.as_deref())
        }
        Commands::Restore { name } => commands::restore::run(&config_path, name.as_deref()),
        Commands::Status => commands::status::run(&config_path),
        Commands::List => commands::list::run(&config_path),
        Commands::Diff => commands::diff::run(&config_path),
        Commands::Add { name, source, symlink } => {
            commands::add::run(&config_path, name, source, symlink)
        }
        Commands::Remove { name } => commands::remove::run(&config_path, &name),
    }
}

/// Prepend known binary dirs that macOS .app bundles strip from PATH.
/// Safe to call before any threads spawn; harmless when dirs are already present.
fn augment_path() {
    #[cfg(target_os = "macos")]
    {
        const PREPEND: &[&str] = &[
            "/opt/homebrew/bin",   // Apple Silicon Homebrew
            "/opt/homebrew/sbin",
            "/usr/local/bin",      // Intel Homebrew + manually installed
            "/usr/local/sbin",
        ];
        let current = std::env::var("PATH").unwrap_or_default();
        let extra: Vec<&str> = PREPEND
            .iter()
            .copied()
            .filter(|p| !current.split(':').any(|seg| seg == *p))
            .collect();
        if !extra.is_empty() {
            let new_path = format!("{}:{}", extra.join(":"), current);
            // SAFETY: called before any threads are spawned.
            unsafe { std::env::set_var("PATH", new_path) };
        }
    }
}

fn main() {
    augment_path();
    if is_cli_mode() {
        if let Err(e) = cli_main() {
            eprintln!("Error: {e:#}");
            std::process::exit(1);
        }
    } else {
        desktop_lib::run();
    }
}
