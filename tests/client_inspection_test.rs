use anyhow::Result;
use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_get_torrent_files() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri());

    let json_response = r#"[
        {
            "index": 0,
            "name": "file.txt",
            "size": 1024,
            "progress": 1.0,
            "priority": 1,
            "is_seed": true,
            "piece_range": [0, 1],
            "availability": 1.0
        }
    ]"#;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/files"))
        .and(query_param("hash", "hash1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(json_response))
        .mount(&mock_server)
        .await;

    let files = client.get_torrent_files("hash1").await?;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "file.txt");

    Ok(())
}

#[tokio::test]
async fn test_get_torrent_properties() -> Result<()> {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri());

    // Minimal JSON for TorrentProperties based on our struct
    let json_response = r#"{
        "save_path": "/downloads",
        "creation_date": 1234567890,
        "piece_size": 1024,
        "comment": "test",
        "total_wasted": 0,
        "total_uploaded": 100,
        "total_downloaded": 200,
        "up_limit": -1,
        "dl_limit": -1,
        "time_elapsed": 10,
        "seeding_time": 5,
        "nb_connections": 5,
        "nb_connections_limit": 50,
        "share_ratio": 0.5,
        "addition_date": 1234567800,
        "completion_date": 1234567890,
        "created_by": "test",
        "dl_speed_avg": 100,
        "dl_speed": 100,
        "eta": 0,
        "last_seen": 1234567890,
        "peers": 2,
        "peers_total": 5,
        "pieces_have": 10,
        "pieces_num": 10,
        "reannounce": 0,
        "seeds": 1,
        "seeds_total": 2,
        "total_size": 200,
        "up_speed_avg": 50,
        "up_speed": 50
    }"#;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/properties"))
        .and(query_param("hash", "hash1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(json_response))
        .mount(&mock_server)
        .await;

    let props = client.get_torrent_properties("hash1").await?;
    assert_eq!(props.save_path, "/downloads");

    Ok(())
}
