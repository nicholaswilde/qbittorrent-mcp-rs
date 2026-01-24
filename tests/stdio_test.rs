use qbittorrent_mcp_rs::client::QBitClient;
use qbittorrent_mcp_rs::server::stdio::StdioServer;

#[test]
fn test_stdio_server_instantiation() {
    let client = QBitClient::new_no_auth("http://localhost:8080");
    let _server = StdioServer::new(client, false);
}
