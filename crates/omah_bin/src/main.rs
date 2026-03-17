mod cli;
mod commands;

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

fn main() -> anyhow::Result<()> {
    use clap::CommandFactory;

    // No subcommand: launch TUI when available, otherwise show banner + help.
    if std::env::args_os().len() == 1 {
        #[cfg(feature = "tui")]
        {
            let config_path = omah_lib::config::get_default_config_path()?;
            return omah_tui::run(&config_path);
        }
        #[cfg(not(feature = "tui"))]
        {
            print_banner();
            let _ = Cli::command().print_help();
            println!();
            return Ok(());
        }
    }

    let cli = Cli::parse();

    // Banner only for init.
    if matches!(cli.command, Commands::Init) {
        print_banner();
    }

    let config_path = match cli.config {
        Some(p) => p,
        None => omah_lib::config::get_default_config_path()?,
    };

    match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Backup { no_git, no_exclude } => {
            commands::backup::run(&config_path, no_git, no_exclude)
        }
        Commands::Restore => commands::restore::run(&config_path),
        Commands::Status => commands::status::run(&config_path),
        Commands::List => commands::list::run(&config_path),
        Commands::Diff => commands::diff::run(&config_path),
        #[cfg(feature = "tui")]
        Commands::Tui => omah_tui::run(&config_path),
    }
}
