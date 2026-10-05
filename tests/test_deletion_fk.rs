use obsidian_mcp::config::{AppConfig, VaultConfig};
use obsidian_mcp::server::McpServer;
use std::sync::{Arc, RwLock};
use tempfile::tempdir;
use std::fs;

#[tokio::test]
async fn test_deletion_fk_constraints() {
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
    
    // Create base files
    let _ = fs::write(dir.path().join("Index.md"), "---\nup: \"[[Index]]\"\n---\n# Root");
    let _ = fs::write(dir.path().join("TargetNote.md"), "---\nup: \"[[Index]]\"\n---\n# Target");
    let _ = fs::write(dir.path().join("SourceNote.md"), "---\nup: \"[[Index]]\"\n---\n# Source\n[[TargetNote]]");
    let _ = fs::write(dir.path().join("_VAULT_RULES.md"), "Rules file");

    // Case 1: Delete note with active backlinks (TargetNote is linked by SourceNote)
    let delete_linked_req = serde_json::json!({
        "name": "delete_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "TargetNote.md"
        }
    });
    let resp = server.handle_call_tool_for_test(Some(delete_linked_req), serde_json::json!(1));
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("Foreign Key Constraint failed"));
    
    // Case 2: Delete rules file (_VAULT_RULES.md)
    let delete_rules_req = serde_json::json!({
        "name": "delete_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "_VAULT_RULES.md"
        }
    });
    let resp = server.handle_call_tool_for_test(Some(delete_rules_req), serde_json::json!(2));
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("cannot delete the auto-generated _VAULT_RULES.md"));
    
    // Case 3: Verify we can delete a note with no backlinks
    let delete_unlinked_req = serde_json::json!({
        "name": "delete_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "SourceNote.md"
        }
    });
    let resp = server.handle_call_tool_for_test(Some(delete_unlinked_req), serde_json::json!(3));
    assert!(resp.error.is_none());
}
