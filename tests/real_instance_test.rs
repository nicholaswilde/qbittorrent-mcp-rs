use qbittorrent_mcp_rs::client::QBitClient;
use std::env;

#[tokio::test]
async fn test_real_instance_connectivity() {
    let host = env::var("QBIT_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("QBIT_PORT").unwrap_or_else(|_| "8080".to_string());
    let username = env::var("QBIT_USERNAME").ok();
    let password = env::var("QBIT_PASSWORD").ok();
    let protocol = env::var("QBIT_PROTOCOL").unwrap_or_else(|_| "http".to_string());

    let base_url = if host.starts_with("http") {
        host
    } else {
        format!("{}://{}:{}", protocol, host, port)
    };

    println!("Testing connectivity to: {}", base_url);

    let client = if let (Some(u), Some(p)) = (username, password) {
        QBitClient::new(base_url, u, p)
    } else {
        QBitClient::new_no_auth(base_url)
    };

    // Try to login if we have creds
    if client.login().await.is_ok() {
        println!("Login successful");
    } else {
        println!("Login failed or not attempted");
    }

    let result = client.get_torrent_list().await;
    match result {
        Ok(torrents) => {
            println!("Successfully retrieved {} torrents", torrents.len());
            for t in torrents.iter().take(5) {
                println!(" - {}: {}%", t.name, (t.progress * 100.0).round());
            }
        }
        Err(e) => {
            panic!("Failed to connect to real instance: {}", e);
        }
    }
}
