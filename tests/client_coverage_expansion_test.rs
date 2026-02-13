use qbittorrent_mcp_rs::client::QBitClient;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_login_base_url_slash() {
    let mock_server = MockServer::start().await;
    let base_url = format!("{}/", mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Ok."))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new(base_url, "admin", "password", false);
    let result = client.login().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_torrent_list_all_params() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .and(query_param("filter", "all"))
        .and(query_param("category", "cat1"))
        .and(query_param("tag", "tag1"))
        .and(query_param("sort", "name"))
        .and(query_param("reverse", "true"))
        .and(query_param("limit", "10"))
        .and(query_param("offset", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new_no_auth(mock_server.uri(), false);
    let result = client
        .get_torrent_list(
            Some("all"),
            Some("cat1"),
            Some("tag1"),
            Some("name"),
            Some(true),
            Some(10),
            Some(5),
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_torrents_info_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new_no_auth(mock_server.uri(), false);
    let result = client.get_torrents_info("abc").await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("500 Internal Server Error")
    );
}

#[tokio::test]
async fn test_pause_torrents_error_paths() {
    let mock_server = MockServer::start().await;

    // Case 1: stop fails with 500
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/stop"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new_no_auth(mock_server.uri(), false);
    let result = client.pause_torrents("abc").await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Failed to stop/pause torrents: 500")
    );

    // Case 2: stop fails with 404, pause fails with 500
    let mock_server2 = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/stop"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server2)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/pause"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server2)
        .await;

    let client2 = QBitClient::new_no_auth(mock_server2.uri(), false);
    let result2 = client2.pause_torrents("abc").await;
    assert!(result2.is_err());
    assert!(
        result2
            .unwrap_err()
            .to_string()
            .contains("Failed to pause torrents: 500")
    );
}

#[tokio::test]
async fn test_resume_torrents_error_paths() {
    let mock_server = MockServer::start().await;

    // Case 1: start fails with 500
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/start"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new_no_auth(mock_server.uri(), false);
    let result = client.resume_torrents("abc").await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Failed to start/resume torrents: 500")
    );

    // Case 2: start fails with 404, resume fails with 500
    let mock_server2 = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/start"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server2)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/resume"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server2)
        .await;

    let client2 = QBitClient::new_no_auth(mock_server2.uri(), false);
    let result2 = client2.resume_torrents("abc").await;
    assert!(result2.is_err());
    assert!(
        result2
            .unwrap_err()
            .to_string()
            .contains("Failed to resume torrents: 500")
    );
}

