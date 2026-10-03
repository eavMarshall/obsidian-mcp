use obsidian_mcp::config::AppConfig;
use obsidian_mcp::server::McpServer;
use anyhow::Result;
use std::fs;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("Starting Headless Markdown MCP Gateway...");
    
    // Default to config.yaml in the current directory
    let config_str = fs::read_to_string("config.yaml").unwrap_or_else(|_| {
        eprintln!("Warning: config.yaml not found, using empty config");
        String::from("server:\n  port: 8080\nvaults: []")
    });
    
    let config: AppConfig = serde_yaml::from_str(&config_str)?;
    let shared_config = Arc::new(std::sync::RwLock::new(config));

    let server = McpServer::new(shared_config);
    server.run().await?;
    
    Ok(())
}
