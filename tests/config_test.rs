use qbittorrent_mcp_rs::config::AppConfig;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_load_config_defaults() {
    let config = AppConfig::load(None, vec![]).expect("Failed to load default config");
    assert_eq!(config.qbittorrent_host, "localhost");
    assert_eq!(config.qbittorrent_port, 8080);
    assert_eq!(config.server_mode, "stdio");
}

#[test]
fn test_load_config_from_toml() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config.toml");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "qbittorrent_host = 'test_host'").unwrap();
    writeln!(file, "qbittorrent_port = 1234").unwrap();

    let config = AppConfig::load(Some(file_path.to_str().unwrap().to_string()), vec![])
        .expect("Failed to load config from TOML");

    assert_eq!(config.qbittorrent_host, "test_host");
    assert_eq!(config.qbittorrent_port, 1234);
}

#[test]
fn test_load_config_from_json() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config.json");
    let mut file = File::create(&file_path).unwrap();
    writeln!(
        file,
        r#"{{
        "qbittorrent_host": "json_host",
        "qbittorrent_port": 5678
    }}"#
    )
    .unwrap();

    let config = AppConfig::load(Some(file_path.to_str().unwrap().to_string()), vec![])
        .expect("Failed to load config from JSON");

    assert_eq!(config.qbittorrent_host, "json_host");
    assert_eq!(config.qbittorrent_port, 5678);
}

#[test]
fn test_cli_overrides() {
    // Simulate CLI args
    let args = vec![
        "app".to_string(),
        "--qbittorrent-host".to_string(),
        "cli_host".to_string(),
        "--qbittorrent-port".to_string(),
        "9999".to_string(),
    ];
    let config = AppConfig::load(None, args).expect("Failed to load config with CLI args");

    assert_eq!(config.qbittorrent_host, "cli_host");
    assert_eq!(config.qbittorrent_port, 9999);
}
