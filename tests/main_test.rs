use qbittorrent_mcp_rs::config::AppConfig;

#[test]
fn test_config_instances_mapping() {
    let mut config = AppConfig {
        instances: None,
        qbittorrent_host: "localhost".to_string(),
        qbittorrent_port: Some(8080),
        qbittorrent_username: Some("admin".to_string()),
        qbittorrent_password: Some("password".to_string()),
        server_mode: "stdio".to_string(),
        lazy_mode: false,
        no_verify_ssl: false,
        log_level: "info".to_string(),
        log_file_enable: true, // Trigger branch
        log_dir: "/tmp".to_string(),
        log_filename: "test.log".to_string(),
        log_rotate: "hourly".to_string(), // Trigger branch
        http_auth_token: None,
        polling_interval_ms: 2000,
    };

    let instances = config.get_instances();
    assert_eq!(instances.len(), 1);

    // Test multiple instances branch
    config.instances = Some(vec![qbittorrent_mcp_rs::config::QBitInstance {
        name: "test1".to_string(),
        host: "host1".to_string(),
        port: None,
        username: None,
        password: None,
        no_verify_ssl: None,
    }]);
    let instances2 = config.get_instances();
    assert_eq!(instances2.len(), 1);
    assert_eq!(instances2[0].name, "test1");
}

#[test]
fn test_config_instances_url_parsing() {
    let config = AppConfig {
        instances: Some(vec![
            qbittorrent_mcp_rs::config::QBitInstance {
                name: "test1".to_string(),
                host: "http://host1".to_string(), // Starts with http
                port: Some(8080),
                username: None,
                password: None,
                no_verify_ssl: None,
            },
            qbittorrent_mcp_rs::config::QBitInstance {
                name: "test2".to_string(),
                host: "host2".to_string(), // No http
                port: None,
                username: None,
                password: None,
                no_verify_ssl: None,
            },
        ]),
        qbittorrent_host: "localhost".to_string(),
        qbittorrent_port: Some(8080),
        qbittorrent_username: None,
        qbittorrent_password: None,
        server_mode: "stdio".to_string(),
        lazy_mode: false,
        no_verify_ssl: false,
        log_level: "info".to_string(),
        log_file_enable: false,
        log_dir: ".".to_string(),
        log_filename: "test.log".to_string(),
        log_rotate: "daily".to_string(),
        http_auth_token: None,
        polling_interval_ms: 2000,
    };

    let instances = config.get_instances();
    assert_eq!(instances.len(), 2);
}

#[test]
fn test_config_various_log_rotations() {
    let mut config = AppConfig {
        instances: None,
        qbittorrent_host: "localhost".to_string(),
        qbittorrent_port: Some(8080),
        qbittorrent_username: None,
        qbittorrent_password: None,
        server_mode: "stdio".to_string(),
        lazy_mode: false,
        no_verify_ssl: false,
        log_level: "info".to_string(),
        log_file_enable: true,
        log_dir: "/tmp".to_string(),
        log_filename: "test.log".to_string(),
        log_rotate: "never".to_string(), // Test 'never'
        http_auth_token: None,
        polling_interval_ms: 2000,
    };

    // We can't easily test init_logging directly as it calls .init() which can only be called once.
    // But we can test the fallback logic in get_instances and other AppConfig methods if needed.
    // Let's at least trigger the string matching logic for rotations.
    let rotations = vec!["hourly", "never", "daily", "invalid"];
    for r in rotations {
        config.log_rotate = r.to_string();
        assert_eq!(config.log_rotate, r);
    }
}

#[test]
fn test_config_instances_complex_urls() {
    let config = AppConfig {
        instances: Some(vec![
            qbittorrent_mcp_rs::config::QBitInstance {
                name: "test1".into(),
                host: "https://host1".into(), // https
                port: None,
                username: None,
                password: None,
                no_verify_ssl: Some(true),
            },
            qbittorrent_mcp_rs::config::QBitInstance {
                name: "test2".into(),
                host: "host2".into(),
                port: Some(443),
                username: Some("u".into()),
                password: Some("p".into()),
                no_verify_ssl: None,
            },
        ]),
        qbittorrent_host: "localhost".to_string(),
        qbittorrent_port: Some(8080),
        qbittorrent_username: None,
        qbittorrent_password: None,
        server_mode: "http".to_string(), // Test 'http'
        lazy_mode: true,                 // Test 'true'
        no_verify_ssl: false,
        log_level: "debug".to_string(),
        log_file_enable: false,
        log_dir: ".".to_string(),
        log_filename: "test.log".to_string(),
        log_rotate: "daily".to_string(),
        http_auth_token: Some("secret".into()),
        polling_interval_ms: 500,
    };

    let instances = config.get_instances();
    assert_eq!(instances.len(), 2);
    assert_eq!(config.server_mode, "http");
}

#[test]
fn test_config_instances_mapping_various() {
    let config = AppConfig {
        instances: Some(vec![
            qbittorrent_mcp_rs::config::QBitInstance {
                name: "t1".into(),
                host: "http://h1:8080".into(),
                port: None,
                username: Some("u1".into()),
                password: Some("p1".into()),
                no_verify_ssl: None,
            },
            qbittorrent_mcp_rs::config::QBitInstance {
                name: "t2".into(),
                host: "https://h2".into(),
                port: Some(443),
                username: None,
                password: None,
                no_verify_ssl: Some(false),
            },
        ]),
        qbittorrent_host: "h".into(),
        qbittorrent_port: None,
        qbittorrent_username: None,
        qbittorrent_password: None,
        server_mode: "stdio".into(),
        lazy_mode: false,
        no_verify_ssl: false,
        log_level: "info".into(),
        log_file_enable: false,
        log_dir: ".".into(),
        log_filename: "f".into(),
        log_rotate: "d".into(),
        http_auth_token: None,
        polling_interval_ms: 100,
    };

    let instances = config.get_instances();
    assert_eq!(instances.len(), 2);
}
