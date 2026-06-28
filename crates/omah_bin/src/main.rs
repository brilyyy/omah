mod cli;
mod commands;

use std::path::PathBuf;

use clap::Parser;
use cli::{Cli, Commands};
use owo_colors::OwoColorize;
use std::io::{self, IsTerminal, Write};
use std::thread::sleep;
use std::time::Duration;

fn print_banner() {
    // Full Unicode block art with a top-to-bottom colour sweep on TTY.
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
        // Phase 1 — paint all lines dim so the shape is immediately visible.
        for (line, _) in FRAMES {
            println!("{}", line.truecolor(30, 50, 65));
        }
        stdout.flush().ok();
        sleep(Duration::from_millis(25));

        // Phase 2 — sweep top-to-bottom, lighting each line to full colour.
        print!("\x1b[{}A", FRAMES.len()); // move cursor back up
        stdout.flush().ok();
        for (line, (r, g, b)) in FRAMES {
            print!("\x1b[2K"); // clear current line in-place
            println!("{}", line.truecolor(*r, *g, *b).bold());
            stdout.flush().ok();
            sleep(Duration::from_millis(22));
        }
    } else {
        // Non-TTY: static coloured art, no delays.
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

fn omah_config_from_env() -> Option<PathBuf> {
    // .env file in CWD (dev convenience)
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(value) = line.strip_prefix("OMAH_CONFIG=") {
                let value = value.trim().trim_matches('"').trim_matches('\'');
                if !value.is_empty() {
                    return Some(PathBuf::from(value));
                }
            }
        }
    }
    // OMAH_CONFIG env var
    if let Ok(val) = std::env::var("OMAH_CONFIG") {
        let val = val.trim().to_string();
        if !val.is_empty() {
            return Some(PathBuf::from(val));
        }
    }
    None
}

fn resolve_config_path(cli_config: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(p) = cli_config {
        return Ok(p);
    }
    if let Some(p) = omah_config_from_env() {
        return Ok(p);
    }
    omah_lib::config::get_default_config_path()
}

fn main() -> anyhow::Result<()> {
    use clap::CommandFactory;

    // No subcommand: show banner + help.
    if std::env::args_os().len() == 1 {
        print_banner();
        let _ = Cli::command().print_help();
        println!();
        return Ok(());
    }

    let cli = Cli::parse();

    // Banner only for init.
    if matches!(cli.command, Commands::Init) {
        print_banner();
    }

    let config_path = resolve_config_path(cli.config)?;

    match cli.command {
        Commands::Init => commands::init::run(&config_path),
        Commands::Backup { no_exclude, name, dry_run } => {
            commands::backup::run(&config_path, no_exclude, dry_run, name.as_deref())
        }
        Commands::Restore { name, dry_run } => {
            commands::restore::run(&config_path, dry_run, name.as_deref())
        }
        Commands::Status { json } => commands::status::run(&config_path, json),
        Commands::List { json } => commands::list::run(&config_path, json),
        Commands::Diff { json } => commands::diff::run(&config_path, json),
        Commands::Add { name, source, symlink } => {
            commands::add::run(&config_path, name, source, symlink)
        }
        Commands::Remove { name } => commands::remove::run(&config_path, &name),
        Commands::Info { name } => commands::info::run(&config_path, name.as_deref()),
    }
}
