use obsidian_mcp::config::{AppConfig, VaultConfig};
use obsidian_mcp::server::McpServer;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_missing_vault_error() {
    let config = AppConfig {
        vaults: vec![],
    };
    let server = McpServer::new(Arc::new(RwLock::new(config)), std::path::PathBuf::from("dummy.yaml"));

    let read_params = serde_json::json!({
        "name": "read_note",
        "arguments": {
            "vault_id": "ghost_vault",
            "path": "test.md"
        }
    });

    let resp = server.handle_call_tool_for_test(Some(read_params), serde_json::json!(1)).await;
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("not found"));
}

#[tokio::test]
async fn test_invalid_tool_error() {
    let config = AppConfig { vaults: vec![] };
    let server = McpServer::new(Arc::new(RwLock::new(config)), std::path::PathBuf::from("dummy.yaml"));

    let params = serde_json::json!({
        "name": "hack_mainframe",
        "arguments": {}
    });

    let resp = server.handle_call_tool_for_test(Some(params), serde_json::json!(2)).await;
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("Unknown tool"));
}
