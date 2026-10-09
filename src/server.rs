use crate::config::{AppConfig, VaultConfig};
use crate::mcp::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::parser::MarkdownParser;
use anyhow::Result;
use serde_json::json;
use tokio::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::io::{self, AsyncBufReadExt, BufReader};

pub struct McpServer {
    config: Arc<RwLock<AppConfig>>,
    config_path: PathBuf,
}

impl McpServer {
    pub fn new(config: Arc<RwLock<AppConfig>>, config_path: PathBuf) -> Self {
        Self { config, config_path }
    }

    pub async fn run(&self) -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut reader = BufReader::new(stdin).lines();
        
        // Open log file for debugging
        let mut log_file = fs::OpenOptions::new().create(true).append(true).open("mcp_log.txt").await?;

        while let Some(line) = reader.next_line().await? {
            if line.trim().is_empty() { continue; }
            
            // Log incoming
            use tokio::io::AsyncWriteExt;
            log_file.write_all(format!("INCOMING: {}", line).as_bytes()).await?;
            
            match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(req) => {
                    if let Some(resp) = self.handle_request(req).await {
                        let resp_str = serde_json::to_string(&resp)? + "\n";
                        log_file.write_all(format!("OUTGOING: {}\n", resp_str.trim()).as_bytes()).await?;
                        stdout.write_all(resp_str.as_bytes()).await?;
                        stdout.flush().await?;
                    }
                }
                Err(e) => {
                    log_file.write_all(format!("PARSE ERROR: {} - Payload: {}", e, line).as_bytes()).await?;
                    let err_resp = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: serde_json::Value::Null,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32700,
                            message: "Parse error".to_string(),
                        }),
                    };
                    let resp_str = serde_json::to_string(&err_resp)? + "\n";
                    stdout.write_all(resp_str.as_bytes()).await?;
                    stdout.flush().await?;
                }
            }
        }

        Ok(())
    }

    pub async fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        // If the request doesn't have an ID, it's a JSON-RPC notification.
        // The JSON-RPC 2.0 spec mandates that servers MUST NOT respond to notifications, even if they fail.
        let is_notification = req.id.is_none() || req.id.as_ref().unwrap().is_null();
        let id = req.id.clone().unwrap_or(serde_json::Value::Null);
        
        match req.method.as_str() {
            "initialize" => Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {},
                        "prompts": {}
                    },
                    "serverInfo": {
                        "name": "obsidian-mcp",
                        "version": "0.1.0"
                    }
                })),
                error: None,
            }),
            "notifications/initialized" => None,
            "tools/list" => Some(self.handle_list_tools(id)),
            "tools/call" => Some(self.handle_call_tool(req.params, id).await),
            "prompts/list" => Some(self.handle_list_prompts(id)),
            "prompts/get" => Some(self.handle_get_prompt(req.params, id)),
            _ => {
                if is_notification {
                    None // Never reply to unknown notifications (e.g. notifications/roots/list_changed)
                } else {
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32601,
                            message: "Method not found".to_string(),
                        }),
                    })
                }
            }
        }
    }

    fn handle_list_tools(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "tools": [
                    {
                        "name": "list_vaults",
                        "description": "Lists all configured vaults",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "add_vault",
                        "description": "Dynamically map a new vault and save it to config.yaml",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "vault_id": { "type": "string" },
                                "path": { "type": "string" },
                                "read_only": { "type": "boolean" }
                            },
                            "required": ["vault_id", "path", "read_only"]
                        }
                    },
                    {
                        "name": "read_note",
                        "description": "Reads a markdown note and applies AST chunking to save tokens",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "vault_id": { "type": "string" },
                                "path": { "type": "string" }
                            },
                            "required": ["vault_id", "path"]
                        }
                    },
                    {
                        "name": "write_note",
                        "description": "Writes or appends to a markdown note",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "vault_id": { "type": "string" },
                                "path": { "type": "string" },
                                "content": { "type": "string" },
                                "append": { "type": "boolean" }
                            },
                            "required": ["vault_id", "path", "content", "append"]
                        }
                    },
                    {
                        "name": "create_folder",
                        "description": "Creates a new empty folder inside a vault",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "vault_id": { "type": "string" },
                                "path": { "type": "string", "description": "Relative path to the folder" }
                            },
                            "required": ["vault_id", "path"]
                        }
                    },
                    {
                        "name": "search_vault",
                        "description": "Searches the vault for markdown chunks matching a query",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "vault_id": { "type": "string", "description": "The vault ID, or 'all' to search globally across all codebases" },
                                "query": { "type": "string" }
                            },
                            "required": ["vault_id", "query"]
                        }
                    },
                    {
                        "name": "get_links",
                        "description": "Returns forward links and backlinks for a note",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "vault_id": { "type": "string", "description": "The vault ID, or 'all' to search globally across all codebases" },
                                "file_name": { "type": "string" }
                            },
                            "required": ["vault_id", "file_name"]
                        }
                    },
                    {
                        "name": "delete_note",
                        "description": "Deletes a markdown note",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "vault_id": { "type": "string" },
                                "path": { "type": "string" }
                            },
                            "required": ["vault_id", "path"]
                        }
                    },
                    {
                        "name": "check_integrity",
                        "description": "Scans the entire vault for dead links (orphans) to ensure referential integrity",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "vault_id": { "type": "string" }
                            },
                            "required": ["vault_id"]
                        }
                    },
                    {
                        "name": "list_mocs",
                        "description": "Returns a list of all structural Maps of Content (MOCs) or index files to help the AI satisfy the parent linking constraints",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "vault_id": { "type": "string", "description": "The vault ID, or 'all'" }
                            },
                            "required": ["vault_id"]
                        }
                    },
                    {
                        "name": "rename_note",
                        "description": "Renames a markdown note and automatically updates all backlinks to point to the new name.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "vault_id": { "type": "string" },
                                "old_path": { "type": "string" },
                                "new_path": { "type": "string" }
                            },
                            "required": ["vault_id", "old_path", "new_path"]
                        }
                    },
                    {
                        "name": "batch_transaction",
                        "description": "Executes multiple file operations (writes, creates, renames, deletes) in a single transaction, deferring strict Foreign Key and graph validation until the end. Essential for resolving cyclic dependencies and bulk migrations.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "vault_id": { "type": "string" },
                                "operations": {
                                    "type": "array",
                                    "description": "An array of operation objects. Each object must have a 'type' (e.g. 'write_note', 'rename_note', 'delete_note') and the corresponding arguments for that tool.",
                                    "items": { "type": "object" }
                                }
                            },
                            "required": ["vault_id", "operations"]
                        }
                    },
                    {
                        "name": "scan_legacy_orphans",
                        "description": "Scans the vault for legacy markdown files that lack a valid parent link (`Part of [[...]]` or implicit `_index.md`), giving the AI a targeted list of files that need to be migrated.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "vault_id": { "type": "string" }
                            },
                            "required": ["vault_id"]
                        }
                    }
                ]
            })),
            error: None,
        }
    }

    fn handle_list_prompts(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "prompts": [
                    {
                        "name": "knowledge_base_architect",
                        "description": "The master system prompt for AI-optimized Knowledge Base maintenance",
                        "arguments": []
                    }
                ]
            })),
            error: None,
        }
    }

    fn handle_get_prompt(&self, params: Option<serde_json::Value>, id: serde_json::Value) -> JsonRpcResponse {
        let name = params.and_then(|p| p.get("name").and_then(|n| n.as_str()).map(|n| n.to_string())).unwrap_or_default();
        if name != "knowledge_base_architect" {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError { code: -32602, message: "Prompt not found".to_string() }),
            };
        }

        let prompt_text = r#"You are the Vault Architect. Your objective is to maintain a high-density, easily traversable knowledge graph.

