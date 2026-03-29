pub use omah_structs::{DotfileConfig, OmahConfig, SetupStep};

pub use omah_lib::config::{
    check_dir_exists, check_file_exists, get_default_config_path, get_default_dir, init_setup,
    load_toml_config, save_toml_config,
};
pub use omah_lib::constants::{DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILE, DEFAULT_VAULT_PATH};
pub use omah_lib::deps::{declared_deps, detect_package_manager, install_command, is_installed, missing_deps, pending_setup_steps, resolve_pkg_manager};
pub use omah_lib::ops::{backup, diff, restore, status, ChangeKind, DotStatus, FileChange};
