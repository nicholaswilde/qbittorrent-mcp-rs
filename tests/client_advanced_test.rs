use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_advanced_torrent_control() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    // Mock toggleSequentialDownload
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/toggleSequentialDownload"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.toggle_sequential_download("hash1").await?;

    // Mock toggleFirstLastPiecePrio
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/toggleFirstLastPiecePrio"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.toggle_first_last_piece_priority("hash1").await?;

    // Mock setForceStart
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/setForceStart"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.set_force_start("hash1", true).await?;

    // Mock setSuperSeeding
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/setSuperSeeding"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.set_super_seeding("hash1", true).await?;

    Ok(())
}

#[tokio::test]
async fn test_tracker_management() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    // Mock addTrackers
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/addTrackers"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.add_trackers("hash1", "http://tracker.com").await?;

    // Mock editTracker
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/editTracker"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.edit_tracker("hash1", "old", "new").await?;

    // Mock removeTrackers
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/removeTrackers"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client
        .remove_trackers("hash1", "http://tracker.com")
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_file_folder_management() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    // Mock renameFolder
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/renameFolder"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.rename_folder("hash1", "old", "new").await?;

    // Mock setFilePrio
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/setFilePrio"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.set_file_priority("hash1", "0|1", 7).await?;

    Ok(())
}

#[tokio::test]
async fn test_tag_category_management_advanced() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    // Mock removeCategories
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/removeCategories"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.remove_categories("cat1\ncat2").await?;

    // Mock removeTags
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/removeTags"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.remove_tags("hash1", "tag1,tag2").await?;

    // Mock createTags
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/createTags"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.create_tags("tag1,tag2").await?;

    // Mock deleteTags
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/deleteTags"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.delete_tags("tag1,tag2").await?;

    Ok(())
}

#[tokio::test]
async fn test_rss_management_advanced() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    // Mock moveItem
    Mock::given(method("POST"))
        .and(path("/api/v2/rss/moveItem"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.move_rss_item("old", "new").await?;

    Ok(())
}
