use qbittorrent_mcp_rs::client::QBitClient;
use qbittorrent_mcp_rs::server::mcp::{JsonRpcRequest, McpServer};
use serde_json::json;
use std::collections::HashMap;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup_mock_server() -> (MockServer, McpServer) {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);
    let mut clients = HashMap::new();
    clients.insert("default".to_string(), client);
    let server = McpServer::new(clients, false);
    (mock_server, server)
}

#[tokio::test]
async fn test_handle_wait_for_torrent_status_success() {
    let (mock_server, server) = setup_mock_server().await;

    // Mock first call as downloading, second as uploading
    let mock_torrents_1 = r#"[
        {"hash": "abc", "name": "Ubuntu", "size": 2048, "progress": 0.5, "dlspeed": 500, "upspeed": 10, "priority": 1, "num_seeds": 5, "num_leechs": 2, "num_incomplete": 2, "num_complete": 5, "ratio": 0.1, "eta": 60, "state": "downloading", "added_on": 1500000000, "completion_on": 0, "seq_dl": true, "f_l_piece_prio": false, "category": "linux", "tags": "os", "super_seeding": false, "force_start": false}
    ]"#;
    let mock_torrents_2 = r#"[
        {"hash": "abc", "name": "Ubuntu", "size": 2048, "progress": 1.0, "dlspeed": 0, "upspeed": 10, "priority": 1, "num_seeds": 5, "num_leechs": 2, "num_incomplete": 2, "num_complete": 5, "ratio": 0.1, "eta": 0, "state": "uploading", "added_on": 1500000000, "completion_on": 1500001000, "seq_dl": true, "f_l_piece_prio": false, "category": "linux", "tags": "os", "super_seeding": false, "force_start": false}
    ]"#;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .and(query_param("hashes", "abc"))
        .respond_with(ResponseTemplate::new(200).set_body_string(mock_torrents_1))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .and(query_param("hashes", "abc"))
        .respond_with(ResponseTemplate::new(200).set_body_string(mock_torrents_2))
        .mount(&mock_server)
        .await;

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "wait_for_torrent_status",
            "arguments": {
                "hash": "abc",
                "target_status": "uploading",
                "timeout_seconds": 5
            }
        })),
        id: Some(json!(1)),
    };

    let resp = server.handle_request(req).await.unwrap();
    let text = resp["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Torrent reached target status: uploading"));
}

#[tokio::test]
async fn test_handle_wait_for_torrent_status_timeout() {
    let (mock_server, server) = setup_mock_server().await;

    let mock_torrents = r#"[
        {"hash": "abc", "name": "Ubuntu", "size": 2048, "progress": 0.5, "dlspeed": 500, "upspeed": 10, "priority": 1, "num_seeds": 5, "num_leechs": 2, "num_incomplete": 2, "num_complete": 5, "ratio": 0.1, "eta": 60, "state": "downloading", "added_on": 1500000000, "completion_on": 0, "seq_dl": true, "f_l_piece_prio": false, "category": "linux", "tags": "os", "super_seeding": false, "force_start": false}
    ]"#;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_string(mock_torrents))
        .mount(&mock_server)
        .await;

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "wait_for_torrent_status",
            "arguments": {
                "hash": "abc",
                "target_status": "uploading",
                "timeout_seconds": 1
            }
        })),
        id: Some(json!(1)),
    };

    let resp = server.handle_request(req).await.unwrap();
    let text = resp["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Timed out waiting for status uploading"));
    assert_eq!(resp["isError"], true);
}

