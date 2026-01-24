use anyhow::Result;
use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_search_lifecycle() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri());

    // 1. Start Search
    Mock::given(method("POST"))
        .and(path("/api/v2/search/start"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "id": 123 })))
        .mount(&mock_server)
        .await;

    // 2. Get Results
    let results_json = serde_json::json!({
        "status": "Running",
        "total": 1,
        "results": [
            {
                "fileName": "Ubuntu.iso",
                "fileUrl": "magnet:...",
                "fileSize": 1000,
                "nbSeeders": 10,
                "nbLeechers": 5,
                "siteUrl": "http://example.com"
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/api/v2/search/results"))
        .respond_with(ResponseTemplate::new(200).set_body_json(results_json))
        .mount(&mock_server)
        .await;

    // 3. Stop Search
    Mock::given(method("POST"))
        .and(path("/api/v2/search/stop"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // 4. Delete Search
    Mock::given(method("POST"))
        .and(path("/api/v2/search/delete"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let id = client.start_search("ubuntu", None).await?;
    assert_eq!(id, 123);

    let response = client.get_search_results(id, None, None).await?;
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].file_name, "Ubuntu.iso");

    client.stop_search(id).await?;
    client.delete_search(id).await?;

    Ok(())
}
