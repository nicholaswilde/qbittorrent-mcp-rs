use qbittorrent_mcp_rs::client::QBitClient;
use qbittorrent_mcp_rs::server::mcp::{JsonRpcRequest, McpServer};
use serde_json::json;
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_mcp_resource_read_complex() {
    let mock_server = MockServer::start().await;
    let client = QBitClient::new_no_auth(mock_server.uri(), false);
    let mut clients = HashMap::new();
    clients.insert("default".to_string(), client.clone());
    clients.insert("inst1".to_string(), client);

    let server = McpServer::new(clients, false);

    // Mock for torrent properties
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/properties"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "save_path": "/downloads",
            "creation_date": 123,
            "piece_size": 1024,
            "comment": "",
            "total_wasted": 0,
            "total_uploaded": 0,
            "total_downloaded": 0,
            "up_limit": 0,
            "dl_limit": 0,
            "time_elapsed": 0,
            "seeding_time": 0,
            "nb_connections": 0,
            "nb_connections_limit": 0,
            "share_ratio": 0.0,
            "addition_date": 123,
            "completion_date": 0,
            "created_by": "",
            "dl_speed_avg": 0,
            "dl_speed": 0,
            "eta": 0,
            "last_seen": 0,
            "peers": 0,
            "peers_total": 0,
            "pieces_have": 0,
            "pieces_num": 0,
            "reannounce": 0,
            "seeds": 0,
            "seeds_total": 0,
            "total_size": 0,
            "up_speed_avg": 0,
            "up_speed": 0
        })))
        .mount(&mock_server)
        .await;

    // Test properties resource
    let res = server
        .handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: Some(json!({"uri": "qbittorrent://inst1/torrent/abc/properties"})),
            id: Some(json!(1)),
        })
        .await
        .unwrap();
    assert!(res.get("contents").is_some());

    // Mock for torrent files
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;
    let res = server
        .handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: Some(json!({"uri": "qbittorrent://inst1/torrent/abc/files"})),
            id: Some(json!(2)),
        })
        .await
        .unwrap();
    assert!(res.get("contents").is_some());

    // Mock for trackers
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/trackers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;
    let res = server
        .handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: Some(json!({"uri": "qbittorrent://inst1/torrent/abc/trackers"})),
            id: Some(json!(3)),
        })
        .await
        .unwrap();
    assert!(res.get("contents").is_some());

    // Mock for torrents list
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;
    let res = server
        .handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: Some(json!({"uri": "qbittorrent://torrents"})),
            id: Some(json!(4)),
        })
        .await
        .unwrap();
    assert!(res.get("contents").is_some());

    // Mock for transfer info
    Mock::given(method("GET"))
        .and(path("/api/v2/transfer/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "dl_info_speed": 0,
            "dl_info_data": 0,
            "up_info_speed": 0,
            "up_info_data": 0,
            "dl_rate_limit": 0,
            "up_rate_limit": 0,
            "dht_nodes": 0,
            "connection_status": "connected"
        })))
        .mount(&mock_server)
        .await;
    let res = server
        .handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: Some(json!({"uri": "qbittorrent://transfer"})),
            id: Some(json!(5)),
        })
        .await
        .unwrap();
    assert!(res.get("contents").is_some());
}

#[tokio::test]
async fn test_mcp_resource_list() {
    let clients = HashMap::new();
    let server = McpServer::new(clients, false);
    let res = server
        .handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/list".to_string(),
            params: None,
            id: Some(json!(6)),
        })
        .await
        .unwrap();
    assert!(res.get("resources").is_some());
}

#[tokio::test]
async fn test_mcp_errors() {
    let clients = HashMap::new();
    let server = McpServer::new(clients, false);

    // get_client fallback error (when no clients)
    let res = server
        .handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: Some(json!({"uri": "qbittorrent://torrents"})),
            id: Some(json!(1)),
        })
        .await;
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .to_string()
            .contains("No instances configured")
    );

    // Missing params for tools/call
    let res2 = server
        .handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: None,
            id: Some(json!(2)),
        })
        .await;
    assert!(res2.is_err());

    // Missing params for resources/read
    let res3 = server
        .handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "resources/read".to_string(),
            params: None,
            id: Some(json!(3)),
        })
        .await;
    assert!(res3.is_err());

    // Missing params for prompts/get
    let res4 = server
        .handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "prompts/get".to_string(),
            params: None,
            id: Some(json!(4)),
        })
        .await;
    assert!(res4.is_err());

    // Ping
    let res5 = server
        .handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "ping".to_string(),
            params: None,
            id: Some(json!(5)),
        })
        .await
        .unwrap();
    assert!(res5.is_object());
}