#[tokio::test]
async fn test_handle_cleanup_completed_ratio() {
    let (mock_server, server) = setup_mock_server().await;

    let mock_torrents = r#"[
        {"hash": "h1", "name": "T1", "size": 1024, "progress": 1.0, "dlspeed": 0, "upspeed": 0, "priority": 0, "num_seeds": 1, "num_leechs": 0, "num_incomplete": 0, "num_complete": 1, "ratio": 2.5, "eta": 0, "state": "uploading", "added_on": 100, "completion_on": 200, "seq_dl": false, "f_l_piece_prio": false, "category": "", "tags": "", "super_seeding": false, "force_start": false},
        {"hash": "h2", "name": "T2", "size": 1024, "progress": 1.0, "dlspeed": 0, "upspeed": 0, "priority": 0, "num_seeds": 1, "num_leechs": 0, "num_incomplete": 0, "num_complete": 1, "ratio": 0.5, "eta": 0, "state": "uploading", "added_on": 100, "completion_on": 200, "seq_dl": false, "f_l_piece_prio": false, "category": "", "tags": "", "super_seeding": false, "force_start": false}
    ]"#;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .and(query_param("filter", "completed"))
        .respond_with(ResponseTemplate::new(200).set_body_string(mock_torrents))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/delete"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "cleanup_completed",
            "arguments": {
                "min_ratio": 2.0,
                "delete_files": false
            }
        })),
        id: Some(json!(1)),
    };

    let resp = server.handle_request(req).await.unwrap();
    let text = resp["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Successfully cleaned up 1 torrents"));
}

#[tokio::test]
async fn test_handle_cleanup_completed_none() {
    let (mock_server, server) = setup_mock_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "cleanup_completed",
            "arguments": {
                "delete_files": false
            }
        })),
        id: Some(json!(1)),
    };

    let resp = server.handle_request(req).await.unwrap();
    let text = resp["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("No torrents matched the cleanup criteria"));
}

#[tokio::test]
async fn test_handle_mass_rename() {
    let (mock_server, server) = setup_mock_server().await;

    let mock_files = r#"[
        {"index": 0, "name": "movie.mp4", "size": 1000, "progress": 1.0, "priority": 1, "availability": 1.0},
        {"index": 1, "name": "sample.mkv", "size": 100, "progress": 1.0, "priority": 1, "availability": 1.0},
        {"index": 2, "name": "readme.txt", "size": 10, "progress": 1.0, "priority": 1, "availability": 1.0}
    ]"#;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/files"))
        .respond_with(ResponseTemplate::new(200).set_body_string(mock_files))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/renameFile"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "mass_rename",
            "arguments": {
                "hash": "abc",
                "pattern": "(.*)\\.(mp4|mkv)",
                "replacement": "video_$1.$2"
            }
        })),
        id: Some(json!(1)),
    };

    let resp = server.handle_request(req).await.unwrap();
    let text = resp["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Successfully renamed 2 files"));
}

#[tokio::test]
async fn test_handle_find_duplicates() {
    let (mock_server, server) = setup_mock_server().await;

    let mock_torrents = r#"[
        {"hash": "h1", "name": "Ubuntu", "size": 1024, "progress": 1.0, "dlspeed": 0, "upspeed": 0, "priority": 0, "num_seeds": 1, "num_leechs": 0, "num_incomplete": 0, "num_complete": 1, "ratio": 1.0, "eta": 0, "state": "uploading", "added_on": 100, "completion_on": 200, "seq_dl": false, "f_l_piece_prio": false, "category": "", "tags": "", "super_seeding": false, "force_start": false},
        {"hash": "h2", "name": "Ubuntu", "size": 1024, "progress": 0.5, "dlspeed": 0, "upspeed": 0, "priority": 0, "num_seeds": 1, "num_leechs": 0, "num_incomplete": 0, "num_complete": 1, "ratio": 1.0, "eta": 0, "state": "downloading", "added_on": 100, "completion_on": 200, "seq_dl": false, "f_l_piece_prio": false, "category": "", "tags": "", "super_seeding": false, "force_start": false},
        {"hash": "h3", "name": "Debian", "size": 1024, "progress": 1.0, "dlspeed": 0, "upspeed": 0, "priority": 0, "num_seeds": 1, "num_leechs": 0, "num_incomplete": 0, "num_complete": 1, "ratio": 1.0, "eta": 0, "state": "uploading", "added_on": 100, "completion_on": 200, "seq_dl": false, "f_l_piece_prio": false, "category": "", "tags": "", "super_seeding": false, "force_start": false}
    ]"#;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_string(mock_torrents))
        .mount(&mock_server)
        .await;

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "find_duplicates",
            "arguments": {}
        })),
        id: Some(json!(1)),
    };

    let resp = server.handle_request(req).await.unwrap();
    let text = resp["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("\"name\": \"Ubuntu\""));
    assert!(text.contains("\"count\": 2"));
}

