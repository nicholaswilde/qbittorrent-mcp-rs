use anyhow::Result;
use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_log_operations() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    // Main Log
    Mock::given(method("GET"))
        .and(path("/api/v2/log/main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": 1,
                "message": "Test message",
                "timestamp": 1234567890,
                "type": 2
            }
        ])))
        .mount(&mock_server)
        .await;

    // Peer Log
    Mock::given(method("GET"))
        .and(path("/api/v2/log/peers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": 1,
                "ip": "1.2.3.4",
                "timestamp": 1234567890,
                "blocked": false,
                "reason": ""
            }
        ])))
        .mount(&mock_server)
        .await;

    let main_logs = client.get_main_log(true, true, true, true, None).await?;
    assert_eq!(main_logs.len(), 1);
    assert_eq!(main_logs[0].message, "Test message");

    let peer_logs = client.get_peer_log(None).await?;
    assert_eq!(peer_logs.len(), 1);
    assert_eq!(peer_logs[0].ip, "1.2.3.4");

    Ok(())
}
