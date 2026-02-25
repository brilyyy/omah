use anyhow::Result;
use omah_lib::config::{get_default_config_path, init_setup};

pub fn run() -> Result<()> {
    init_setup()?;
    let config_path = get_default_config_path()?;
    println!("Initialized: {}", config_path.display());
    Ok(())
}
