use obsidian_mcp::config::{AppConfig, VaultConfig};
use obsidian_mcp::server::McpServer;
use std::sync::Arc;
use tokio::sync::RwLock;
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

    let server = McpServer::new(Arc::new(RwLock::new(config)), std::path::PathBuf::from("dummy.yaml"));

    // 1. Missing parent linking in body or implicit folder _index
    let missing_parent = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "test1.md",
            "content": "---\ntitle: \"My Note\"\n---\nBody text",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(missing_parent), serde_json::json!(1)).await;
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("Parent _index.md not found in the folder"));

    // 2. Forbidden `up:` key
    let has_up = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "test2.md",
            "content": "---\nup: \"[[_index]]\"\n---\nBody text",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(has_up), serde_json::json!(2)).await;
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("deprecated")); 

    // 3. Disallowed keys
    let disallowed_key = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "test3.md",
            "content": "---\ntitle: \"Valid\"\nbad_key: null\n---\nPart of [[_index]]\nBody text",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(disallowed_key), serde_json::json!(3)).await;
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("not allowed"));
    
    // 4. Malformed YAML
    let malformed_yaml = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "test4.md",
            "content": "---\ntitle: \"T\"\nunclosed bracket: [\n---\nBody",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(malformed_yaml), serde_json::json!(4)).await;
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("Invalid YAML"));
    
    // 5. Exempt folders tests/ and .obsidian/ shouldn't complain about missing parent
    let exempt_tests = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "tests/my_test_case.md",
            "content": "---\ntitle: \"Test\"\nsummary: \"\"\ntags: []\n---\nBody text with no part of",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(exempt_tests), serde_json::json!(5)).await;
    assert!(resp.error.is_none());
    
    let exempt_obsidian = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": ".obsidian/workspace",
            "content": "---\ntitle: \"Test\"\nsummary: \"\"\ntags: []\n---\nNo parent needed",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(exempt_obsidian), serde_json::json!(6)).await;
    assert!(resp.error.is_none());
}
