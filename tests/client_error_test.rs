use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_login_wrong_body() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Unexpected body"))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new(mock_server.uri(), "admin", "password", false);
    // Only "Fails." means bad credentials; other 2xx bodies are success.
    assert!(client.login().await.is_ok());
}

#[tokio::test]
async fn test_login_empty_body_is_success() {
    // qBittorrent 5.3 answers a successful login with 200 and an empty body.
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new(mock_server.uri(), "admin", "password", false);
    assert!(client.login().await.is_ok());
}

#[tokio::test]
async fn test_login_wrong_password_body() {
    // qBittorrent answers a wrong password with 200 "Fails.", not an error status.
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Fails."))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new(mock_server.uri(), "admin", "wrong", false);
    let err = client.login().await.unwrap_err().to_string();
    assert!(err.contains("wrong username or password"), "{}", err);
}

#[tokio::test]
async fn test_login_401_is_wrong_password() {
    // qBittorrent 5.x answers a wrong password with 401 Unauthorized.
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Fails."))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new(mock_server.uri(), "admin", "wrong", false);
    let err = client.login().await.unwrap_err().to_string();
    assert!(err.contains("wrong username or password"), "{}", err);
}

#[tokio::test]
async fn test_relogin_failure_is_reported() {
    // An expired session gets 403; if logging in again fails, the caller should see why.
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Fails."))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new(mock_server.uri(), "admin", "wrong", false);
    let err = client
        .get_torrent_list(None, None, None, None, None, None, None)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("wrong username or password"), "{}", err);
}

#[tokio::test]
async fn test_get_torrent_list_error() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new_no_auth(mock_server.uri(), false);
    let result = client
        .get_torrent_list(None, None, None, None, None, None, None)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_add_torrent_error() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/add"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new_no_auth(mock_server.uri(), false);
    let result = client.add_torrent("magnet:...", None, None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pause_torrent_fallback() {
    let mock_server = MockServer::start().await;

    // 1. Initial v5 call fails with 404
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/stop"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    // 2. Fallback v4 call succeeds
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/pause"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new_no_auth(mock_server.uri(), false);
    let result = client.pause_torrents("abc").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_resume_torrent_fallback() {
    let mock_server = MockServer::start().await;

    // 1. Initial v5 call fails with 404
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/start"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    // 2. Fallback v4 call succeeds
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/resume"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new_no_auth(mock_server.uri(), false);
    let result = client.resume_torrents("abc").await;
    assert!(result.is_ok());
}
