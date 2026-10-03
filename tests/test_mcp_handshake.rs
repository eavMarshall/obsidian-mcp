use obsidian_mcp::config::AppConfig;
use obsidian_mcp::server::McpServer;
use obsidian_mcp::mcp::JsonRpcRequest;
use std::sync::{Arc, RwLock};

#[test]
fn test_mcp_handshake() {
    let config = AppConfig {
        vaults: vec![],
    };

    let server = McpServer::new(Arc::new(RwLock::new(config)));

    // Test initialize request
    let init_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(1)),
        method: "initialize".to_string(),
        params: None,
    };

    let init_resp = server.handle_request(init_req).expect("Expected a response for initialize");
    
    assert_eq!(init_resp.id, serde_json::json!(1));
    assert!(init_resp.error.is_none());
    
    let result = init_resp.result.unwrap();
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert!(result["capabilities"].get("tools").is_some());
    assert_eq!(result["serverInfo"]["name"], "obsidian-mcp");

    // Test notifications/initialized (should return None since it's a notification)
    let notif_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: None,
        method: "notifications/initialized".to_string(),
        params: None,
    };

    let notif_resp = server.handle_request(notif_req);
    assert!(notif_resp.is_none());
}
