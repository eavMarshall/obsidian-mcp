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
                        "tools": {}
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
            
        fs::write("config.yaml", yaml_str)
            .map_err(|e| format!("Failed to write config.yaml: {}", e))?;

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
        
        let mut full_path = PathBuf::from(&vault.path);
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
}
