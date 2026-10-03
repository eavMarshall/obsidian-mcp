use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub vaults: Vec<VaultConfig>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct VaultConfig {
    pub id: String,
    pub path: String,
    pub read_only: bool,
}
