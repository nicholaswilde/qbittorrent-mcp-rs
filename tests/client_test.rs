use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{body_string, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_login_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .and(body_string("username=admin&password=password"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Ok."))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new(mock_server.uri(), "admin", "password");
    let result = client.login().await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_login_failure() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(401)) // Fails to login
        .mount(&mock_server)
        .await;

    let client = QBitClient::new(mock_server.uri(), "admin", "wrong_password");
    let result = client.login().await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_torrent_list() {
    let mock_server = MockServer::start().await;

    let json_response = r#"[
        {
            "hash": "8c4a5c5b5d5e5f5g5h5i5j5k5l5m5n5o5p5q5r5s",
            "name": "Ubuntu Linux",
            "size": 2000000000,
            "progress": 0.5,
            "dlspeed": 5000,
            "upspeed": 1000,
            "priority": 1,
            "num_seeds": 10,
            "num_leechs": 5,
            "num_incomplete": 5,
            "num_complete": 10,
            "ratio": 1.5,
            "eta": 3600,
            "state": "downloading",
            "seq_dl": false,
            "f_l_piece_prio": false,
            "category": "ISOs",
            "tags": "linux,iso",
            "super_seeding": false,
            "force_start": false
        }
    ]"#;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_string(json_response))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new(mock_server.uri(), "admin", "password");
    // login not strictly needed if we don't mock the cookie requirement for this test,
    // but in real app we need login.

    let torrents = client
        .get_torrent_list()
        .await
        .expect("Failed to get torrent list");

    assert_eq!(torrents.len(), 1);
    assert_eq!(torrents[0].name, "Ubuntu Linux");
}
