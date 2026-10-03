use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub vaults: Vec<VaultConfig>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct VaultConfig {
    pub id: String,
    pub path: String,
    pub read_only: bool,
}
