use qbittorrent_mcp_rs::client::QBitClient;
use qbittorrent_mcp_rs::server::mcp::McpServer;

#[test]
fn test_stdio_server_instantiation() {
    let client = QBitClient::new("http://localhost:8080", "admin", "adminadmin", false);
    let mut clients = std::collections::HashMap::new();
    clients.insert("default".to_string(), client);
    let _server = McpServer::new(clients, false);
}

#[test]
fn test_server_shutdown_status() {
    let mut clients = std::collections::HashMap::new();
    let client = QBitClient::new("http://localhost:8080", "admin", "adminadmin", false);
    clients.insert("default".to_string(), client);
    let server = McpServer::new(clients, false);
    assert!(server.is_running());
    server.shutdown();
    assert!(!server.is_running());
}
