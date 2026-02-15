use qbittorrent_mcp_rs::app::run_app;
use qbittorrent_mcp_rs::config::AppConfig;
use tempfile::tempdir;
use tokio::sync::oneshot;

#[tokio::test]
async fn test_run_app_stdio() {
    let args = vec![
        "app".to_string(),
        "--polling-interval-ms".to_string(),
        "100".to_string(),
    ];
    let config = AppConfig::load(None, args).unwrap();

    let (tx, rx) = oneshot::channel();
    let _ = tx.send(()); // Send immediately

    let result = run_app(config, Some(rx)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_app_http() {
    let port = 3100;
    let args = vec![
        "app".to_string(),
        "--server-mode".to_string(),
        "http".to_string(),
        "--qbittorrent-port".to_string(),
        port.to_string(),
        "--polling-interval-ms".to_string(),
        "100".to_string(),
    ];
    let config = AppConfig::load(None, args).unwrap();

    let (tx, rx) = oneshot::channel();

    let handle = tokio::spawn(async move {
        let _ = run_app(config, Some(rx)).await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let _ = tx.send(());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
}

#[tokio::test]
async fn test_init_logging_never_rotation_direct() {
    let dir = tempdir().unwrap();
    let args = vec![
        "app".to_string(),
        "--log-file-enable".to_string(),
        "--log-dir".to_string(),
        dir.path().to_str().unwrap().to_string(),
        "--log-rotate".to_string(),
        "never".to_string(),
    ];
    let config = AppConfig::load(None, args).unwrap();
    // We can't easily test direct logging because it uses global state,
    // but run_app is what we want to cover.
    // Let's try one more time with a very short run.
    let (tx, rx) = oneshot::channel();
    let _ = tx.send(());
    let result = run_app(config, Some(rx)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_app_no_instances_error() {
    let config = AppConfig {
        instances: Some(vec![]),
        qbittorrent_host: "".into(),
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

    let result = run_app(config, None).await;
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "No qBittorrent instances configured"
    );
}

#[tokio::test]
async fn test_run_app_https_and_port() {
    let config = AppConfig {
        instances: Some(vec![
            qbittorrent_mcp_rs::config::QBitInstance {
                name: "https_inst".into(),
                host: "https://secure.host".into(),
                port: Some(443),
                username: None,
                password: None,
                no_verify_ssl: Some(true),
            },
            qbittorrent_mcp_rs::config::QBitInstance {
                name: "https_no_port".into(),
                host: "https://secure.host2".into(),
                port: None,
                username: None,
                password: None,
                no_verify_ssl: None,
            },
        ]),
        qbittorrent_host: "localhost".into(),
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

    let (tx, rx) = oneshot::channel();
    let _ = tx.send(());
    let _ = run_app(config, Some(rx)).await;
}
