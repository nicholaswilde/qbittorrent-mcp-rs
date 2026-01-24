use anyhow::Result;
use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_add_torrent() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/add"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Ok."))
        .mount(&mock_server)
        .await;

    // Test minimal arguments
    client
        .add_torrent("magnet:?xt=urn:btih:test", None, None)
        .await?;

    // Test with optional arguments
    client
        .add_torrent(
            "magnet:?xt=urn:btih:test2",
            Some("/downloads"),
            Some("linux"),
        )
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_manage_torrents() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/pause"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/resume"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/delete"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.pause_torrents("hash1|hash2").await?;
    client.resume_torrents("hash1").await?;
    client.delete_torrents("hash3", true).await?;

    Ok(())
}
