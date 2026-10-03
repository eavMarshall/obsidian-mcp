use obsidian_mcp::vault::VaultScanner;
use std::fs::File;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_scan_markdown_files_only() {
    // Create a temporary directory mimicking a vault
    let dir = tempdir().unwrap();
    let file_path1 = dir.path().join("rule_api.md");
    let file_path2 = dir.path().join("ignore_me.txt");
    let file_path3 = dir.path().join("system_architecture.md");

    File::create(&file_path1).unwrap();
    File::create(&file_path2).unwrap();
    File::create(&file_path3).unwrap();

    let md_files = VaultScanner::scan_markdown_files(dir.path()).expect("Failed to scan directory");
    
    assert_eq!(md_files.len(), 2, "Should only pick up .md files");
    
    let paths: Vec<PathBuf> = md_files.into_iter().collect();
    assert!(paths.contains(&file_path1));
    assert!(paths.contains(&file_path3));
    assert!(!paths.contains(&file_path2), "Should completely ignore .txt files");
}
