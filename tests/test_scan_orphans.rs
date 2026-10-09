use obsidian_mcp::config::{AppConfig, VaultConfig};
use obsidian_mcp::server::McpServer;
use std::sync::Arc;
use tokio::sync::RwLock;
use tempfile::tempdir;
use std::fs;

#[tokio::test]
async fn test_scan_legacy_orphans() {
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
    
    // Create some files
    // 1. Valid file with up
    fs::write(dir.path().join("Valid.md"), "---\nup: \"[[Index]]\"\n---\n").unwrap();
    
    // 2. Orphan file with no frontmatter
    fs::write(dir.path().join("OrphanNoFrontmatter.md"), "# Just a file").unwrap();
    
    // 3. Orphan file with frontmatter but no up
    fs::write(dir.path().join("OrphanNoUp.md"), "---\ntags: [test]\n---\n# Just a file").unwrap();
    
    // 4. Orphan file with empty up
    fs::write(dir.path().join("OrphanEmptyUp.md"), "---\nup: \n---\n").unwrap();

    // 5. Orphan file with up: ""
    fs::write(dir.path().join("OrphanEmptyStringUp.md"), "---\nup: \"\"\n---\n").unwrap();
    
    let scan_req = serde_json::json!({
        "name": "scan_legacy_orphans",
        "arguments": {
            "vault_id": "test_vault"
        }
    });

    let resp = server.handle_call_tool_for_test(Some(scan_req), serde_json::json!(1)).await;
    
    assert!(resp.error.is_none());
    
    let result_obj = resp.result.unwrap();
    let text_val = result_obj.get("content").unwrap().as_array().unwrap()[0].get("text").unwrap().as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text_val).unwrap();
    let result_map = parsed.as_object().unwrap();
    
    let orphans_found = result_map.get("orphans_found").unwrap().as_i64().unwrap();
    assert_eq!(orphans_found, 4); // 4 orphans created above
    
    let orphans = result_map.get("orphans").unwrap().as_array().unwrap();
    let orphan_names: Vec<&str> = orphans.iter().map(|v| v.as_str().unwrap()).collect();
    
    assert!(orphan_names.iter().any(|n| n.contains("OrphanNoFrontmatter.md")));
    assert!(orphan_names.iter().any(|n| n.contains("OrphanNoUp.md")));
    assert!(orphan_names.iter().any(|n| n.contains("OrphanEmptyUp.md")));
    assert!(orphan_names.iter().any(|n| n.contains("OrphanEmptyStringUp.md")));
    assert!(!orphan_names.iter().any(|n| n.contains("Valid.md")));
}
