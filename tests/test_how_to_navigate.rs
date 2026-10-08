use obsidian_mcp::config::AppConfig;
use obsidian_mcp::server::McpServer;
use std::sync::Arc;
use tokio::sync::RwLock;
use tempfile::tempdir;
use std::fs;

#[tokio::test]
async fn test_how_to_navigate_generation_and_protection() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().to_str().unwrap().to_string();

    let config = AppConfig {
        vaults: vec![],
    };

    let server = McpServer::new(Arc::new(RwLock::new(config)), std::path::PathBuf::from("dummy.yaml"));

    // 1. Call add_vault and ensure HOW_TO_NAVIGATE.md is generated
    let add_vault_req = serde_json::json!({
        "name": "add_vault",
        "arguments": {
            "vault_id": "test_vault",
            "path": vault_path.clone(),
            "read_only": false
        }
    });

    let resp = server.handle_call_tool_for_test(Some(add_vault_req), serde_json::json!(1)).await;
    assert!(resp.error.is_none(), "Failed to add vault");

    let nav_path = dir.path().join("HOW_TO_NAVIGATE.md");
    assert!(nav_path.exists(), "HOW_TO_NAVIGATE.md was not generated");
    let content = fs::read_to_string(&nav_path).unwrap();
    assert!(content.contains("How to Navigate This Vault"));
    assert!(!dir.path().join("_VAULT_RULES.md").exists(), "_VAULT_RULES.md should not be generated");

    // 2. Try to write/overwrite HOW_TO_NAVIGATE.md
    let write_req = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "HOW_TO_NAVIGATE.md",
            "content": "Malicious overwrite",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(write_req), serde_json::json!(2)).await;
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("strictly forbidden from modifying the core HOW_TO_NAVIGATE.md"));

    // 3. Try to append to HOW_TO_NAVIGATE.md
    let append_req = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "HOW_TO_NAVIGATE.md",
            "content": "Malicious append",
            "append": true
        }
    });
    let resp = server.handle_call_tool_for_test(Some(append_req), serde_json::json!(3)).await;
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("strictly forbidden"));

    // 4. Try to rename HOW_TO_NAVIGATE.md
    let rename_req = serde_json::json!({
        "name": "rename_note",
        "arguments": {
            "vault_id": "test_vault",
            "old_path": "HOW_TO_NAVIGATE.md",
            "new_path": "RENAMED.md"
        }
    });
    let resp = server.handle_call_tool_for_test(Some(rename_req), serde_json::json!(4)).await;
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("cannot rename the auto-generated HOW_TO_NAVIGATE.md"));

    // 5. Try to delete HOW_TO_NAVIGATE.md
    let delete_req = serde_json::json!({
        "name": "delete_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "HOW_TO_NAVIGATE.md"
        }
    });
    let resp = server.handle_call_tool_for_test(Some(delete_req), serde_json::json!(5)).await;
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("cannot delete the auto-generated HOW_TO_NAVIGATE.md file"));
}
