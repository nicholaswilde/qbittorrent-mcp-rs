use anyhow::Result;
use qbittorrent_mcp_rs::client::QBitClient;
use testcontainers::GenericImage;
use testcontainers::ImageExt;
use testcontainers::core::{ContainerPort, Mount, WaitFor};
use testcontainers::runners::AsyncRunner;
use tokio::io::AsyncBufReadExt;

#[tokio::test]
async fn test_harness_connectivity() -> Result<()> {
    println!("🐳 Starting qBittorrent container...");

    let config_path = std::env::current_dir()?.join("tests/resources/qBittorrent.conf");
    let config_path_str = config_path.to_str().expect("Valid path");

    // Use latest linuxserver image
    let image = GenericImage::new("linuxserver/qbittorrent", "latest")
        .with_exposed_port(ContainerPort::Tcp(8080))
        .with_wait_for(WaitFor::message_on_stdout("WebUI will be started"))
        .with_env_var("PUID", "1000")
        .with_env_var("PGID", "1000")
        .with_env_var("WEBUI_PORT", "8080")
        .with_mount(Mount::bind_mount(config_path_str, "/config/qBittorrent/qBittorrent.conf"));

    let container = image.start().await?;

    // Pipe stdout logs
    let stdout = container.stdout(true);
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            println!("DOCKER STDOUT: {}", line);
        }
    });

    // Pipe stderr logs
    let stderr = container.stderr(true);
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            println!("DOCKER STDERR: {}", line);
        }
    });

    let port = container.get_host_port_ipv4(8080).await?;
    let base_url = format!("http://localhost:{}", port);

    println!("✅ qBittorrent container started at {}", base_url);

    // Allow some time for the WebUI to actually be ready
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Check connectivity
    let client_http = reqwest::Client::new();
    let resp = client_http.get(&base_url).send().await?;
    println!("Connectivity Status: {}", resp.status());
    assert!(resp.status().as_u16() < 500, "Container unreachable");

    println!("🔑 Using credentials from config (admin / adminadmin)");

    // Initialize the client
    let client = QBitClient::new(&base_url, "admin", "adminadmin", false);

    // Login
    println!("Logging in...");
    client
        .login()
        .await
        .expect("Failed to login to qBittorrent");
    println!("✅ Login successful!");

    // --- Phase 2: System Info Tools (GREEN PHASE) ---
    println!("Testing get_app_version...");
    let version = client.get_app_version().await?;
    println!("✅ App Version: {}", version);

    println!("Testing get_build_info...");
    let build_info = client.get_build_info().await?;
    println!("✅ Build Info: {:?}", build_info);

    println!("Testing get_app_preferences...");
    let _prefs = client.get_app_preferences().await?;
    println!("✅ App Preferences retrieved");

    Ok(())
}