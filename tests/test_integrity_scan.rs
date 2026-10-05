use obsidian_mcp::config::{AppConfig, VaultConfig};
use obsidian_mcp::server::McpServer;
use std::sync::{Arc, RwLock};
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

    let server = McpServer::new(Arc::new(RwLock::new(config)));
    
    // Create base files
    let _ = fs::write(dir.path().join("Index.md"), "---\nup: \"[[Index]]\"\n---\n# Root\n[[DanglingLink]]\n[[EmptyNote]]");
    let _ = fs::write(dir.path().join("ValidNote.md"), "---\nup: \"[[Index]]\"\n---\n# Valid");
    let _ = fs::write(dir.path().join("OrphanNote.md"), "---\nup: \"[[Index]]\"\n---\n# Orphan\nNo links to this.");
    let _ = fs::write(dir.path().join("EmptyNote.md"), "---\nup: \"[[Index]]\"\n---\n"); // Empty headers test case

    let integrity_req = serde_json::json!({
        "name": "check_integrity",
        "arguments": {
            "vault_id": "test_vault"
        }
    });
    
    let resp = server.handle_call_tool_for_test(Some(integrity_req), serde_json::json!(1));
    assert!(resp.error.is_none());
    
    let result = resp.result.unwrap();
    let report = result.get("report").unwrap().as_str().unwrap();
    
    // Case 2: Dangling forward link detection
    assert!(report.contains("DanglingLink"));
    
    // Case 1 & 3: Unreferenced / Orphaned files
    assert!(report.contains("OrphanNote.md"));
    
    // Case 4: Empty headers
    assert!(report.contains("EmptyNote.md"));
}
