use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub vaults: Vec<VaultConfig>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct VaultConfig {
    pub id: String,
    pub path: String,
    pub read_only: bool,
}
