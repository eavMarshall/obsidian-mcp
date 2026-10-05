use crate::config::{AppConfig, VaultConfig};
use crate::mcp::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::parser::MarkdownParser;
use anyhow::Result;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct McpServer {
    config: Arc<RwLock<AppConfig>>,
}

impl McpServer {
    pub fn new(config: Arc<RwLock<AppConfig>>) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut reader = BufReader::new(stdin).lines();
        
        // Open log file for debugging
        let mut log_file = fs::OpenOptions::new().create(true).append(true).open("mcp_log.txt")?;

        while let Some(line) = reader.next_line().await? {
            if line.trim().is_empty() { continue; }
            
            // Log incoming
            use std::io::Write;
            writeln!(log_file, "INCOMING: {}", line)?;
            
            match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(req) => {
                    if let Some(resp) = self.handle_request(req) {
                        let resp_str = serde_json::to_string(&resp)? + "\n";
                        writeln!(log_file, "OUTGOING: {}", resp_str.trim())?;
                        stdout.write_all(resp_str.as_bytes()).await?;
                        stdout.flush().await?;
                    }
                }
                Err(e) => {
                    writeln!(log_file, "PARSE ERROR: {} - Payload: {}", e, line)?;
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

    pub fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
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
            "tools/call" => Some(self.handle_call_tool(req.params, id)),
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
3. STRUCTURAL METADATA (YAML): Every single file (whether a concept, test case, or script) must begin with valid YAML containing at minimum the `up:` field which links to its parent category or MOC.
   Example:
   ---
   up: "[[Main Topic]]"
   related: "[[Lateral Topic]]"
   ---
4. NO ORPHANS: Every new file must link UP to at least one broader parent node (`up:`). Unlinked nodes are strictly prohibited.
5. CONTEXTUAL LINKS: Use standard [[WikiLinks]] in the body text for human-readable context. Never drop an isolated link without surrounding text explaining the relationship.
6. APPEND > REWRITE: To save token bandwidth, try to append to existing files instead of rewriting large monolithic documents."#;

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

    fn handle_call_tool(&self, params: Option<serde_json::Value>, id: serde_json::Value) -> JsonRpcResponse {
        let empty_args = json!({});
        let params = params.unwrap_or(empty_args.clone());
        let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let args = params.get("arguments").unwrap_or(&empty_args);

        let result = match name {
            "list_vaults" => {
                let config = self.config.read().unwrap();
                let vaults: Vec<_> = config.vaults.iter().map(|v| {
                    json!({"id": v.id, "read_only": v.read_only})
                }).collect();
                Ok(json!({ "vaults": vaults }))
            }
            "add_vault" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                let read_only = args.get("read_only").and_then(|v| v.as_bool()).unwrap_or(false);
                self.add_vault_logic(vault_id, path, read_only)
            }
            "read_note" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                self.read_note_logic(vault_id, path)
            }
            "write_note" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let append = args.get("append").and_then(|v| v.as_bool()).unwrap_or(false);
                self.write_note_logic(vault_id, path, content, append)
            }
            "create_folder" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                self.create_folder_logic(vault_id, path)
            }
            "search_vault" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                self.search_vault_logic(vault_id, query)
            }
            "get_links" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let file_name = args.get("file_name").and_then(|v| v.as_str()).unwrap_or("");
                self.get_links_logic(vault_id, file_name)
            }
            "delete_note" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                self.delete_note_logic(vault_id, path)
            }
            "rename_note" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                let old_path = args.get("old_path").and_then(|v| v.as_str()).unwrap_or("");
                let new_path = args.get("new_path").and_then(|v| v.as_str()).unwrap_or("");
                self.rename_note_logic(vault_id, old_path, new_path)
            }
            "check_integrity" => {
                let vault_id = args.get("vault_id").and_then(|v| v.as_str()).unwrap_or("");
                self.check_integrity_logic(vault_id)
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

    pub fn handle_call_tool_for_test(&self, params: Option<serde_json::Value>, id: serde_json::Value) -> JsonRpcResponse {
        self.handle_call_tool(params, id)
    }

    fn add_vault_logic(&self, vault_id: &str, path: &str, read_only: bool) -> Result<serde_json::Value, String> {
        let mut config = self.config.write().unwrap();
        
        // Physically create the vault directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(path) {
            eprintln!("Warning: Could not physically create vault directory at {}: {}", path, e);
        } else {
            // Automatically generate a human-readable rulebook for other human users
            let rules_path = std::path::PathBuf::from(path).join("_VAULT_RULES.md");
            if !rules_path.exists() {
                let rules_content = r#"# Vault Architectural Rules

> **[AUTO-GENERATED FILE - DO NOT EDIT]**
> This file was automatically generated by the Obsidian MCP Gateway. 
> This vault is actively managed by an autonomous AI Assistant. To prevent breaking the AI's internal knowledge graph, please adhere to the following rules when manually creating or editing files.

## 1. Core Ontological Zones
Pure knowledge must be strictly organized into these root directories:
- `/MOCs`: Maps of Content (navigational hubs)
- `/Concepts`: Pure atomic knowledge (one idea per file)
- `/Entities`: Real-world instantiations (people, codebase mappings)
- `/Sources`: Raw unprocessed data
- `/Logs`: Chronological append-only entries

## 2. Operational Freedom
You may create any other folders you need for active work (e.g., `/Test Cases`, `/Projects`, `/Drafts`).

## 3. The Mandatory YAML Link
Every single markdown file in this vault **MUST** include YAML frontmatter linking it back to a parent concept in the Knowledge Graph. Do not leave "orphan" files.

Example:
```yaml
---
up: "[[Main Topic]]"
---
```
"#;
                let _ = fs::write(rules_path, rules_content);
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
            
        let config_path = std::path::PathBuf::from("obsidian-mcp-config.yaml");

        fs::write(&config_path, yaml_str)
            .map_err(|e| format!("Failed to write obsidian-mcp-config.yaml: {}", e))?;

        Ok(json!({
            "status": "success",
            "message": format!("Vault {} successfully mapped to {}", vault_id, path)
        }))
    }

    fn read_note_logic(&self, vault_id: &str, relative_path: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().unwrap();
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
        
        let mut full_path = PathBuf::from(&vault.path);
        full_path.push(relative_path);

        let content = fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read file {}: {}", full_path.display(), e))?;

        let chunks = MarkdownParser::chunk_by_headers(&content);
        Ok(json!({
            "file": relative_path,
            "ast_chunks": chunks.iter().map(|c| {
                json!({"header": c.header, "content": c.content})
            }).collect::<Vec<_>>()
        }))
    }

    fn write_note_logic(&self, vault_id: &str, relative_path: &str, content: &str, append: bool) -> Result<serde_json::Value, String> {
        let config = self.config.read().unwrap();
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
            
        if vault.read_only {
            return Err(format!("Vault {} is configured as read-only", vault_id));
        }

        // STRICT ARCHITECTURAL VALIDATION (Only validate on new files, not appends)
        if relative_path.to_lowercase() == "_vault_rules.md" || relative_path.to_lowercase().ends_with("/_vault_rules.md") {
            return Err("ARCHITECTURAL VIOLATION: You are strictly forbidden from modifying the core _VAULT_RULES.md file. This file is auto-generated by the system.".to_string());
        }

        if !append {
            if !content.trim().starts_with("---") {
                return Err("ARCHITECTURAL VIOLATION: Every new file must contain valid YAML frontmatter at the very top, starting with `---`.".to_string());
            }
            
            // Extract the YAML block
            let parts: Vec<&str> = content.split("---").collect();
            if parts.len() < 3 {
                return Err("ARCHITECTURAL VIOLATION: Malformed YAML frontmatter. Ensure the frontmatter is enclosed in `---` blocks.".to_string());
            }
            
            let yaml_str = parts[1];
            match serde_yaml::from_str::<serde_json::Value>(yaml_str) {
                Ok(yaml_val) => {
                    if let Some(up_val) = yaml_val.get("up") {
                        if up_val.is_null() || (up_val.is_string() && up_val.as_str().unwrap().trim().is_empty()) {
                            return Err("ARCHITECTURAL VIOLATION: The `up:` field in the YAML frontmatter cannot be empty or null.".to_string());
                        }
                    } else {
                        return Err("ARCHITECTURAL VIOLATION: Every new file must include at minimum an `up:` field linking to its parent node or MOC (e.g. `up: \"[[Parent Topic]]\"`). No orphans allowed.".to_string());
                    }
                },
                Err(e) => {
                    return Err(format!("ARCHITECTURAL VIOLATION: Invalid YAML frontmatter: {}", e));
                }
            }
        }
        
        // FOREIGN KEY CONSTRAINT: Check outgoing links against existing files
        let mut all_files = std::collections::HashSet::new();
        if let Ok(md_files) = crate::vault::VaultScanner::scan_markdown_files(&vault.path) {
            for f in &md_files {
                if let Some(stem) = f.file_stem().and_then(|s| s.to_str()) {
                    all_files.insert(stem.to_lowercase());
                }
            }
        }
        // Allow the file to link to itself during creation
        if let Some(stem) = std::path::Path::new(&relative_path).file_stem().and_then(|s| s.to_str()) {
            all_files.insert(stem.to_lowercase());
        }

        let extracted_links = crate::parser::MarkdownParser::extract_links(content);
        for link in extracted_links {
            if !all_files.contains(&link.to_lowercase()) {
                return Err(format!("ARCHITECTURAL VIOLATION: Foreign Key Constraint failed. You attempted to link to `[[{}]]`, but this file does not exist in the vault. You must create the target file first.", link));
            }
        }

        let mut full_path = std::path::PathBuf::from(&vault.path);
        full_path.push(relative_path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directories: {}", e))?;
        }

        if append {
            use std::fs::OpenOptions;
            use std::io::Write;
            let mut file = OpenOptions::new().create(true).append(true).open(&full_path)
                .map_err(|e| format!("Failed to open file for appending: {}", e))?;
            file.write_all(content.as_bytes())
                .map_err(|e| format!("Failed to append to file: {}", e))?;
        } else {
            fs::write(&full_path, content)
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }

        Ok(json!({
            "status": "success",
            "file": relative_path,
            "action": if append { "appended" } else { "written" }
        }))
    }

    fn create_folder_logic(&self, vault_id: &str, relative_path: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().unwrap();
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
            
        if vault.read_only {
            return Err(format!("Vault {} is configured as read-only", vault_id));
        }
        
        let mut full_path = PathBuf::from(&vault.path);
        full_path.push(relative_path);

        fs::create_dir_all(&full_path)
            .map_err(|e| format!("Failed to create folder: {}", e))?;

        Ok(json!({
            "status": "success",
            "folder": relative_path
        }))
    }

    fn search_vault_logic(&self, vault_id: &str, query: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().unwrap();
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
                    if let Ok(content) = fs::read_to_string(&file) {
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

    fn get_links_logic(&self, vault_id: &str, file_name: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().unwrap();
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
                    if let Ok(content) = fs::read_to_string(&file) {
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

    fn delete_note_logic(&self, vault_id: &str, relative_path: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().unwrap();
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
            
        if vault.read_only {
            return Err(format!("Vault {} is configured as read-only", vault_id));
        }

        if relative_path.to_lowercase() == "_vault_rules.md" || relative_path.to_lowercase().ends_with("/_vault_rules.md") {
            return Err("ARCHITECTURAL VIOLATION: You cannot delete the auto-generated _VAULT_RULES.md file.".to_string());
        }
        
        // FOREIGN KEY CONSTRAINT: Prevent deleting files that are linked by other files
        if let Some(target_stem) = std::path::Path::new(&relative_path).file_stem().and_then(|s| s.to_str()) {
            let target_lower = target_stem.to_lowercase();
            if let Ok(md_files) = crate::vault::VaultScanner::scan_markdown_files(&vault.path) {
                for file in &md_files {
                    if let Ok(content) = fs::read_to_string(&file) {
                        let extracted = crate::parser::MarkdownParser::extract_links(&content);
                        if extracted.iter().any(|l| l.to_lowercase() == target_lower) {
                            let rel = file.strip_prefix(&vault.path).unwrap_or(&file).to_string_lossy().to_string();
                            if rel != relative_path {
                                return Err(format!("ARCHITECTURAL VIOLATION: Foreign Key Constraint failed. Cannot delete `{}` because `{}` links to it. You must remove the link first.", relative_path, rel));
                            }
                        }
                    }
                }
            }
        }

        let mut full_path = std::path::PathBuf::from(&vault.path);
        full_path.push(relative_path);

        if full_path.exists() {
            fs::remove_file(&full_path).map_err(|e| format!("Failed to delete file: {}", e))?;
            Ok(json!({
                "status": "success",
                "message": format!("File {} deleted successfully.", relative_path)
            }))
        } else {
            Err(format!("File {} does not exist.", relative_path))
        }
    }

    fn rename_note_logic(&self, vault_id: &str, old_path: &str, new_path: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().unwrap();
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
            
        if vault.read_only {
            return Err(format!("Vault {} is configured as read-only", vault_id));
        }

        if old_path.to_lowercase() == "_vault_rules.md" || old_path.to_lowercase().ends_with("/_vault_rules.md") {
            return Err("ARCHITECTURAL VIOLATION: You cannot rename the auto-generated _VAULT_RULES.md file.".to_string());
        }

        let mut old_full_path = std::path::PathBuf::from(&vault.path);
        old_full_path.push(old_path);
        
        let mut new_full_path = std::path::PathBuf::from(&vault.path);
        new_full_path.push(new_path);

        if !old_full_path.exists() {
            return Err(format!("Source file {} does not exist.", old_path));
        }
        if new_full_path.exists() {
            return Err(format!("Destination file {} already exists.", new_path));
        }

        let old_stem = old_full_path.file_stem().unwrap().to_string_lossy().to_string();
        let new_stem = new_full_path.file_stem().unwrap().to_string_lossy().to_string();
        let old_lower = old_stem.to_lowercase();
        
        let mut updated_files = Vec::new();

        // Find all backlinks and update them atomically
        if let Ok(md_files) = crate::vault::VaultScanner::scan_markdown_files(&vault.path) {
            for file in &md_files {
                if file == &old_full_path { continue; }
                if let Ok(content) = fs::read_to_string(&file) {
                    let extracted = crate::parser::MarkdownParser::extract_links(&content);
                    if extracted.iter().any(|l| l.to_lowercase() == old_lower) {
                        // Very naive but effective link replacement for now
                        let mut new_content = content.replace(&format!("[[{}]]", old_stem), &format!("[[{}]]", new_stem));
                        new_content = new_content.replace(&format!("[[{}|", old_stem), &format!("[[{}|", new_stem));
                        new_content = new_content.replace(&format!("[[{}#", old_stem), &format!("[[{}#", new_stem));
                        
                        if let Ok(_) = fs::write(file, new_content) {
                            updated_files.push(file.strip_prefix(&vault.path).unwrap_or(file).to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        // Now safe to rename the file itself
        if let Some(parent) = new_full_path.parent() {
            fs::create_dir_all(parent).unwrap_or(());
        }
        
        fs::rename(&old_full_path, &new_full_path).map_err(|e| format!("Failed to rename file: {}", e))?;

        Ok(json!({
            "status": "success",
            "message": format!("File renamed from {} to {}.", old_path, new_path),
            "updated_backlinks": updated_files
        }))
    }

    fn check_integrity_logic(&self, vault_id: &str) -> Result<serde_json::Value, String> {
        let config = self.config.read().unwrap();
        let vault = config.vaults.iter().find(|v| v.id == vault_id)
            .ok_or_else(|| format!("Vault {} not found", vault_id))?;
            
        let mut dead_links = Vec::new();
        let mut all_files = std::collections::HashSet::new();
        
        // Pass 1: Collect all valid basenames
        if let Ok(md_files) = crate::vault::VaultScanner::scan_markdown_files(&vault.path) {
            for file in &md_files {
                if let Some(stem) = file.file_stem().and_then(|s| s.to_str()) {
                    all_files.insert(stem.to_lowercase());
                }
            }
            
            // Pass 2: Check every link in every file
            for file in md_files {
                if let Ok(content) = std::fs::read_to_string(&file) {
                    let extracted = crate::parser::MarkdownParser::extract_links(&content);
                    let relative_path = file.strip_prefix(&vault.path).unwrap_or(&file).to_string_lossy().to_string();
                    
                    for link in extracted {
                        if !all_files.contains(&link.to_lowercase()) {
                            dead_links.push(json!({
                                "source_file": relative_path,
                                "broken_link": link
                            }));
                        }
                    }
                }
            }
        }

        Ok(json!({
            "status": "success",
            "dead_links_found": dead_links.len(),
            "dead_links": dead_links
        }))
    }
}
