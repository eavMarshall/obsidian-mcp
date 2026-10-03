use obsidian_mcp::config::AppConfig;

#[test]
fn test_parse_valid_config() {
    let yaml_str = r#"
server:
  port: 8080
vaults:
  - id: "work"
    path: "/vaults/work"
    read_only: false
  - id: "personal"
    path: "/vaults/personal"
    read_only: true
"#;

    // This will fail because AppConfig does not exist yet!
    let config: AppConfig = serde_yaml::from_str(yaml_str).expect("Failed to parse YAML");

    assert_eq!(config.server.port, 8080);
    assert_eq!(config.vaults.len(), 2);
    
    assert_eq!(config.vaults[0].id, "work");
    assert_eq!(config.vaults[0].read_only, false);

    assert_eq!(config.vaults[1].id, "personal");
    assert_eq!(config.vaults[1].read_only, true);
}
