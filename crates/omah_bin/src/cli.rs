use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "omah", version, about = "Dotfile manager — panggonan kanggo nyimpen backup")]
pub struct Cli {
    /// Path to config file [default: ~/.config/omah/omah-config.toml]
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize config directory and scaffold default config
    Init,
    /// Back up all dotfiles to the vault
    Backup,
    /// Restore all dotfiles from the vault
    Restore,
    /// Show sync status of all dotfiles
    Status,
    /// List all configured dotfiles
    List,
    /// Launch the TUI dashboard
    #[cfg(feature = "tui")]
    Tui,
}
