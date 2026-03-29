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
    Backup {
        /// Ignore exclude patterns from config
        #[arg(long)]
        no_exclude: bool,
    },
    /// Restore all dotfiles from the vault
    Restore,
    /// Show sync status of all dotfiles
    Status,
    /// List all configured dotfiles
    List,
    /// Show what has changed between source and vault
    Diff,
}
