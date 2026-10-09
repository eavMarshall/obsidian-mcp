use obsidian_mcp::config::{AppConfig, VaultConfig};
use obsidian_mcp::server::McpServer;
use std::sync::Arc;
use tokio::sync::RwLock;
use tempfile::tempdir;
use std::fs;

#[tokio::test]
async fn test_integrity_scan_cases() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().to_str().unwrap().to_string();

    let config = AppConfig {
        vaults: vec![
            VaultConfig {
                id: "test_vault".to_string(),
                path: vault_path.clone(),
                read_only: false,
            }
        ],
    };

    let server = McpServer::new(Arc::new(RwLock::new(config)), std::path::PathBuf::from("dummy.yaml"));
    
    // Create base files
    let _ = fs::write(dir.path().join("_index.md"), "---\ntitle: \"Test\"\nsummary: \"\"\ntags: []\n---\nPart of [[_index]]# Root\n[[DanglingLink]]\n[[EmptyNote]]");
    let _ = fs::write(dir.path().join("ValidNote.md"), "---\ntitle: \"Test\"\nsummary: \"\"\ntags: []\n---\nPart of [[_index]]# Valid");
    let _ = fs::write(dir.path().join("OrphanNote.md"), "---\ntitle: \"Test\"\nsummary: \"\"\ntags: []\n---\nPart of [[_index]]# Orphan\nNo links to this.");
    let _ = fs::write(dir.path().join("EmptyNote.md"), "---\ntitle: \"Test\"\nsummary: \"\"\ntags: []\n---\nPart of [[_index]]"); // Empty headers test case

    let integrity_req = serde_json::json!({
        "name": "check_integrity",
        "arguments": {
            "vault_id": "test_vault"
        }
    });
    
    let resp = server.handle_call_tool_for_test(Some(integrity_req), serde_json::json!(1)).await;
    assert!(resp.error.is_none());
    
    let result = resp.result.unwrap();
    let content_arr = result.get("content").unwrap().as_array().unwrap();
    let text = content_arr[0].get("text").unwrap().as_str().unwrap();
    
    // Case 1: Dangling forward link detection
    assert!(text.contains("DanglingLink"));
}
