use anyhow::Result;
use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_set_torrent_share_limits() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/setShareLimits"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client
        .set_torrent_share_limits("hash1|hash2", 2.0, 120)
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_set_torrent_speed_limits() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/setDownloadLimit"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/setUploadLimit"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client
        .set_torrent_download_limit("hash1", 1024 * 1024)
        .await?;
    client.set_torrent_upload_limit("hash1", 512 * 1024).await?;

    Ok(())
}