RULES:
1. ATOMICITY: Every concept must be extremely atomic. One indivisible concept per markdown file. Never combine disparate ideas. If a note exceeds 200 words, split it into two notes.
2. CORE ZONES & FOLDER FREEDOM: You have complete freedom to create operational folders (e.g., `/Test Cases`, `/Projects`), but you MUST organize pure knowledge into these exact core folders:
   - `/MOCs`: Maps of Content. Navigational hubs and domain indices.
   - `/Concepts`: Pure atomic knowledge (theories, logic, math).
   - `/Entities`: Real-world instantiations (people, codebase mappings).
   - `/Sources`: Raw unprocessed data or external highlights.
   - `/Logs`: Chronological append-only entries (meetings, daily notes).
   Remember: Operational/working files can live anywhere, but they MUST connect back to the core knowledge graph.
3. STRUCTURAL METADATA (YAML): Every single file must begin with valid YAML. Expected keys are `title`, `tags`, and `summary`. `code`, `file`, and `area` are optionally allowed. No other keys are permitted.
   Example:
   ---
   title: "My Topic"
   tags: ["topic"]
   summary: "A brief summary"
   ---
4. NO ORPHANS: Every new file must link UP to a parent node. This is taken from the folder's `_index.md`, or from the body's `Part of [[...]]` line. `up:` in frontmatter is forbidden.
5. CONTEXTUAL LINKS: Use standard [[WikiLinks]] in the body text for human-readable context. Never drop an isolated link without surrounding text explaining the relationship.
6. NOTES ARE TOPICS: A note is a topic holding many cases. There is no word limit and no need for atomic splitting. Chunking is done automatically on case headings (##/###)."#;

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "description": "The master system prompt for AI-optimized Knowledge Base maintenance",
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": prompt_text
                        }
                    }
                ]
            })),
            error: None,
        }
    }

    async fn handle_call_tool(&self, params: Option<serde_json::Value>, id: serde_json::Value) -> JsonRpcResponse {
        let empty_args = json!({});
        let params = params.unwrap_or(empty_args.clone());
        let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let args = params.get("arguments").unwrap_or(&empty_args);

        let result = match name {
            "list_vaults" => {
                let config = self.config.read().await;
                let vaults: Vec<_> = config.vaults.iter().map(|v| {
                    json!({"id": v.id, "read_only": v.read_only})
                }).collect();
                Ok(json!({ "vaults": vaults }))
            }
            "add_vault" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                let read_only = args.get("read_only").and_then(|v| v.as_bool()).unwrap_or(false);
                self.add_vault_logic(vault_id, path, read_only).await
            }
            "read_note" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                self.read_note_logic(vault_id, path).await
            }
            "write_note" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let append = args.get("append").and_then(|v| v.as_bool()).unwrap_or(false);
                self.write_note_logic(vault_id, path, content, append).await
            }
            "create_folder" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                self.create_folder_logic(vault_id, path).await
            }
            "search_vault" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                self.search_vault_logic(vault_id, query).await
            }
            "get_links" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let file_name = args.get("file_name").and_then(|v| v.as_str()).unwrap_or("");
                self.get_links_logic(vault_id, file_name).await
            }
            "delete_note" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                self.delete_note_logic(vault_id, path).await
            }
            "rename_note" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let old_path = args.get("old_path").and_then(|v| v.as_str()).unwrap_or("");
                let new_path = args.get("new_path").and_then(|v| v.as_str()).unwrap_or("");
                self.rename_note_logic(vault_id, old_path, new_path).await
            }
            "check_integrity" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                self.check_integrity_logic(vault_id).await
            }
            "list_mocs" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                self.list_mocs_logic(vault_id).await
            }
            "batch_transaction" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let empty_vec = Vec::new();
                let ops = args.get("operations").and_then(|v| v.as_array()).unwrap_or(&empty_vec);
                self.batch_transaction_logic(vault_id, ops).await
            }
            "scan_legacy_orphans" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                self.scan_legacy_orphans_logic(vault_id).await
            }
            _ => Err("Unknown tool".to_string()),
        };

        match result {
            Ok(data) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "content": [{ "type": "text", "text": data.to_string() }]
                })),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: e,
                }),
            },
        }
    }

    pub async fn handle_call_tool_for_test(&self, params: Option<serde_json::Value>, id: serde_json::Value) -> JsonRpcResponse {
        self.handle_call_tool(params, id).await
    }

    async fn add_vault_logic(&self, vault_id: &str, path: &str, read_only: bool) -> Result<serde_json::Value, String> {
        let mut config = self.config.write().await;
        
        // Physically create the vault directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(path).await {
            eprintln!("Warning: Could not physically create vault directory at {}: {}", path, e);
        } else {
            // Automatically generate a friendly navigation and rules guide for human users
            let nav_path = std::path::PathBuf::from(path).join("HOW_TO_NAVIGATE.md");
            if !nav_path.exists() {
                let nav_content = r#"# How to Navigate This Vault

> **[AUTO-GENERATED FILE - DO NOT EDIT]**
> This vault is actively managed by an autonomous AI Assistant. It is organized as an interconnected **Knowledge Graph** rather than a strict top-down folder hierarchy. To prevent breaking the AI's internal graph, please adhere to the following structure when manually creating or editing files.

## 1. How to Traverse (For Humans)
If you are not using AI tools to navigate, here is the easiest way to traverse the vault:
- **Start at the MOCs:** Begin in the `/MOCs` directory or the root `_index.md` (if it exists). These act as dashboards grouping links to related concepts.
- **Follow the Parent Links (Bottom-Up):** If you land on a deeply nested note, look for a `Part of [[...]]` line in the body or the folder's `_index.md` to zoom out to its parent category. No note is an orphan; you can always find your way back up.
- **Lateral Links & Backlinks:** Use standard `[[WikiLinks]]` in the body text to explore laterally. Use your markdown editor's **Backlinks pane** to see every atomic concept that mentions the note you are currently viewing.

## 2. Folder Structure
Notes are topics that hold many cases. Use folders freely to organize them.



## 3. The Mandatory YAML Link
Every single markdown file in this vault **MUST** include YAML frontmatter linking it back to a parent concept in the Knowledge Graph. 

Example:
```yaml
---
title: "Main Topic"
tags: []
summary: "Summary here"
---
```
Part of [[Main Topic]]
"#;
                let _ = fs::write(nav_path, nav_content).await;
            }
        }
        
        // Update in-memory state
        config.vaults.retain(|v| v.id != vault_id);
        config.vaults.push(VaultConfig {
            id: vault_id.to_string(),
            path: path.to_string(),
            read_only,
        });

        // Write to disk so it remembers for next time
        let yaml_str = serde_yaml::to_string(&*config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
            
        fs::write(&self.config_path, yaml_str).await
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        Ok(json!({
            "status": "success",
            "message": format!("Vault {} successfully mapped to {}", vault_id, path)
        }))
    }

    fn sanitize_path(path: &str) -> Result<String, String> {
        let normalized = path.replace("\\", "/");
        if normalized.contains("..") || normalized.starts_with("/") || normalized.contains(":") {
            return Err(format!("Invalid path traversal detected in '{}'", path));
        }
        Ok(normalized)
    }

    async fn read_note_logic(&self, vault_id: &str, relative_path: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().await;
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
        
        let safe_path = Self::sanitize_path(relative_path)?;
        let mut full_path = PathBuf::from(&vault.path);
        full_path.push(&safe_path);

        let content = fs::read_to_string(&full_path).await
            .map_err(|e| format!("Failed to read file {}: {}", full_path.display(), e))?;

        let chunks = MarkdownParser::chunk_by_headers(&content);
        Ok(json!({
            "file": relative_path,
            "ast_chunks": chunks.iter().map(|c| {
                json!({"header": c.header, "content": c.content})
            }).collect::<Vec<_>>()
        }))
    }

    async fn write_note_logic(&self, vault_id: &str, relative_path: &str, content: &str, append: bool) -> Result<serde_json::Value, String> {
        self.write_note_logic_ext(vault_id, relative_path, content, append, false).await
    }

    async fn write_note_logic_ext(&self, vault_id: &str, relative_path: &str, content: &str, append: bool, skip_fk_validation: bool) -> Result<serde_json::Value, String> {
        let config = self.config.read().await;
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
            
        if vault.read_only {
            return Err(format!("Vault {} is configured as read-only", vault_id));
        }
        
        let safe_path = Self::sanitize_path(relative_path)?;

        // STRICT ARCHITECTURAL VALIDATION
        if safe_path.to_lowercase() == "how_to_navigate.md" || safe_path.to_lowercase().ends_with("/how_to_navigate.md") {
            return Err("ARCHITECTURAL VIOLATION: You are strictly forbidden from modifying the core HOW_TO_NAVIGATE.md file. This file is auto-generated by the system.".to_string());
        }

        let mut full_path = std::path::PathBuf::from(&vault.path);
        full_path.push(&safe_path);
        
        let mut is_exempt = false;
        let lower_safe = safe_path.to_lowercase();
        if lower_safe.starts_with(".obsidian/") || lower_safe.contains("/.obsidian/") || lower_safe.starts_with("tests/") || lower_safe.contains("/tests/") {
            is_exempt = true;
        }

        if !append && !is_exempt {
            let trimmed_content = content.trim_start();
            if !trimmed_content.starts_with("---") {
                return Err("ARCHITECTURAL VIOLATION: Every new or overwritten file must contain valid YAML frontmatter at the very top, starting with `---`.".to_string());
            }
            
            // Extract the YAML block
            let content_after_first = &trimmed_content[3..];
            if let Some(end_idx) = content_after_first.find("---") {
                let yaml_str = &content_after_first[..end_idx];
                match serde_yaml::from_str::<serde_json::Value>(yaml_str) {
                    Ok(yaml_val) => {
                        if yaml_val.get("up").is_some() {
                            return Err("ARCHITECTURAL VIOLATION: The `up:` frontmatter field is deprecated. Use `Part of [[...]]` in the body or rely on the folder's `_index.md`.".to_string());
                        }
                        if let Some(obj) = yaml_val.as_object() {
                            let expected = ["title", "tags", "summary", "code", "file", "area"];
                            for k_str in obj.keys() {
                                if !expected.contains(&k_str.as_str()) {
                                    return Err(format!("ARCHITECTURAL VIOLATION: Frontmatter key '{}' is not allowed. Expected keys: title, tags, summary, code, file, area.", k_str));
                                }
                            }
                        }
                    },
                    Err(e) => {
                        return Err(format!("ARCHITECTURAL VIOLATION: Invalid YAML frontmatter: {}", e));
                    }
                }
            } else {
                return Err("ARCHITECTURAL VIOLATION: Malformed YAML frontmatter. Ensure the frontmatter is enclosed in `---` blocks.".to_string());
            }
        }
        
        // FOREIGN KEY CONSTRAINT: Check outgoing links against existing files
        if !skip_fk_validation && !is_exempt {
            let mut all_files = std::collections::HashSet::new();
            if let Ok(md_files) = crate::vault::VaultScanner::scan_markdown_files(&vault.path) {
                for f in &md_files {
                    let rel = f.strip_prefix(&vault.path).unwrap_or(f).to_string_lossy().replace("\\", "/");
                    let rel_lower = rel.to_lowercase();
                    if let Some(stem) = f.file_stem().and_then(|s| s.to_str()) {
                        all_files.insert(stem.to_lowercase());
                    }
                    if let Some(no_ext) = rel_lower.strip_suffix(".md") {
                        all_files.insert(no_ext.to_string());
                    }
                    all_files.insert(rel_lower);
                }
            }
            // Allow the file to link to itself during creation
            let safe_lower = safe_path.to_lowercase();
            if let Some(stem) = std::path::Path::new(&safe_path).file_stem().and_then(|s| s.to_str()) {
                all_files.insert(stem.to_lowercase());
            }
            if let Some(no_ext) = safe_lower.strip_suffix(".md") {
                all_files.insert(no_ext.to_string());
            }
            all_files.insert(safe_lower);

            let extracted_links = crate::parser::MarkdownParser::extract_links(content);
            let has_part_of = content.contains("Part of [[");
            let is_index = safe_path.to_lowercase().ends_with("_index.md");
            
            if !has_part_of && !is_index {
                let mut index_path = std::path::PathBuf::from(&vault.path);
                if let Some(parent) = std::path::Path::new(&safe_path).parent() {
                    index_path.push(parent);
                }
                index_path.push("_index.md");
                
                if !index_path.exists() {
                     return Err("ARCHITECTURAL VIOLATION: Parent _index.md not found in the folder. You must create it first or use `Part of [[...]]` in the body.".to_string());
                }
            }
            for link in extracted_links {
                if link.starts_with("obsidian://") { continue; }
                let mut target_file = link.as_str();
                if let Some((f, _)) = link.split_once('#') {
                    target_file = f;
                }
                
                let mut effective_target = target_file.to_string();
                if target_file.to_lowercase() == "_index" {
                    if let Some(parent) = std::path::Path::new(&safe_path).parent() {
                        let parent_str = parent.to_string_lossy().replace("\\", "/");
                        if !parent_str.is_empty() {
                            effective_target = format!("{}/_index", parent_str);
                        }
                    }
                }
                
                if effective_target.is_empty() {
                    effective_target = safe_path.clone();
                }
                
                if !all_files.contains(&effective_target.to_lowercase()) {
                    return Err(format!("ARCHITECTURAL VIOLATION: Foreign Key Constraint failed. You attempted to link to `[[{}]]`, but this file does not exist in the vault. You must create the target file first.", link));
                }
            }
        }

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| format!("Failed to create directories: {}", e))?;
        }

        if append {
            use tokio::fs::OpenOptions;
            use tokio::io::AsyncWriteExt;
            let mut file = OpenOptions::new().create(true).append(true).open(&full_path).await
                .map_err(|e| format!("Failed to open file for appending: {}", e))?;
            file.write_all(content.as_bytes()).await
                .map_err(|e| format!("Failed to append to file: {}", e))?;
        } else {
            fs::write(&full_path, content).await
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }

        Ok(json!({
            "status": "success",
            "file": safe_path,
            "action": if append { "appended" } else { "written" }
        }))
    }

    async fn create_folder_logic(&self, vault_id: &str, relative_path: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().await;
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
            
        if vault.read_only {
            return Err(format!("Vault {} is configured as read-only", vault_id));
        }
        
        let safe_path = Self::sanitize_path(relative_path)?;
        let mut full_path = PathBuf::from(&vault.path);
        full_path.push(&safe_path);

        fs::create_dir_all(&full_path).await
            .map_err(|e| format!("Failed to create folder: {}", e))?;

        Ok(json!({
            "status": "success",
            "folder": safe_path
        }))
    }

    async fn search_vault_logic(&self, vault_id: &str, query: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().await;
        let vaults_to_search: Vec<_> = if vault_id == "all" {
            config.vaults.iter().collect()
        } else {
            vec![config.vaults.iter().find(|v| v.id == vault_id)
                .ok_or_else(|| format!("Vault {} not found", vault_id))?]
        };
            
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        
        for vault in vaults_to_search {
            if let Ok(md_files) = crate::vault::VaultScanner::scan_markdown_files(&vault.path) {
                for file in md_files {
                    if let Ok(content) = fs::read_to_string(&file).await {
                        let relative_path = file.strip_prefix(&vault.path).unwrap_or(&file).to_string_lossy().to_string();
                        let chunks = MarkdownParser::chunk_by_headers(&content);
                        for chunk in chunks {
                            if chunk.header.to_lowercase().contains(&query_lower) || chunk.content.to_lowercase().contains(&query_lower) {
                                results.push(json!({
                                    "vault_id": vault.id,
                                    "file": relative_path,
                                    "header": chunk.header,
                                    "snippet": chunk.content.chars().take(300).collect::<String>()
                                }));
                            }
                        }
                    }
                }
            }
        }
        results.truncate(10);
        Ok(json!({
            "query": query,
            "results": results
        }))
    }

    async fn get_links_logic(&self, vault_id: &str, file_name: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().await;
        let vaults_to_search: Vec<_> = if vault_id == "all" {
            config.vaults.iter().collect()
        } else {
            vec![config.vaults.iter().find(|v| v.id == vault_id)
                .ok_or_else(|| format!("Vault {} not found", vault_id))?]
        };
            
        let mut forward_links = Vec::new();
        let mut backlinks = Vec::new();
        let target_base_name = std::path::Path::new(file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file_name);

        for vault in vaults_to_search {
            if let Ok(md_files) = crate::vault::VaultScanner::scan_markdown_files(&vault.path) {
                for file in md_files {
                    if let Ok(content) = fs::read_to_string(&file).await {
                        let current_relative_path = file.strip_prefix(&vault.path).unwrap_or(&file).to_string_lossy().to_string();
                        let current_base_name = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                        let extracted = MarkdownParser::extract_links(&content);
                        
                        if current_base_name == target_base_name || current_relative_path == file_name {
                            forward_links = extracted.clone();
                        }
                        
                        if extracted.iter().any(|link| link.as_str() == target_base_name) {
                            backlinks.push(json!({"vault_id": vault.id.clone(), "file": current_relative_path}));
                        }
                    }
                }
            }
        }
        
        Ok(json!({
            "file": file_name,
            "forward_links": forward_links,
            "backlinks": backlinks
        }))
    }

    async fn delete_note_logic(&self, vault_id: &str, relative_path: &str) -> Result<serde_json::Value, String> {
        self.delete_note_logic_ext(vault_id, relative_path, false).await
    }

    async fn delete_note_logic_ext(&self, vault_id: &str, relative_path: &str, skip_fk_validation: bool) -> Result<serde_json::Value, String> {
        let config = self.config.read().await;
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
            
        if vault.read_only {
            return Err(format!("Vault {} is configured as read-only", vault_id));
        }

        let safe_path = Self::sanitize_path(relative_path)?;
        let is_exempt = safe_path.to_lowercase().starts_with("tests/") || safe_path.to_lowercase().starts_with(".obsidian/");

        if safe_path.to_lowercase() == "how_to_navigate.md" || safe_path.to_lowercase().ends_with("/how_to_navigate.md") {
            return Err("ARCHITECTURAL VIOLATION: You cannot delete the auto-generated HOW_TO_NAVIGATE.md file.".to_string());
        }
        
        let mut full_path = std::path::PathBuf::from(&vault.path);
        full_path.push(&safe_path);

        // FOREIGN KEY CONSTRAINT: Prevent deleting files that are linked by other files
        if !skip_fk_validation && !is_exempt {
            let safe_lower = safe_path.to_lowercase();
            let target_stem = std::path::Path::new(&safe_path).file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            let target_no_ext = safe_lower.strip_suffix(".md").unwrap_or(&safe_lower).to_string();

            if let Ok(md_files) = crate::vault::VaultScanner::scan_markdown_files(&vault.path) {
                for file in &md_files {
                    let rel = file.strip_prefix(&vault.path).unwrap_or(&file).to_string_lossy().replace("\\", "/");
                    if rel.to_lowercase() == safe_lower {
                        continue; // allow file to link to itself, or it's the file we're deleting
                    }
                    if let Ok(content) = fs::read_to_string(&file).await {
                        let extracted = crate::parser::MarkdownParser::extract_links(&content);
                        if extracted.iter().any(|l| {
                            let link_lower = l.to_lowercase();
                            link_lower == target_stem || link_lower == target_no_ext || link_lower == safe_lower
                        }) {
                            return Err(format!("ARCHITECTURAL VIOLATION: Foreign Key Constraint failed. Cannot delete `{}` because `{}` links to it. You must remove the link first.", safe_path, rel));
                        }
                    }
                }
            }
        }

        if full_path.exists() {
            fs::remove_file(&full_path).await.map_err(|e| format!("Failed to delete file: {}", e))?;
            Ok(json!({
                "status": "success",
                "message": format!("File {} deleted successfully.", safe_path)
            }))
        } else {
            Err(format!("File {} does not exist.", safe_path))
        }
    }

    async fn rename_note_logic(&self, vault_id: &str, old_path: &str, new_path: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().await;
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
            
        if vault.read_only {
            return Err(format!("Vault {} is configured as read-only", vault_id));
        }

        let safe_old = Self::sanitize_path(old_path)?;
        let safe_new = Self::sanitize_path(new_path)?;

        if safe_old.to_lowercase() == "how_to_navigate.md" || safe_old.to_lowercase().ends_with("/how_to_navigate.md") {
            return Err("ARCHITECTURAL VIOLATION: You cannot rename the auto-generated HOW_TO_NAVIGATE.md file.".to_string());
        }

        let mut old_full_path = std::path::PathBuf::from(&vault.path);
        old_full_path.push(&safe_old);
        
        let mut new_full_path = std::path::PathBuf::from(&vault.path);
        new_full_path.push(&safe_new);

        if !old_full_path.exists() {
            return Err(format!("Source file {} does not exist.", safe_old));
        }
        if new_full_path.exists() {
            return Err(format!("Destination file {} already exists.", safe_new));
        }

        let old_stem = old_full_path.file_stem().unwrap().to_string_lossy().to_string();
        let new_stem = new_full_path.file_stem().unwrap().to_string_lossy().to_string();
        
        let safe_old_lower = safe_old.to_lowercase();
        let old_stem_lower = old_stem.to_lowercase();
        let old_no_ext = safe_old_lower.strip_suffix(".md").unwrap_or(&safe_old_lower).to_string();
        
        let mut updated_files = Vec::new();

        // Find all backlinks and update them atomically
        let old_targets = vec![old_stem_lower, old_no_ext, safe_old_lower];
        if let Ok(md_files) = crate::vault::VaultScanner::scan_markdown_files(&vault.path) {
            for file in &md_files {
                if file == &old_full_path { continue; }
                if let Ok(content) = fs::read_to_string(&file).await {
                    let extracted = crate::parser::MarkdownParser::extract_links(&content);
                    let mut needs_update = false;
                    for link in extracted {
                        let link_lower = link.to_lowercase();
                        if old_targets.iter().any(|t| t == &link_lower) {
                            needs_update = true;
                            break;
                        }
                    }

                    if needs_update {
                        let new_content = crate::parser::MarkdownParser::rename_links(&content, &old_targets, &new_stem);
                        if let Ok(_) = fs::write(file, new_content).await {
                            updated_files.push(file.strip_prefix(&vault.path).unwrap_or(file).to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        // Now safe to rename the file itself
        if let Some(parent) = new_full_path.parent() {
            fs::create_dir_all(parent).await.unwrap_or(());
        }
        
        fs::rename(&old_full_path, &new_full_path).await.map_err(|e| format!("Failed to rename file: {}", e))?;

        Ok(json!({
            "status": "success",
            "message": format!("File renamed from {} to {}.", safe_old, safe_new),
            "updated_backlinks": updated_files
        }))
    }

    async fn check_integrity_logic(&self, vault_id: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().await;
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
            
        let mut dead_links = Vec::new();
        let mut all_files = std::collections::HashMap::new();
        
        for entry in walkdir::WalkDir::new(&vault.path).into_iter().filter_map(|e| e.ok()) {
            if entry.path().is_file() {
                let path_buf = entry.path().to_path_buf();
                if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                    all_files.insert(stem.to_lowercase(), path_buf.clone());
                }
                let rel = entry.path().strip_prefix(&vault.path).unwrap_or(entry.path()).to_string_lossy().replace("\\", "/");
                let rel_lower = rel.to_lowercase();
                all_files.insert(rel_lower.clone(), path_buf.clone());
                if let Some((no_ext, _)) = rel_lower.rsplit_once('.') {
                    all_files.insert(no_ext.to_string(), path_buf.clone());
                }
            }
        }
        
        if let Ok(md_files) = crate::vault::VaultScanner::scan_markdown_files(&vault.path) {
            for file in md_files {
                if let Ok(content) = tokio::fs::read_to_string(&file).await {
                    let extracted = crate::parser::MarkdownParser::extract_links(&content);
                    let relative_path = file.strip_prefix(&vault.path).unwrap_or(&file).to_string_lossy().replace("\\", "/").to_string();
                    
                    for link in extracted {
                        if link.starts_with("obsidian://") { continue; }
                        
                        let mut target_file = link.as_str();
                        let mut target_heading = None;
                        if let Some((f, h)) = link.split_once('#') {
                            target_file = f;
                            target_heading = Some(h);
                        }
                        
                        let mut effective_target = target_file.to_string();
                        if target_file.to_lowercase() == "_index" {
                            if let Some(parent) = std::path::Path::new(&relative_path).parent() {
                                let parent_str = parent.to_string_lossy().replace("\\", "/");
                                if !parent_str.is_empty() {
                                    effective_target = format!("{}/_index", parent_str);
                                }
                            }
                        }
                        
                        if effective_target.is_empty() {
                            effective_target = relative_path.clone();
                        }
                        
                        if let Some(actual_path) = all_files.get(&effective_target.to_lowercase()) {
                            if let Some(heading) = target_heading {
                                if let Ok(target_content) = std::fs::read_to_string(actual_path) {
                                    let heading_lower = heading.to_lowercase();
                                    let content_lower = target_content.to_lowercase();
                                    if !content_lower.contains(&heading_lower) {
                                        dead_links.push(serde_json::json!({
                                            "source_file": relative_path,
                                            "broken_link": link,
                                            "reason": "Heading not found"
                                        }));
                                    }
                                }
                            }
                        } else {
                            dead_links.push(serde_json::json!({
                                "source_file": relative_path,
                                "broken_link": link,
                                "reason": "File not found"
                            }));
                        }
                    }
                }
            }
        }
        
        Ok(serde_json::json!({
            "status": "success",
            "dead_links_found": dead_links.len(),
            "dead_links": dead_links
        }))
    }

    async fn list_mocs_logic(&self, vault_id: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().await;
        let vaults_to_search: Vec<_> = if vault_id == "all" {
            config.vaults.iter().collect()
        } else {
            vec![config.vaults.iter().find(|v| v.id == vault_id)
                .ok_or_else(|| format!("Vault {} not found", vault_id))?]
        };
            
        let mut mocs = Vec::new();
        
        for vault in vaults_to_search {
            if let Ok(md_files) = crate::vault::VaultScanner::scan_markdown_files(&vault.path) {
                for file in md_files {
                    let rel_path = file.strip_prefix(&vault.path).unwrap_or(&file).to_string_lossy().replace("\\", "/");
                    let rel_lower = rel_path.to_lowercase();
                    let stem_lower = file.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                    
                    if rel_lower.starts_with("mocs/") || rel_lower.contains("/mocs/") || stem_lower.ends_with("moc") || stem_lower == "_index" {
                        mocs.push(json!({
                            "vault_id": vault.id.clone(),
                            "file": rel_path
                        }));
                    }
                }
            }
        }
        
        Ok(json!({
            "mocs": mocs
        }))
    }
    
    async fn batch_transaction_logic(&self, vault_id: &str, ops: &Vec<serde_json::Value>) -> Result<serde_json::Value, String> {
        let mut results = Vec::new();
        for op in ops {
            let op_type = op.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match op_type {
                "write_note" => {
                    let path = op.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    let content = op.get("content").and_then(|v| v.as_str()).unwrap_or("");
                    let append = op.get("append").and_then(|v| v.as_bool()).unwrap_or(false);
                    let res = self.write_note_logic_ext(vault_id, path, content, append, true).await;
                    results.push(json!({"type": "write_note", "path": path, "result": res.unwrap_or_else(|e| json!({"error": e}))}));
                }
                "create_folder" => {
                    let path = op.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    let res = self.create_folder_logic(vault_id, path).await;
                    results.push(json!({"type": "create_folder", "path": path, "result": res.unwrap_or_else(|e| json!({"error": e}))}));
                }
                "rename_note" => {
                    let old_path = op.get("old_path").and_then(|v| v.as_str()).unwrap_or("");
                    let new_path = op.get("new_path").and_then(|v| v.as_str()).unwrap_or("");
                    let res = self.rename_note_logic(vault_id, old_path, new_path).await;
                    results.push(json!({"type": "rename_note", "old_path": old_path, "new_path": new_path, "result": res.unwrap_or_else(|e| json!({"error": e}))}));
                }
                "delete_note" => {
                    let path = op.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    let res = self.delete_note_logic_ext(vault_id, path, true).await;
                    results.push(json!({"type": "delete_note", "path": path, "result": res.unwrap_or_else(|e| json!({"error": e}))}));
                }
                _ => {
                    results.push(json!({"type": op_type, "error": "Unknown batch operation type"}));
                }
            }
        }
        
        // Final integrity check
        let integrity = self.check_integrity_logic(vault_id).await;
        let mut dead_links = 0;
        if let Ok(integ) = integrity {
            if let Some(count) = integ.get("dead_links_found").and_then(|v| v.as_u64()) {
                dead_links = count;
            }
        }

        Ok(json!({
            "status": "success",
            "operations_completed": results.len(),
            "results": results,
            "final_integrity_warning": if dead_links > 0 { Some(format!("Transaction committed, but left {} dead links in the vault.", dead_links)) } else { None }
        }))
    }

    async fn scan_legacy_orphans_logic(&self, vault_id: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().await;
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
            
        let mut orphans = Vec::new();
        if let Ok(md_files) = crate::vault::VaultScanner::scan_markdown_files(&vault.path) {
            for file in md_files {
                let rel = file.strip_prefix(&vault.path).unwrap_or(&file).to_string_lossy().to_string();
                if rel.to_lowercase() == "how_to_navigate.md" { continue; }
                
                if let Ok(content) = tokio::fs::read_to_string(&file).await {
                    let has_part_of = content.contains("Part of [[");
                    let is_index = rel.to_lowercase().ends_with("_index.md");
                    
                    let mut has_implicit_parent = false;
                    if let Some(parent) = file.parent() {
                        if parent.join("_index.md").exists() {
                            has_implicit_parent = true;
                        }
                    }
                    
                    if !has_part_of && !is_index && !has_implicit_parent {
                        orphans.push(rel);
                    }
                }
            }
        }
        
        Ok(json!({
            "orphans_found": orphans.len(),
            "orphans": orphans
        }))
    }
}
