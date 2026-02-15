use anyhow::Result;
use qbittorrent_mcp_rs::client::QBitClient;
use qbittorrent_mcp_rs::server::mcp::{JsonRpcRequest, McpServer};
use serde_json::{Value, json};
use std::collections::HashMap;
use testcontainers::GenericImage;
use testcontainers::ImageExt;
use testcontainers::core::{ContainerPort, Mount, WaitFor};
use testcontainers::runners::AsyncRunner;
use tokio::io::AsyncBufReadExt;

#[tokio::test]
async fn test_docker_mcp_integration() -> Result<()> {
    // Skip if RUN_DOCKER_TESTS is not set to true in CI
    if std::env::var("CI").is_ok()
        && std::env::var("RUN_DOCKER_TESTS").unwrap_or_default() != "true"
    {
        println!("Skipping Docker integration test (RUN_DOCKER_TESTS not set to true)");
        return Ok(());
    }

    println!("🐳 Starting qBittorrent container...");

    let config_path = std::env::current_dir()?.join("tests/resources/qBittorrent.conf");
    let config_path_str = config_path.to_str().expect("Valid path");

    let image = GenericImage::new("linuxserver/qbittorrent", "latest")
        .with_exposed_port(ContainerPort::Tcp(8080))
        .with_wait_for(WaitFor::message_on_stdout("WebUI will be started"))
        .with_env_var("PUID", "1000")
        .with_env_var("PGID", "1000")
        .with_env_var("WEBUI_PORT", "8080")
        .with_mount(Mount::bind_mount(
            config_path_str,
            "/config/qBittorrent/qBittorrent.conf",
        ));

    let container = match image.start().await {
        Ok(c) => c,
        Err(e) => {
            println!(
                "⚠️ Failed to start Docker container. Skipping integration test. Error: {}",
                e
            );
            return Ok(());
        }
    };

    // Pipe stdout logs for debugging
    let stdout = container.stdout(true);
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            println!("DOCKER STDOUT: {}", line);
        }
    });

    let port = container.get_host_port_ipv4(8080).await?;
    let base_url = format!("http://localhost:{}", port);

    println!("✅ qBittorrent container started at {}", base_url);

    // Allow some time for the WebUI to actually be ready
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Initialize Client and MCP Server
    let client = QBitClient::new(&base_url, "admin", "adminadmin", false);
    client
        .login()
        .await
        .expect("Failed to login to qBittorrent");

    let mut clients = HashMap::new();
    clients.insert("default".to_string(), client);
    let server = McpServer::new(clients, false);

    // --- 1. Test get_system_info (Consolidated) ---
    println!("Testing get_system_info...");
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "get_system_info",
            "arguments": {}
        })),
        id: Some(json!(1)),
    };
    let resp = server.handle_request(req).await?;
    let text = resp["content"][0]["text"].as_str().unwrap();
    let sys_info: Value = serde_json::from_str(text)?;
    assert!(sys_info.get("app_version").is_some());
    assert!(sys_info.get("transfer_info").is_some());
    println!("✅ get_system_info verified");

    // --- 2. Setup: Add a torrent ---
    println!("Adding test torrent...");
    let magnet = "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10&dn=Sintel";
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "add_torrent",
            "arguments": { "url": magnet }
        })),
        id: Some(json!(2)),
    };
    server.handle_request(req).await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // --- 3. Test list_torrents (Enhanced) ---
    println!("Testing list_torrents with properties...");
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "list_torrents",
            "arguments": { "include_properties": true }
        })),
        id: Some(json!(3)),
    };
    let resp = server.handle_request(req).await?;
    let text = resp["content"][0]["text"].as_str().unwrap();
    let list: Value = serde_json::from_str(text)?;
    let torrent = &list[0];
    assert!(torrent.get("hash").is_some());
    assert!(torrent.get("properties").is_some());
    let hash = torrent["hash"].as_str().unwrap().to_string();
    println!("✅ list_torrents (enhanced) verified");

    // --- 4. Test inspect_torrent (Consolidated) ---
    println!("Testing inspect_torrent...");
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "inspect_torrent",
            "arguments": { "hash": hash }
        })),
        id: Some(json!(4)),
    };
    let resp = server.handle_request(req).await?;
    let text = resp["content"][0]["text"].as_str().unwrap();
    let inspection: Value = serde_json::from_str(text)?;
    assert!(inspection.get("properties").is_some());
    assert!(inspection.get("files").is_some());
    assert!(inspection.get("trackers").is_some());
    println!("✅ inspect_torrent verified");

    // --- 5. Test manage_torrents (Unified) ---
    println!("Testing manage_torrents: pause/resume...");
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "manage_torrents",
            "arguments": { "hashes": hash, "action": "pause" }
        })),
        id: Some(json!(5)),
    };
    server.handle_request(req).await?;
    println!("✅ manage_torrents verified");

    // Cleanup
    println!("Cleaning up...");
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "delete_torrent",
            "arguments": { "hash": hash, "delete_files": true }
        })),
        id: Some(json!(6)),
    };
    server.handle_request(req).await?;

    Ok(())
}
