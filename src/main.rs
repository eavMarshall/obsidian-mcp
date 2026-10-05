use obsidian_mcp::config::AppConfig;
use obsidian_mcp::server::McpServer;
use anyhow::Result;
use std::fs;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // The cross-compiler strips directories, so it drops this in the Antigravity CWD.
    // Renamed to prevent collisions with other project config files.
    let config_path = std::path::PathBuf::from("obsidian-mcp-config.yaml");
    
    let config_str = fs::read_to_string(&config_path).unwrap_or_else(|_| {
        String::from("vaults: []")
    });
    
    let config: AppConfig = serde_yaml::from_str(&config_str)?;
    let shared_config = Arc::new(std::sync::RwLock::new(config));

    let server = McpServer::new(shared_config);
    server.run().await?;
    
    Ok(())
}
