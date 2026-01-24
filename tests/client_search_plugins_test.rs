use anyhow::Result;
use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_search_plugins() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    // Get Plugins
    Mock::given(method("GET"))
        .and(path("/api/v2/search/plugins"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "name": "legit_torrents",
                "fullName": "Legit Torrents",
                "version": "1.0",
                "url": "http://legittorrents.info",
                "supported_categories": ["All"],
                "enabled": true
            }
        ])))
        .mount(&mock_server)
        .await;

    // Install Plugin
    Mock::given(method("POST"))
        .and(path("/api/v2/search/installPlugin"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Uninstall Plugin
    Mock::given(method("POST"))
        .and(path("/api/v2/search/uninstallPlugin"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Enable Plugin
    Mock::given(method("POST"))
        .and(path("/api/v2/search/enablePlugin"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Update Plugins
    Mock::given(method("POST"))
        .and(path("/api/v2/search/updatePlugins"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let plugins = client.get_search_plugins().await?;
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].name, "legit_torrents");

    client
        .install_search_plugin("http://example.com/plugin.py")
        .await?;
    client.enable_search_plugin("legit_torrents", false).await?;
    client.update_search_plugins().await?;
    client.uninstall_search_plugin("legit_torrents").await?;

    Ok(())
}
