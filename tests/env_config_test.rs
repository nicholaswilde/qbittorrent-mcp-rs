use qbittorrent_mcp_rs::config::AppConfig;

// Own test binary, so setting process env vars cannot race other config tests.
#[test]
fn env_vars_load_and_keep_strings() {
    unsafe {
        // Short names from the README.
        std::env::set_var("QBITTORRENT_HOST", "http://nas:9777");
        std::env::set_var("QBITTORRENT_USERNAME", "admin");
        std::env::set_var("QBITTORRENT_PASSWORD", "007123");
        std::env::set_var("QBITTORRENT_HTTP_PORT", "3001");
        // Read through the generic env source: must stay a string.
        std::env::set_var("QBITTORRENT_HTTP_AUTH_TOKEN", "0123");
        std::env::set_var("QBITTORRENT_LOG_LEVEL", "debug");
    }
    let config = AppConfig::load(None, vec![]).unwrap();
    assert_eq!(config.qbittorrent_host, "http://nas:9777");
    assert_eq!(config.qbittorrent_username.as_deref(), Some("admin"));
    assert_eq!(config.qbittorrent_password.as_deref(), Some("007123"));
    assert_eq!(config.http_port, 3001);
    assert_eq!(config.http_auth_token.as_deref(), Some("0123"));
    // The CLI parser used to inject "info" and override the env var.
    assert_eq!(config.log_level, "debug");
}
