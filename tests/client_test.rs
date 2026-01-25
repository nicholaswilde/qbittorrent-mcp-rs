use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_login_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Ok."))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new(mock_server.uri(), "admin", "password", false);
    let result = client.login().await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_login_failure() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(403)) // Forbidden
        .mount(&mock_server)
        .await;

    let client = QBitClient::new(mock_server.uri(), "admin", "wrong_password", false);
    let result = client.login().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_torrent_list() {
    let mock_server = MockServer::start().await;

    let mock_torrents = r#"[
        {"hash": "123", "name": "Linux ISO", "size": 1024, "progress": 1.0, "dlspeed": 0, "upspeed": 0, "priority": 0, "num_seeds": 10, "num_leechs": 0, "num_incomplete": 0, "num_complete": 10, "ratio": 1.0, "eta": 0, "state": "uploading", "added_on": 1500000000, "completion_on": 1500000500, "seq_dl": false, "f_l_piece_prio": false, "category": "", "tags": "", "super_seeding": false, "force_start": false}
    ]"#;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_string(mock_torrents))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new(mock_server.uri(), "admin", "password", false);
    let torrents = client
        .get_torrent_list(None, None, None, None, None, None, None)
        .await
        .unwrap();

    assert_eq!(torrents.len(), 1);
    assert_eq!(torrents[0].name, "Linux ISO");
}

#[tokio::test]
async fn test_get_torrents_info() {
    let mock_server = MockServer::start().await;

    let mock_torrents = r#"[
        {"hash": "abc", "name": "Ubuntu", "size": 2048, "progress": 0.5, "dlspeed": 500, "upspeed": 10, "priority": 1, "num_seeds": 5, "num_leechs": 2, "num_incomplete": 2, "num_complete": 5, "ratio": 0.1, "eta": 60, "state": "downloading", "added_on": 1500000000, "completion_on": 0, "seq_dl": true, "f_l_piece_prio": false, "category": "linux", "tags": "os", "super_seeding": false, "force_start": false}
    ]"#;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_string(mock_torrents))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new_no_auth(mock_server.uri(), false);
    let torrents = client.get_torrents_info("abc").await.unwrap();

    assert_eq!(torrents.len(), 1);
    assert_eq!(torrents[0].hash, "abc");
}
