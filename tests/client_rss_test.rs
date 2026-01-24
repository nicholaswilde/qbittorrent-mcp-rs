use anyhow::Result;
use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_rss_operations() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    // Add Feed
    Mock::given(method("POST"))
        .and(path("/api/v2/rss/addFeed"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Get Feeds
    Mock::given(method("GET"))
        .and(path("/api/v2/rss/allFeeds"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "test_feed": { "url": "http://example.com/rss", "title": "Test Feed" }
        })))
        .mount(&mock_server)
        .await;

    // Set Rule
    Mock::given(method("POST"))
        .and(path("/api/v2/rss/setRule"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Get Rules
    Mock::given(method("GET"))
        .and(path("/api/v2/rss/allRules"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "test_rule": {
                "enabled": true,
                "must_contain": "linux",
                "must_not_contain": "windows",
                "use_regex": false,
                "episode_filter": "",
                "smart_episode_filter": false,
                "assign_category": "ISO",
                "save_path": "/downloads"
            }
        })))
        .mount(&mock_server)
        .await;

    client
        .add_rss_feed("http://example.com/rss", "test_feed")
        .await?;
    let feeds = client.get_all_rss_feeds().await?;
    assert!(feeds.contains_key("test_feed"));

    client.set_rss_rule("test_rule", "{}").await?;
    let rules = client.get_all_rss_rules().await?;
    assert!(rules.contains_key("test_rule"));

    Ok(())
}
