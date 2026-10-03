use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct VaultScanner;

impl VaultScanner {
    /// Scans a given vault directory and returns all Markdown files (.md).
    pub fn scan_markdown_files<P: AsRef<Path>>(vault_path: P) -> Result<Vec<PathBuf>> {
        let mut markdown_files = Vec::new();
        
        for entry in WalkDir::new(vault_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" {
                        markdown_files.push(path.to_path_buf());
                    }
                }
            }
        }
        
        Ok(markdown_files)
    }
}
