/// Standalone binary entry point — `omah-tui [CONFIG_PATH]`
/// For normal use, run `omah tui` instead (recommended).
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let config_path = if args.len() > 1 {
        std::path::PathBuf::from(&args[1])
    } else {
        omah_lib::config::get_default_config_path()?
    };
    omah_tui::run(&config_path)
}
