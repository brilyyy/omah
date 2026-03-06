use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct OmahConfig {
    pub vault_path: String,
    pub dots: Vec<DotfileConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetupStep {
    /// Path to check for existence; if it exists, skip this step.
    pub check: Option<String>,
    /// Shell command to run.
    pub install: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DotfileConfig {
    pub name: String,
    pub source: String,
    pub symlink: Option<bool>,
    pub deps: Option<Vec<String>>,
    pub setup: Option<Vec<SetupStep>>,
}
