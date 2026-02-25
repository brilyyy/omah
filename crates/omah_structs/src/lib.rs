use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct OmahConfig {
    pub vault_path: String,
    pub dots: Vec<DotfileConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DotfileConfig {
    pub name: String,
    pub source: String,
    pub symlink: Option<bool>,
}
