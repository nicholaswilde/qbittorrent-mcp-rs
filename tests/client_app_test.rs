use anyhow::Result;
use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_app_preferences() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri());

    // Get Preferences
    Mock::given(method("GET"))
        .and(path("/api/v2/app/preferences"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "save_path": "/downloads"
        })))
        .mount(&mock_server)
        .await;

    // Set Preferences
    Mock::given(method("POST"))
        .and(path("/api/v2/app/setPreferences"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let prefs = client.get_app_preferences().await?;
    assert_eq!(prefs["save_path"], "/downloads");

    client
        .set_app_preferences(&serde_json::json!({"save_path": "/new/path"}))
        .await?;

    Ok(())
}
