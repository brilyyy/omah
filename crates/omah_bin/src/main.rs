mod cli;
mod commands;

use clap::Parser;
use cli::{Cli, Commands};
use owo_colors::OwoColorize;

fn print_banner() {
    let rows = [
        "   ███   █   █   ███   █   █  ",
        "  █   █  ██ ██  █   █  █   █  ",
        "  █   █  █ █ █  █████  █████  ",
        "  █   █  █   █  █   █  █   █  ",
        "   ███   █   █  █   █  █   █  ",
    ];
    for row in &rows {
        println!("{}", row.cyan().bold());
    }
    println!(
        "  {}  {}",
        "omah".bold(),
        "— panggonan kanggo nyimpen backup".dimmed()
    );
    println!();
}

fn main() -> anyhow::Result<()> {
    print_banner();

    let cli = Cli::parse();

    let config_path = match cli.config {
        Some(p) => p,
        None => omah_lib::config::get_default_config_path()?,
    };

    match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Backup => commands::backup::run(&config_path),
        Commands::Restore => commands::restore::run(&config_path),
        Commands::Status => commands::status::run(&config_path),
        Commands::List => commands::list::run(&config_path),
        #[cfg(feature = "tui")]
        Commands::Tui => omah_tui::run(&config_path),
    }
}
