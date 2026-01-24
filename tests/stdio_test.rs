use qbittorrent_mcp_rs::client::QBitClient;
use qbittorrent_mcp_rs::server::mcp::McpServer;

#[test]
fn test_stdio_server_instantiation() {
    let client = QBitClient::new_no_auth("http://localhost:8080", false);
    let _server = McpServer::new(client, false);
}
