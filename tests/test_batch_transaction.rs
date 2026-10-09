use obsidian_mcp::config::{AppConfig, VaultConfig};
use obsidian_mcp::server::McpServer;
use std::sync::Arc;
use tokio::sync::RwLock;
use tempfile::tempdir;
use std::fs;

#[tokio::test]
async fn test_batch_transaction() {
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
    
    // Create an initial Index file manually
    fs::write(dir.path().join("Index.md"), "---\nup: \"[[Index]]\"\n---\n").unwrap();

    // Call batch_transaction with operations that would normally fail FK checks if done individually
    // op1: Create A.md with up: B.md
    // op2: Create B.md with up: Index.md
    
    let batch_req = serde_json::json!({
        "name": "batch_transaction",
        "arguments": {
            "vault_id": "test_vault",
            "operations": [
                {
                    "type": "write_note",
                    "path": "A.md",
                    "content": "---\nup: \"[[B]]\"\n---\n",
                    "append": false
                },
                {
                    "type": "write_note",
                    "path": "B.md",
                    "content": "---\nup: \"[[Index]]\"\n---\n",
                    "append": false
                }
            ]
        }
    });

    let resp = server.handle_call_tool_for_test(Some(batch_req), serde_json::json!(1)).await;
    
    assert!(resp.error.is_none());
    
    let result_obj = resp.result.unwrap();
    let text_val = result_obj.get("content").unwrap().as_array().unwrap()[0].get("text").unwrap().as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text_val).unwrap();
    let result_map = parsed.as_object().unwrap();
    
    let operations_completed = result_map.get("operations_completed").unwrap().as_i64().unwrap();
    assert_eq!(operations_completed, 2);
    
    // Check final integrity warning is null since all links resolve
    let final_integrity_warning = result_map.get("final_integrity_warning").unwrap();
    assert!(final_integrity_warning.is_null());
    
    // Verify files exist
    assert!(dir.path().join("A.md").exists());
    assert!(dir.path().join("B.md").exists());
}

#[tokio::test]
async fn test_batch_transaction_with_dead_links() {
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
    
    // Create an initial Index file manually
    fs::write(dir.path().join("Index.md"), "---\nup: \"[[Index]]\"\n---\n").unwrap();

    let batch_req = serde_json::json!({
        "name": "batch_transaction",
        "arguments": {
            "vault_id": "test_vault",
            "operations": [
                {
                    "type": "write_note",
                    "path": "A.md",
                    "content": "---\nup: \"[[NonExistentFile]]\"\n---\n",
                    "append": false
                }
            ]
        }
    });

    let resp = server.handle_call_tool_for_test(Some(batch_req), serde_json::json!(1)).await;
    
    assert!(resp.error.is_none());
    
    let result_obj = resp.result.unwrap();
    let text_val = result_obj.get("content").unwrap().as_array().unwrap()[0].get("text").unwrap().as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text_val).unwrap();
    let result_map = parsed.as_object().unwrap();
    
    // Should warn about dead links
    let final_integrity_warning = result_map.get("final_integrity_warning").unwrap();
    assert!(final_integrity_warning.is_string());
    assert!(final_integrity_warning.as_str().unwrap().contains("dead links"));
}
