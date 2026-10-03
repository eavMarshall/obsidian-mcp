use obsidian_mcp::config::{AppConfig, VaultConfig};
use obsidian_mcp::server::McpServer;
use std::sync::{Arc, RwLock};
use tempfile::tempdir;

#[tokio::test]
async fn test_read_and_write_note() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().to_str().unwrap().to_string();

    let config = AppConfig {
        vaults: vec![
            VaultConfig {
                id: "test_vault".to_string(),
                path: vault_path.clone(),
                read_only: false,
            },
            VaultConfig {
                id: "readonly_vault".to_string(),
                path: vault_path.clone(),
                read_only: true,
            },
        ],
    };

    let server = McpServer::new(Arc::new(RwLock::new(config)));

    // Test writing a note
    let write_params = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "test.md",
            "content": "# Test Header\nThis is a test note.",
            "append": false
        }
    });

    let write_resp = server.handle_call_tool_for_test(Some(write_params), serde_json::json!(1));
    assert!(write_resp.error.is_none());

    // Test reading the note back, expecting AST chunks
    let read_params = serde_json::json!({
        "name": "read_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "test.md"
        }
    });

    let read_resp = server.handle_call_tool_for_test(Some(read_params), serde_json::json!(2));
    assert!(read_resp.error.is_none());
    
    let result = read_resp.result.unwrap();
    let text = result["content"][0]["text"].as_str().unwrap();
    
    // Validate AST chunking worked
    assert!(text.contains(r#""header":"Test Header""#));
    assert!(text.contains(r#""content":"This is a test note.""#));

    // Test Read-Only enforcement
    let write_readonly_params = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "readonly_vault",
            "path": "test2.md",
            "content": "Should fail",
            "append": false
        }
    });

    let readonly_resp = server.handle_call_tool_for_test(Some(write_readonly_params), serde_json::json!(3));
    assert!(readonly_resp.error.is_some());
    assert!(readonly_resp.error.unwrap().message.contains("read-only"));

    // Test Search Vault
    let search_params = serde_json::json!({
        "name": "search_vault",
        "arguments": {
            "vault_id": "test_vault",
            "query": "test note"
        }
    });

    let search_resp = server.handle_call_tool_for_test(Some(search_params), serde_json::json!(4));
    assert!(search_resp.error.is_none());
    
    let search_result = search_resp.result.unwrap();
    let results_array = search_result["content"][0]["text"].as_str().unwrap();
    
    // We expect the result text to contain the chunk we wrote earlier
    assert!(results_array.contains("test.md"));
    assert!(results_array.contains("Test Header"));

    // Write a second note that links to the first note
    let write_link_params = serde_json::json!({
        "name": "write_note",
        "arguments": {
            "vault_id": "test_vault",
            "path": "note_b.md",
            "content": "This links to [[test]] and [[test|Alias]]",
            "append": false
        }
    });
    let _ = server.handle_call_tool_for_test(Some(write_link_params), serde_json::json!(5));

    // Test get_links
    let link_params = serde_json::json!({
        "name": "get_links",
        "arguments": {
            "vault_id": "test_vault",
            "file_name": "test"
        }
    });

    let link_resp = server.handle_call_tool_for_test(Some(link_params), serde_json::json!(6));
    assert!(link_resp.error.is_none());
    
    let link_result = link_resp.result.unwrap();
    let text = link_result["content"][0]["text"].as_str().unwrap();
    
    // We expect note_b.md to be listed in the backlinks
    // We expect note_b.md to be listed in the backlinks. Wait, get_links with 'all' returns an array of objects.
    // The previous test expects `["note_b.md"]` which was the old format, the new format is `[{"vault_id": "test_vault", "file": "note_b.md"}]`
    assert!(text.contains(r#""file":"note_b.md""#));

    // Test Global Search with "all"
    let global_search_params = serde_json::json!({
        "name": "search_vault",
        "arguments": {
            "vault_id": "all",
            "query": "test note"
        }
    });

    let global_search_resp = server.handle_call_tool_for_test(Some(global_search_params), serde_json::json!(7));
    assert!(global_search_resp.error.is_none());
    let global_search_result = global_search_resp.result.unwrap();
    let global_results_array = global_search_result["content"][0]["text"].as_str().unwrap();
    assert!(global_results_array.contains("test.md"));
    assert!(global_results_array.contains("test_vault"));

    // Test add_vault physical directory creation
    let new_vault_path = dir.path().join("new_physical_vault");
    let add_vault_params = serde_json::json!({
        "name": "add_vault",
        "arguments": {
            "vault_id": "new_vault",
            "path": new_vault_path.to_str().unwrap(),
            "read_only": false
        }
    });

    let add_resp = server.handle_call_tool_for_test(Some(add_vault_params), serde_json::json!(8));
    assert!(add_resp.error.is_none());
    assert!(new_vault_path.exists());
    assert!(new_vault_path.is_dir());
}
