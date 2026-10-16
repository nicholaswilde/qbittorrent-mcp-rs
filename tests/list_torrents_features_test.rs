use qbittorrent_mcp_rs::client::QBitClient;
use qbittorrent_mcp_rs::server::mcp::McpServer;
use serde_json::json;
use std::collections::HashMap;
use std::env;

async fn setup() -> (Option<wiremock::MockServer>, McpServer) {
    if let Ok(host) = env::var("QBIT_HOST") {
        let client = QBitClient::new_no_auth(host, false);
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        return (None, McpServer::new(clients, false));
    }

    let mock_server = wiremock::MockServer::start().await;
    let mock_torrents = json!([
        {
            "hash": "0123456789abcdef0123456789abcdef01234567",
            "name": "Ubuntu 22.04 LTS Desktop",
            "size": 2000000000_i64,
            "progress": 0.45,
            "dlspeed": 50000000_i64,
            "upspeed": 1000000_i64,
            "priority": 1,
            "num_seeds": 10,
            "num_leechs": 2,
            "num_incomplete": 2,
            "num_complete": 10,
            "ratio": 0.5,
            "eta": 60,
            "state": "downloading",
            "added_on": 1500000000,
            "completion_on": 0,
            "seq_dl": true,
            "f_l_piece_prio": false,
            "category": "linux",
            "tags": "os",
            "super_seeding": false,
            "force_start": false
        },
        {
            "hash": "123456789abcdef0123456789abcdef012345670",
            "name": "Ben 10 Ultimate Alien S01",
            "size": 500000000_i64,
            "progress": 1.0,
            "dlspeed": 0,
            "upspeed": 500000_i64,
            "priority": 1,
            "num_seeds": 5,
            "num_leechs": 0,
            "num_incomplete": 0,
            "num_complete": 5,
            "ratio": 1.2,
            "eta": 0,
            "state": "uploading",
            "added_on": 1500000000,
            "completion_on": 1500001000,
            "seq_dl": false,
            "f_l_piece_prio": false,
            "category": "TV",
            "tags": "animation",
            "super_seeding": false,
            "force_start": false
        },
        {
            "hash": "short_hash",
            "name": "Ben 10 (2005) Season 1",
            "size": 1200000000_i64,
            "progress": 1.0,
            "dlspeed": 0,
            "upspeed": 0,
            "priority": 1,
            "num_seeds": 3,
            "num_leechs": 0,
            "num_incomplete": 0,
            "num_complete": 3,
            "ratio": 2.0,
            "eta": 0,
            "state": "pausedUP",
            "added_on": 1500000000,
            "completion_on": 1500002000,
            "seq_dl": false,
            "f_l_piece_prio": false,
            "category": "TV",
            "tags": "animation",
            "super_seeding": false,
            "force_start": false
        },
        {
            "hash": "abcdef0123456789abcdef0123456789abcdef01",
            "name": "Debian 12 Netinst",
            "size": 700000000_i64,
            "progress": 1.0,
            "dlspeed": 0,
            "upspeed": 0,
            "priority": 1,
            "num_seeds": 8,
            "num_leechs": 0,
            "num_incomplete": 0,
            "num_complete": 8,
            "ratio": 1.0,
            "eta": 0,
            "state": "queuedUP",
            "added_on": 1500000000,
            "completion_on": 1500003000,
            "seq_dl": false,
            "f_l_piece_prio": false,
            "category": "",
            "tags": "",
            "super_seeding": false,
            "force_start": false
        }
    ]);

    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/api/v2/torrents/info"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(mock_torrents))
        .mount(&mock_server)
        .await;

    let client = QBitClient::new_no_auth(mock_server.uri(), false);
    let mut clients = HashMap::new();
    clients.insert("default".to_string(), client);
    (Some(mock_server), McpServer::new(clients, false))
}

#[tokio::test]
async fn test_summary_mode() {
    let (_mock, server) = setup().await;

    let result = server
        .call_tool("list_torrents", &json!({ "summary": true, "limit": 5 }))
        .await
        .expect("summary call failed");

    let text = result["content"][0]["text"].as_str().unwrap();
    println!("=== Summary Mode (limit 5) ===\n{}", text);

    assert!(text.contains("Found "));
    assert!(text.contains("Name"));
    assert!(text.contains("Prog %"));
    assert!(text.contains("State"));
    assert!(text.contains("Hash"));
}

#[tokio::test]
async fn test_search_filter() {
    let (_mock, server) = setup().await;

    let result = server
        .call_tool(
            "list_torrents",
            &json!({ "search": "ubuntu", "summary": true }),
        )
        .await
        .expect("search call failed");

    let text = result["content"][0]["text"].as_str().unwrap();
    println!("=== Search 'ubuntu' ===\n{}", text);

    // Every line after the header should contain "ubuntu" (case-insensitive)
    for line in text.lines().skip(3) {
        if !line.is_empty() {
            assert!(
                line.to_lowercase().contains("ubuntu"),
                "Line doesn't match search: {}",
                line
            );
        }
    }
}

