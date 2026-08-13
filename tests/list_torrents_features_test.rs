use qbittorrent_mcp_rs::client::QBitClient;
use qbittorrent_mcp_rs::server::mcp::McpServer;
use serde_json::json;
use std::collections::HashMap;
use std::env;

fn setup() -> Option<McpServer> {
    let host = env::var("QBIT_HOST").ok()?;
    let client = QBitClient::new_no_auth(host, false);
    let mut clients = HashMap::new();
    clients.insert("default".to_string(), client);
    Some(McpServer::new(clients, false))
}

#[tokio::test]
async fn test_summary_mode() {
    let server = match setup() {
        Some(s) => s,
        None => {
            println!("Skipping: QBIT_HOST not set");
            return;
        }
    };

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
    let server = match setup() {
        Some(s) => s,
        None => {
            println!("Skipping: QBIT_HOST not set");
            return;
        }
    };

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
    let server = match setup() {
        Some(s) => s,
        None => {
            println!("Skipping: QBIT_HOST not set");
            return;
        }
    };

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
    let server = match setup() {
        Some(s) => s,
        None => {
            println!("Skipping: QBIT_HOST not set");
            return;
        }
    };

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
    let server = match setup() {
        Some(s) => s,
        None => {
            println!("Skipping: QBIT_HOST not set");
            return;
        }
    };

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
