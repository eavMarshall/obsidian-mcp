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
    // 1. Valid file with Part of
    fs::write(dir.path().join("Valid.md"), "---\ntitle: \"Test\"\nsummary: \"\"\ntags: []\n---\nPart of [[_index]]").unwrap();
    
    // 2. Orphan file with no frontmatter and no Part of
    fs::write(dir.path().join("OrphanNoFrontmatter.md"), "# Just a file").unwrap();
    
    // 3. Orphan file with frontmatter but no Part of
    fs::write(dir.path().join("OrphanNoParent.md"), "---\ntitle: \"test\"\n---\n# Just a file").unwrap();
    
    // 4. Implicit valid file (due to _index.md in folder)
    let sub = dir.path().join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("_index.md"), "---\ntitle: \"Index\"\n---\n").unwrap();
    fs::write(sub.join("ImplicitValid.md"), "---\ntitle: \"Sub\"\n---\nBody").unwrap();

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
    assert_eq!(orphans_found, 2); 
    
    let orphans = result_map.get("orphans").unwrap().as_array().unwrap();
    let orphan_names: Vec<&str> = orphans.iter().map(|v| v.as_str().unwrap()).collect();
    
    assert!(orphan_names.iter().any(|n| n.contains("OrphanNoFrontmatter.md")));
    assert!(orphan_names.iter().any(|n| n.contains("OrphanNoParent.md")));
    assert!(!orphan_names.iter().any(|n| n.contains("Valid.md")));
    assert!(!orphan_names.iter().any(|n| n.contains("ImplicitValid.md")));
}
