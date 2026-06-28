use std::path::Path;

use anyhow::Result;

pub fn run(config_path: &Path) -> Result<()> {
    omah_tui::run(config_path)
}
