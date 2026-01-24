use anyhow::Result;
use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_get_global_transfer_info() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    let json_response = r#"{
        "dl_info_speed": 1024,
        "dl_info_data": 2048,
        "up_info_speed": 512,
        "up_info_data": 1024,
        "dl_rate_limit": 0,
        "up_rate_limit": 0,
        "dht_nodes": 100,
        "connection_status": "connected"
    }"#;

    Mock::given(method("GET"))
        .and(path("/api/v2/transfer/info"))
        .respond_with(ResponseTemplate::new(200).set_body_string(json_response))
        .mount(&mock_server)
        .await;

    let info = client.get_global_transfer_info().await?;
    assert_eq!(info.dl_info_speed, 1024);
    assert_eq!(info.connection_status, "connected");

    Ok(())
}

#[tokio::test]
async fn test_set_transfer_limits() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    Mock::given(method("POST"))
        .and(path("/api/v2/transfer/setDownloadLimit"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/transfer/setUploadLimit"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.set_download_limit(5000).await?;
    client.set_upload_limit(2000).await?;

    Ok(())
}
