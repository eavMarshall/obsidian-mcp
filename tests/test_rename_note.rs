use obsidian_mcp::config::{AppConfig, VaultConfig};
use obsidian_mcp::server::McpServer;
use std::sync::Arc;
use tokio::sync::RwLock;
use tempfile::tempdir;

#[tokio::test]
async fn test_rename_note_auto_updates_links() {
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

    // 1. Create _index.md
    let write_index = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "_index.md",
            "content": "---\ntitle: \"Test\"\nsummary: \"\"\ntags: []\n---\nPart of [[_index]]Root",
            "append": false
        }
    });
    let resp1 = server.handle_call_tool_for_test(Some(write_index), serde_json::json!(1)).await;
    assert!(resp1.error.is_none(), "Failed to create _index.md: {:?}", resp1.error);

    // 2. Create Target.md
    let write_target = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "Target.md",
            "content": "---\ntitle: \"Test\"\nsummary: \"\"\ntags: []\n---\nPart of [[_index]]I am the target.",
            "append": false
        }
    });
    let resp2 = server.handle_call_tool_for_test(Some(write_target), serde_json::json!(2)).await;
    assert!(resp2.error.is_none(), "Failed to create Target.md: {:?}", resp2.error);

    // 3. Create Dependent.md linking to Target.md
    let write_dependent = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "Dependent.md",
            "content": "---\ntitle: \"Test\"\nsummary: \"\"\ntags: []\n---\nPart of [[Target]]I link to [[Target]] and also [[Target|Alias]] and [[Target#Header]].",
            "append": false
        }
    });
    let resp3 = server.handle_call_tool_for_test(Some(write_dependent), serde_json::json!(3)).await;
    assert!(resp3.error.is_none(), "Failed to create Dependent.md: {:?}", resp3.error);

    // 4. Try to rename Target.md to NewTarget.md
    let rename = serde_json::json!({
        "name": "rename_note",
        "arguments": {
            "vault_id": "test_vault",
            "old_path": "Target.md",
            "new_path": "NewTarget.md"
        }
    });
    let resp = server.handle_call_tool_for_test(Some(rename), serde_json::json!(4)).await;
    assert!(resp.error.is_none(), "Failed to rename note: {:?}", resp.error);

    // 5. Verify Target.md no longer exists but NewTarget.md does
    let old_file = dir.path().join("Target.md");
    let new_file = dir.path().join("NewTarget.md");
    assert!(!old_file.exists(), "Old file should have been deleted/renamed");
    assert!(new_file.exists(), "New file should exist");

    // 6. Verify Dependent.md was updated
    let dep_file = dir.path().join("Dependent.md");
    let dep_content = std::fs::read_to_string(&dep_file).unwrap();
    
    assert!(dep_content.contains("[[NewTarget]]"), "Standard link was not updated");
    assert!(dep_content.contains("[[NewTarget|Alias]]"), "Aliased link was not updated");
    assert!(dep_content.contains("[[NewTarget#Header]]"), "Anchor link was not updated");
    assert!(!dep_content.contains("[[Target]]"), "Old link should be completely gone");
}