#[tokio::test]
async fn test_generic_error_paths() {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    let endpoints = vec![
        ("/api/v2/torrents/delete", "POST"),
        ("/api/v2/torrents/reannounce", "POST"),
        ("/api/v2/torrents/recheck", "POST"),
        ("/api/v2/torrents/files", "GET"),
        ("/api/v2/torrents/properties", "GET"),
        ("/api/v2/torrents/trackers", "GET"),
        ("/api/v2/transfer/info", "GET"),
        ("/api/v2/transfer/setDownloadLimit", "POST"),
        ("/api/v2/transfer/setUploadLimit", "POST"),
        ("/api/v2/transfer/toggleSpeedLimitsMode", "POST"),
        ("/api/v2/transfer/speedLimitsMode", "GET"),
        ("/api/v2/search/start", "POST"),
        ("/api/v2/search/results", "POST"),
        ("/api/v2/search/stop", "POST"),
        ("/api/v2/search/delete", "POST"),
        ("/api/v2/torrents/categories", "GET"),
        ("/api/v2/torrents/createCategory", "POST"),
        ("/api/v2/torrents/setCategory", "POST"),
        ("/api/v2/torrents/addTags", "POST"),
        ("/api/v2/search/plugins", "GET"),
        ("/api/v2/search/installPlugin", "POST"),
        ("/api/v2/search/uninstallPlugin", "POST"),
        ("/api/v2/search/enablePlugin", "POST"),
        ("/api/v2/search/updatePlugins", "POST"),
        ("/api/v2/rss/addFeed", "POST"),
        ("/api/v2/rss/removeItem", "POST"),
        ("/api/v2/rss/setRule", "POST"),
        ("/api/v2/app/preferences", "GET"),
        ("/api/v2/app/setPreferences", "POST"),
        ("/api/v2/app/version", "GET"),
        ("/api/v2/app/buildInfo", "GET"),
        ("/api/v2/app/shutdown", "POST"),
        ("/api/v2/log/main", "GET"),
        ("/api/v2/log/peers", "GET"),
        ("/api/v2/transfer/banPeers", "POST"),
        ("/api/v2/torrents/renameFile", "POST"),
        ("/api/v2/torrents/setShareLimits", "POST"),
        ("/api/v2/torrents/setDownloadLimit", "POST"),
        ("/api/v2/torrents/setUploadLimit", "POST"),
        ("/api/v2/sync/maindata", "GET"),
        ("/api/v2/torrents/toggleSequentialDownload", "POST"),
        ("/api/v2/torrents/toggleFirstLastPiecePrio", "POST"),
        ("/api/v2/torrents/setForceStart", "POST"),
        ("/api/v2/torrents/setSuperSeeding", "POST"),
        ("/api/v2/torrents/addTrackers", "POST"),
        ("/api/v2/torrents/editTracker", "POST"),
        ("/api/v2/torrents/removeTrackers", "POST"),
        ("/api/v2/torrents/renameFolder", "POST"),
        ("/api/v2/torrents/setFilePrio", "POST"),
        ("/api/v2/torrents/removeCategories", "POST"),
        ("/api/v2/torrents/removeTags", "POST"),
        ("/api/v2/torrents/createTags", "POST"),
        ("/api/v2/torrents/deleteTags", "POST"),
        ("/api/v2/rss/moveItem", "POST"),
    ];

    for (path_str, method_str) in endpoints {
        Mock::given(method(method_str))
            .and(path(path_str))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;
    }

    // Now call each method and expect error
    assert!(client.delete_torrents("abc", true).await.is_err());
    assert!(client.reannounce_torrents("abc").await.is_err());
    assert!(client.recheck_torrents("abc").await.is_err());
    assert!(client.get_torrent_files("abc").await.is_err());
    assert!(client.get_torrent_properties("abc").await.is_err());
    assert!(client.get_torrent_trackers("abc").await.is_err());
    assert!(client.get_global_transfer_info().await.is_err());
    assert!(client.set_download_limit(100).await.is_err());
    assert!(client.set_upload_limit(100).await.is_err());
    assert!(client.toggle_alternative_speed_limits().await.is_err());
    assert!(client.get_speed_limits_mode().await.is_err());
    assert!(client.start_search("pattern", None).await.is_err());
    assert!(client.get_search_results(1, None, None).await.is_err());
    assert!(client.stop_search(1).await.is_err());
    assert!(client.delete_search(1).await.is_err());
    assert!(client.get_categories().await.is_err());
    assert!(client.create_category("cat", "path").await.is_err());
    assert!(client.set_category("abc", "cat").await.is_err());
    assert!(client.add_tags("abc", "tag").await.is_err());
    assert!(client.get_search_plugins().await.is_err());
    assert!(client.install_search_plugin("url").await.is_err());
    assert!(client.uninstall_search_plugin("name").await.is_err());
    assert!(client.enable_search_plugin("name", true).await.is_err());
    assert!(client.update_search_plugins().await.is_err());
    assert!(client.add_rss_feed("url", "path").await.is_err());
    assert!(client.remove_rss_item("path").await.is_err());
    assert!(client.set_rss_rule("name", "def").await.is_err());
    assert!(client.get_app_preferences().await.is_err());
    assert!(
        client
            .set_app_preferences(&serde_json::json!({}))
            .await
            .is_err()
    );
    assert!(client.get_app_version().await.is_err());
    assert!(client.get_build_info().await.is_err());
    assert!(client.shutdown_app().await.is_err());
    assert!(
        client
            .get_main_log(true, true, true, true, None)
            .await
            .is_err()
    );
    assert!(client.get_peer_log(None).await.is_err());
    assert!(client.ban_peers("peers").await.is_err());
    assert!(client.rename_file("abc", "old", "new").await.is_err());
    assert!(
        client
            .set_torrent_share_limits("abc", 1.0, 10, None)
            .await
            .is_err()
    );
    assert!(client.set_torrent_download_limit("abc", 100).await.is_err());
    assert!(client.set_torrent_upload_limit("abc", 100).await.is_err());
    assert!(client.get_main_data(1).await.is_err());
    assert!(client.toggle_sequential_download("abc").await.is_err());
    assert!(
        client
            .toggle_first_last_piece_priority("abc")
            .await
            .is_err()
    );
    assert!(client.set_force_start("abc", true).await.is_err());
    assert!(client.set_super_seeding("abc", true).await.is_err());
    assert!(client.add_trackers("abc", "url").await.is_err());
    assert!(client.edit_tracker("abc", "old", "new").await.is_err());
    assert!(client.remove_trackers("abc", "url").await.is_err());
    assert!(client.rename_folder("abc", "old", "new").await.is_err());
    assert!(client.set_file_priority("abc", "1", 1).await.is_err());
    assert!(client.remove_categories("cat").await.is_err());
    assert!(client.remove_tags("abc", "tag").await.is_err());
    assert!(client.create_tags("tag").await.is_err());
    assert!(client.delete_tags("tag").await.is_err());
    assert!(client.move_rss_item("old", "new").await.is_err());
}

#[tokio::test]
async fn test_rss_feeds_rules_error_paths() {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    // Feeds fallback error
    Mock::given(method("GET"))
        .and(path("/api/v2/rss/items"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/rss/allFeeds"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;
    assert!(client.get_all_rss_feeds().await.is_err());

    // Rules fallback error
    let mock_server2 = MockServer::start().await;
    let client2 = QBitClient::new_no_auth(mock_server2.uri(), false);
    Mock::given(method("GET"))
        .and(path("/api/v2/rss/rules"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server2)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/rss/allRules"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server2)
        .await;
    assert!(client2.get_all_rss_rules().await.is_err());
}

#[tokio::test]
async fn test_client_parameter_coverage() {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);

    // start_search with category
    Mock::given(method("POST"))
        .and(path("/api/v2/search/start"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": 1})))
        .mount(&mock_server)
        .await;
    assert!(client.start_search("pattern", Some("cat")).await.is_ok());

    // get_search_results with limit/offset
    Mock::given(method("POST"))
        .and(path("/api/v2/search/results"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"results": [], "total": 0, "status": "stopped"})),
        )
        .mount(&mock_server)
        .await;
    assert!(
        client
            .get_search_results(1, Some(10), Some(5))
            .await
            .is_ok()
    );

    // get_main_log with last_id
    Mock::given(method("GET"))
        .and(path("/api/v2/log/main"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;
    assert!(
        client
            .get_main_log(true, true, true, true, Some(1))
            .await
            .is_ok()
    );

    // get_peer_log with last_id
    Mock::given(method("GET"))
        .and(path("/api/v2/log/peers"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;
    assert!(client.get_peer_log(Some(1)).await.is_ok());

    // set_torrent_share_limits with inactive
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/setShareLimits"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;
    assert!(
        client
            .set_torrent_share_limits("abc", 1.0, 10, Some(5))
            .await
            .is_ok()
    );
}
