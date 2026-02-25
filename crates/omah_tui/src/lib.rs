use std::path::Path;

use anyhow::Result;

pub fn run(_config_path: &Path) -> Result<()> {
    // TODO: implement TUI dashboard
    // Planned screens:
    //   - Dotfile list with sync status indicators
    //   - Vault path display
    //   - Keybindings: (b)ackup, (r)estore, (q)uit
    todo!("TUI not yet implemented")
}