#[tokio::test]
async fn test_handle_resource_read_templates() {
    let (mock_server, server) = setup_mock_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/properties"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "save_path": "/tmp", "creation_date": 0, "piece_size": 0, "comment": "", "total_wasted": 0, "total_uploaded": 0, "total_downloaded": 0, "up_limit": 0, "dl_limit": 0, "time_elapsed": 0, "seeding_time": 0, "nb_connections": 0, "nb_connections_limit": 0, "share_ratio": 0.0, "addition_date": 0, "completion_date": 0, "created_by": "", "dl_speed_avg": 0, "dl_speed": 0, "eta": 0, "last_seen": 0, "peers": 0, "peers_total": 0, "pieces_have": 0, "pieces_num": 0, "reannounce": 0, "seeds": 0, "seeds_total": 0, "total_size": 0, "up_speed_avg": 0, "up_speed": 0 })))
        .mount(&mock_server)
        .await;

    let uris = vec![
        "qbittorrent://default/torrent/abc/properties",
        "qbittorrent://default/torrent/abc/files",
        "qbittorrent://default/torrent/abc/trackers",
    ];

    for uri in uris {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: Some(json!({ "uri": uri })),
            id: Some(json!(1)),
        };
        // Just verify it doesn't bail
        let _ = server.handle_request(req).await;
    }
}

#[tokio::test]
async fn test_handle_resource_read_legacy() {
    let (mock_server, server) = setup_mock_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&mock_server)
        .await;

    let uris = vec![
        "qbittorrent://torrents",
        "qbittorrent://transfer",
        "qbittorrent://categories",
    ];

    for uri in uris {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: Some(json!({ "uri": uri })),
            id: Some(json!(1)),
        };
        let _ = server.handle_request(req).await;
    }
}

#[tokio::test]
async fn test_handle_resource_read_errors() {
    let (_mock_server, server) = setup_mock_server().await;

    let uris = vec!["qbittorrent://unknown/torrents", "unknown://uri"];

    for uri in uris {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: Some(json!({ "uri": uri })),
            id: Some(json!(1)),
        };
        let result = server.handle_request(req).await;
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_handle_cleanup_completed_with_age() {
    let (mock_server, server) = setup_mock_server().await;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mock_torrents = json!([
        {
            "hash": "h1", "name": "T1", "size": 1024, "progress": 1.0, "dlspeed": 0, "upspeed": 0, "priority": 0,
            "num_seeds": 1, "num_leechs": 0, "num_incomplete": 0, "num_complete": 1, "ratio": 1.0, "eta": 0,
            "state": "uploading", "added_on": 100, "completion_on": now - (10 * 24 * 3600), // 10 days ago
            "seq_dl": false, "f_l_piece_prio": false, "category": "", "tags": "", "super_seeding": false, "force_start": false
        }
    ]).to_string();

    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_string(mock_torrents))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/delete"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "cleanup_completed",
            "arguments": {
                "max_age_days": 5,
                "delete_files": true
            }
        })),
        id: Some(json!(1)),
    };

    let resp = server.handle_request(req).await.unwrap();
    assert!(
        resp["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Successfully cleaned up 1 torrents")
    );
}

#[tokio::test]
async fn test_handle_search_torrents_lifecycle() {
    let (mock_server, server) = setup_mock_server().await;

    Mock::given(method("POST"))
        .and(path("/api/v2/search/start"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": 123})))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/search/results"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{"fileName": "f1", "fileUrl": "u1", "fileSize": 100, "nbSeeders": 1, "nbLeechers": 0, "siteUrl": "s1"}],
            "status": "Stopped",
            "total": 1
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/search/stop"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/search/delete"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "search_torrents",
            "arguments": { "query": "test" }
        })),
        id: Some(json!(1)),
    };

    let resp = server.handle_request(req).await.unwrap();
    assert!(resp["content"][0]["text"].as_str().unwrap().contains("f1"));
}