#[tokio::test]
async fn test_field_selection() {
    let (_mock, server) = setup().await;

    let result = server
        .call_tool(
            "list_torrents",
            &json!({ "fields": ["name", "progress", "state"], "limit": 3 }),
        )
        .await
        .expect("fields call failed");

    let text = result["content"][0]["text"].as_str().unwrap();
    println!("=== Field Selection ===\n{}", text);

    let parsed: serde_json::Value = serde_json::from_str(text).expect("should be valid JSON");
    assert!(parsed["total"].as_u64().unwrap() > 0);

    let torrents = parsed["torrents"].as_array().unwrap();
    for t in torrents {
        let obj = t.as_object().unwrap();
        // Should only have the requested fields
        assert!(obj.contains_key("name"), "missing 'name'");
        assert!(obj.contains_key("progress"), "missing 'progress'");
        assert!(obj.contains_key("state"), "missing 'state'");
        // Should NOT have other fields
        assert!(!obj.contains_key("hash"), "should not have 'hash'");
        assert!(!obj.contains_key("dlspeed"), "should not have 'dlspeed'");
        println!(
            "  {} - {:.0}% - {}",
            obj["name"].as_str().unwrap(),
            obj["progress"].as_f64().unwrap() * 100.0,
            obj["state"].as_str().unwrap()
        );
    }
}

#[tokio::test]
async fn test_search_with_fields() {
    let (_mock, server) = setup().await;

    let result = server
        .call_tool(
            "list_torrents",
            &json!({ "search": "ben 10", "fields": ["name", "hash", "progress", "category"] }),
        )
        .await
        .expect("search+fields call failed");

    let text = result["content"][0]["text"].as_str().unwrap();
    println!("=== Search 'ben 10' + Fields ===\n{}", text);

    let parsed: serde_json::Value = serde_json::from_str(text).expect("should be valid JSON");
    let torrents = parsed["torrents"].as_array().unwrap();
    for t in torrents {
        assert!(
            t["name"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .contains("ben 10")
        );
    }
}

#[tokio::test]
async fn test_no_results_search() {
    let (_mock, server) = setup().await;

    let result = server
        .call_tool(
            "list_torrents",
            &json!({ "search": "xyznonexistenttorrent999", "summary": true }),
        )
        .await
        .expect("empty search call failed");

    let text = result["content"][0]["text"].as_str().unwrap();
    println!("=== Empty Search ===\n{}", text);
    assert!(text.contains("Found 0 torrents"));
}

#[tokio::test]
async fn test_group_by_state_and_short_hash() {
    let (_mock, server) = setup().await;

    let result = server
        .call_tool("list_torrents", &json!({ "group_by_state": true }))
        .await
        .expect("group_by_state failed");

    let text = result["content"][0]["text"].as_str().unwrap();
    println!("=== Group by State ===\n{}", text);

    assert!(text.contains("Total: 4 torrents across 4 states"));
    assert!(text.contains("── downloading (1) ──"));
    assert!(text.contains("── uploading (1) ──"));
    assert!(text.contains("── pausedUP (1) ──"));
    assert!(text.contains("── queuedUP (1) ──"));
    assert!(text.contains("short_hash"));
}

#[tokio::test]
async fn test_group_by_category() {
    let (_mock, server) = setup().await;

    let result = server
        .call_tool("list_torrents", &json!({ "group_by_category": true }))
        .await
        .expect("group_by_category failed");

    let text = result["content"][0]["text"].as_str().unwrap();
    println!("=== Group by Category ===\n{}", text);

    assert!(text.contains("Total: 4 torrents across 3 categories"));
    assert!(text.contains("── TV (2) ──"));
    assert!(text.contains("── linux (1) ──"));
    assert!(text.contains("── (none) (1) ──"));
}

#[tokio::test]
async fn test_stats_only() {
    let (_mock, server) = setup().await;

    let result = server
        .call_tool("list_torrents", &json!({ "stats_only": true }))
        .await
        .expect("stats_only failed");

    let text = result["content"][0]["text"].as_str().unwrap();
    println!("=== Stats Only ===\n{}", text);

    assert!(text.contains("4 torrents"));
    assert!(text.contains("By state:"));
    assert!(text.contains("By category:"));
    assert!(text.contains("downloading"));
    assert!(text.contains("uploading"));
}

#[tokio::test]
async fn test_search_with_limit_pagination() {
    let (_mock, server) = setup().await;

    // Search for "Ben 10" with limit 1
    let result = server
        .call_tool(
            "list_torrents",
            &json!({ "search": "Ben 10", "limit": 1, "fields": ["name"] }),
        )
        .await
        .expect("search with limit failed");

    let text = result["content"][0]["text"].as_str().unwrap();
    println!("=== Search with Limit ===\n{}", text);

    let parsed: serde_json::Value = serde_json::from_str(text).expect("should be valid JSON");
    assert_eq!(parsed["total"].as_u64().unwrap(), 1);
    let torrents = parsed["torrents"].as_array().unwrap();
    assert_eq!(torrents.len(), 1);
    assert!(torrents[0]["name"].as_str().unwrap().contains("Ben 10"));
}
