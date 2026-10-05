use obsidian_mcp::config::{AppConfig, VaultConfig};
use obsidian_mcp::server::McpServer;
use std::sync::{Arc, RwLock};
use tempfile::tempdir;

#[tokio::test]
async fn test_foreign_key_constraints() {
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

    // 1. Create the root Index
    let write_index = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "Index.md",
            "content": "---\nup: \"[[Index]]\"\n---\nRoot",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(write_index), serde_json::json!(1));
    assert!(resp.error.is_none(), "Failed to create Index.md");

    // 2. Try to write a note linking to a missing file (Should fail FK check)
    let write_bad_link = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "note_b.md",
            "content": "---\nup: \"[[Index]]\"\n---\nThis links to [[Missing Note]]",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(write_bad_link), serde_json::json!(2));
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("Foreign Key Constraint failed"));

    // 3. Create the missing note
    let write_missing = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "Missing Note.md",
            "content": "---\nup: \"[[Index]]\"\n---\nI exist now",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(write_missing), serde_json::json!(3));
    assert!(resp.error.is_none(), "Failed to create Missing Note.md");

    // 4. Try again to write note_b.md (Should succeed now)
    let write_good_link = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "note_b.md",
            "content": "---\nup: \"[[Index]]\"\n---\nThis links to [[Missing Note]]",
            "append": false
        }
    });
    let resp = server.handle_call_tool_for_test(Some(write_good_link), serde_json::json!(4));
    assert!(resp.error.is_none(), "Failed to write note_b.md after creating dependency");

    // 5. Try to delete Missing Note.md (Should fail because note_b links to it)
    let delete_dep = serde_json::json!({
        "name": "delete_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "Missing Note.md"
        }
    });
    let resp = server.handle_call_tool_for_test(Some(delete_dep), serde_json::json!(5));
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().message.contains("Foreign Key Constraint failed"));

    // 6. Delete note_b.md (Success)
    let delete_child = serde_json::json!({
        "name": "delete_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "note_b.md"
        }
    });
    let resp = server.handle_call_tool_for_test(Some(delete_child), serde_json::json!(6));
    assert!(resp.error.is_none(), "Failed to delete note_b.md");

    // 7. Delete Missing Note.md (Should succeed now that dependent is gone)
    let delete_dep_retry = serde_json::json!({
        "name": "delete_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "Missing Note.md"
        }
    });
    let resp = server.handle_call_tool_for_test(Some(delete_dep_retry), serde_json::json!(7));
    assert!(resp.error.is_none(), "Failed to delete Missing Note.md after removing dependent");
}
