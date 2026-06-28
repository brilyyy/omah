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
    /// Back up all dotfiles (or a single named dotfile) to the vault
    Backup {
        /// Ignore exclude patterns from config
        #[arg(long)]
        no_exclude: bool,
        /// Only back up this dotfile
        #[arg(value_name = "NAME")]
        name: Option<String>,
        /// Show what would be backed up without copying
        #[arg(long)]
        dry_run: bool,
    },
    /// Restore all dotfiles (or a single named dotfile) from the vault
    Restore {
        /// Only restore this dotfile
        #[arg(value_name = "NAME")]
        name: Option<String>,
        /// Show what would be restored without copying
        #[arg(long)]
        dry_run: bool,
    },
    /// Show sync status of all dotfiles
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List all configured dotfiles
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show what has changed between source and vault
    Diff {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add a dotfile entry to the config
    Add {
        /// Display name (used as vault folder)
        name: String,
        /// Path to the source file or directory
        source: String,
        /// Replace source with a symlink after backup
        #[arg(long)]
        symlink: bool,
    },
    /// Remove a dotfile entry from the config (does not delete files)
    Remove {
        /// Name of the dotfile to remove
        name: String,
    },
    /// Show detailed info about a dotfile
    Info {
        /// Dotfile name (omit to show all)
        name: Option<String>,
    },
    /// Migrate legacy vault to new ID-based structure
    Migrate,
    /// Launch the terminal user interface (interactive mode)
    Tui,
}
