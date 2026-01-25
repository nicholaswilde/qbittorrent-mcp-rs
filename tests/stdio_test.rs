use qbittorrent_mcp_rs::client::QBitClient;
use qbittorrent_mcp_rs::server::mcp::McpServer;

#[test]
fn test_stdio_server_instantiation() {
    let client = QBitClient::new("http://localhost:8080", "admin", "adminadmin", false);
    let mut clients = std::collections::HashMap::new();
    clients.insert("default".to_string(), client);
    let _server = McpServer::new(clients, false);
}
