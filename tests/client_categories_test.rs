use anyhow::Result;
use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_category_operations() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    // Get Categories
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/categories"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Movies": { "name": "Movies", "savePath": "/downloads/movies" }
        })))
        .mount(&mock_server)
        .await;

    // Create Category
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/createCategory"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Set Category
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/setCategory"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let categories = client.get_categories().await?;
    assert!(categories.contains_key("Movies"));

    client.create_category("Music", "/downloads/music").await?;
    client.set_category("hash1", "Music").await?;

    Ok(())
}

#[tokio::test]
async fn test_tags_operations() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/addTags"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    client.add_tags("hash1", "tag1,tag2").await?;

    Ok(())
}
