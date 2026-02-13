use anyhow::Result;
use futures::StreamExt;
use qbittorrent_mcp_rs::client::QBitClient;
use qbittorrent_mcp_rs::server::http::{create_router, run_http_server};
use qbittorrent_mcp_rs::server::mcp::McpServer;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

async fn setup_test_server(auth_token: Option<String>) -> (String, tokio::task::JoinHandle<()>) {
    let client = QBitClient::new("http://localhost:8080", "admin", "adminadmin", false);
    let mut clients = HashMap::new();
    clients.insert("default".to_string(), client);
    let server = McpServer::new(clients, false);

    let app = create_router(server, auth_token).await;

    // Use port 0 for ephemeral port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();
    let base_url = format!("http://127.0.0.1:{}", port);

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Wait a tiny bit for server to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    (base_url, handle)
}

#[tokio::test]
async fn test_http_sse_session_and_message() -> Result<()> {
    let (base_url, _handle) = setup_test_server(None).await;
    let client = reqwest::Client::new();

    // 1. Connect to SSE
    let mut source = client
        .get(format!("{}/sse", base_url))
        .send()
        .await?
        .bytes_stream();

    // 2. Expect 'endpoint' event
    let first_chunk = timeout(Duration::from_secs(2), source.next())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Expected endpoint event, got None"))??;
    let first_chunk_str = String::from_utf8_lossy(&first_chunk);

    assert!(first_chunk_str.contains("event: endpoint"));
    assert!(first_chunk_str.contains("data: /message?session_id="));

    // Extract session_id
    let session_id = first_chunk_str.split("session_id=").last().unwrap().trim();

    // 3. Send a message
    let req_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping",
        "params": {}
    });

    let resp = client
        .post(format!("{}/message?session_id={}", base_url, session_id))
        .json(&req_body)
        .send()
        .await?;

    assert_eq!(resp.status(), reqwest::StatusCode::ACCEPTED);

    // 4. Expect 'message' event in SSE
    let second_chunk = timeout(Duration::from_secs(2), source.next())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Expected message event, got None"))??;
    let second_chunk_str = String::from_utf8_lossy(&second_chunk);

    assert!(second_chunk_str.contains("event: message"));
    assert!(second_chunk_str.contains("\"result\":{}"));
    assert!(second_chunk_str.contains("\"id\":1"));
    assert!(second_chunk_str.contains("\"jsonrpc\":\"2.0\""));

    Ok(())
}

#[tokio::test]
async fn test_http_auth_header() -> Result<()> {
    let token = "test-token".to_string();
    let (base_url, _handle) = setup_test_server(Some(token.clone())).await;
    let client = reqwest::Client::new();

    // Unauthorized
    let resp = client.get(format!("{}/sse", base_url)).send().await?;
    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);

    // Authorized
    let resp = client
        .get(format!("{}/sse", base_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_http_auth_query_param() -> Result<()> {
    let token = "test-token".to_string();
    let (base_url, _handle) = setup_test_server(Some(token.clone())).await;
    let client = reqwest::Client::new();

    // Authorized via query param
    let resp = client
        .get(format!("{}/sse?token={}", base_url, token))
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn test_http_session_not_found() -> Result<()> {
    let (base_url, _handle) = setup_test_server(None).await;
    let client = reqwest::Client::new();

    let req_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping",
        "params": {}
    });

    let resp = client
        .post(format!("{}/message?session_id=invalid-session", base_url))
        .json(&req_body)
        .send()
        .await?;

    assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn test_run_http_server_execution() -> Result<()> {
    let client = QBitClient::new("http://localhost:8080", "admin", "adminadmin", false);
    let mut clients = HashMap::new();
    clients.insert("default".to_string(), client);
    let server = McpServer::new(clients, false);

    // Pick a likely free port
    let port = 3098;
    let server_clone = server.clone();

    let handle = tokio::spawn(async move {
        let _ = run_http_server(server_clone, "127.0.0.1", port, None).await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client_http = reqwest::Client::new();
    let resp = client_http
        .get(format!("http://127.0.0.1:{}/sse", port))
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_http_malformed_json() -> Result<()> {
    let (base_url, _handle) = setup_test_server(None).await;
    let client = reqwest::Client::new();

    let first_chunk_str = {
        let mut source = client
            .get(format!("{}/sse", base_url))
            .send()
            .await?
            .bytes_stream();
        let chunk = timeout(Duration::from_secs(2), source.next())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Expected endpoint event"))??;
        String::from_utf8_lossy(&chunk).to_string()
    };
    let session_id = first_chunk_str.split("session_id=").last().unwrap().trim();

    let resp = client
        .post(format!("{}/message?session_id={}", base_url, session_id))
        .body("not-json")
        .header("Content-Type", "application/json")
        .send()
        .await?;

    assert_ne!(resp.status(), reqwest::StatusCode::ACCEPTED);

    Ok(())
}

#[tokio::test]
async fn test_http_mcp_error_response() -> Result<()> {
    let (base_url, _handle) = setup_test_server(None).await;
    let client = reqwest::Client::new();

    let mut source = client
        .get(format!("{}/sse", base_url))
        .send()
        .await?
        .bytes_stream();
    let first_chunk = timeout(Duration::from_secs(2), source.next())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Expected endpoint event"))??;
    let first_chunk_str = String::from_utf8_lossy(&first_chunk);
    let session_id = first_chunk_str.split("session_id=").last().unwrap().trim();

    // Call a non-existent tool
    let req_body = json!({
        "jsonrpc": "2.0",
        "id": 999,
        "method": "tools/call",
        "params": { "name": "non_existent_tool", "arguments": {} }
    });

    client
        .post(format!("{}/message?session_id={}", base_url, session_id))
        .json(&req_body)
        .send()
        .await?;

    let second_chunk = timeout(Duration::from_secs(2), source.next())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Expected message event"))??;
    let second_chunk_str = String::from_utf8_lossy(&second_chunk);

    assert!(second_chunk_str.contains("event: message"));
    assert!(second_chunk_str.contains("\"error\":"));
    assert!(second_chunk_str.contains("\"code\":-32603"));

    Ok(())
}
