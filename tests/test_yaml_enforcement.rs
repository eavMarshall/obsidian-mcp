use obsidian_mcp::config::{AppConfig, VaultConfig};
use obsidian_mcp::server::McpServer;
use std::sync::{Arc, RwLock};
use tempfile::tempdir;

#[tokio::test]
async fn test_yaml_enforcement() {
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

    let server = McpServer::new(Arc::new(RwLock::new(config)));

    // 1. Missing `up:` key but valid YAML
    let missing_up = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "test1.md",
            "content": "---\ntitle: \"My Note\"\n---\nBody text",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(missing_up), serde_json::json!(1));
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("must include at minimum an `up:` field"));

    // 2. Empty `up:` key
    let empty_up = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "test2.md",
            "content": "---\nup: \"\"\n---\nBody text",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(empty_up), serde_json::json!(2));
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("empty")); // Wait, the bouncer message might say "missing `up:`" or similar, need to ensure the implementation handles empty correctly. The current implementation in server.rs line 465 just checks if frontmatter.contains_key("up"), we might need to update the server.rs to enforce it's not empty!

    // 3. Null `up:` key
    let null_up = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "test3.md",
            "content": "---\nup: null\n---\nBody text",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(null_up), serde_json::json!(3));
    assert!(resp.error.is_some());
    
    // 4. Malformed YAML
    let malformed_yaml = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "test4.md",
            "content": "---\nup: [[Index]]\nunclosed bracket: [\n---\nBody",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(malformed_yaml), serde_json::json!(4));
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("Invalid YAML"));
}
