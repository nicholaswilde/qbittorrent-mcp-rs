use qbittorrent_mcp_rs::client::QBitClient;
use std::env;

#[tokio::test]
async fn test_real_instance_connectivity() {
    let host = match env::var("QBIT_HOST") {
        Ok(h) => h,
        Err(_) => {
            println!("Skipping real instance test: QBIT_HOST not set");
            return;
        }
    };
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
        QBitClient::new(base_url, u, p, false)
    } else {
        QBitClient::new_no_auth(base_url, false)
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

            // Test Categories (Read)
            println!("\n--- Testing Categories ---");
            match client.get_categories().await {
                Ok(cats) => {
                    println!("Successfully retrieved {} categories", cats.len());
                    for (name, cat) in cats.iter().take(3) {
                        println!(" - {}: {}", name, cat.save_path);
                    }
                }
                Err(e) => println!("Failed to get categories: {}", e),
            }

            // Test Create Category
            println!("\n--- Testing Create Category ---");
            match client.create_category("mcp_test_category", "").await {
                Ok(_) => {
                    println!("Successfully created category 'mcp_test_category'");
                    // Verify it exists
                    match client.get_categories().await {
                        Ok(cats) => {
                            if cats.contains_key("mcp_test_category") {
                                println!("Verified 'mcp_test_category' exists in category list");
                            } else {
                                println!(
                                    "WARNING: 'mcp_test_category' not found in list immediately after creation"
                                );
                            }
                        }
                        Err(e) => println!("Failed to get categories: {}", e),
                    }
                }
                Err(e) => println!("Failed to create category: {}", e),
            }

            // Test Global Transfer Info
            println!("\n--- Testing Global Transfer Info ---");
            match client.get_global_transfer_info().await {
                Ok(info) => {
                    println!("Connection Status: {}", info.connection_status);
                    println!("DL Speed: {} b/s", info.dl_info_speed);
                    println!("UP Speed: {} b/s", info.up_info_speed);
                }
                Err(e) => println!("Failed to get global info: {}", e),
            }

            // Test App Preferences
            println!("\n--- Testing App Preferences ---");
            match client.get_app_preferences().await {
                Ok(prefs) => {
                    println!("Successfully retrieved app preferences");
                    println!("Save path: {}", prefs["save_path"]);
                }
                Err(e) => println!("Failed to get app preferences: {}", e),
            }

            // Test RSS
            println!("\n--- Testing RSS ---");
            match client.get_all_rss_feeds().await {
                Ok(feeds) => println!("Successfully retrieved {} RSS feeds", feeds.len()),
                Err(e) => println!("Failed to get RSS feeds: {}", e),
            }
            match client.get_all_rss_rules().await {
                Ok(rules) => println!("Successfully retrieved {} RSS rules", rules.len()),
                Err(e) => println!("Failed to get RSS rules: {}", e),
            }

            // Test Search Plugins
            println!("\n--- Testing Search Plugins ---");
            match client.get_search_plugins().await {
                Ok(plugins) => {
                    println!("Successfully retrieved {} search plugins", plugins.len());
                    for p in plugins.iter().take(3) {
                        println!(" - {}: enabled={}", p.name, p.enabled);
                    }
                }
                Err(e) => println!("Failed to get search plugins: {}", e),
            }

            // Test Search
            println!("\n--- Testing Search (ubuntu) ---");
            match client.start_search("ubuntu", None).await {
                Ok(id) => {
                    println!("Search started with ID: {}", id);
                    // Poll a few times
                    for i in 0..5 {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        match client.get_search_results(id, None, None).await {
                            Ok(resp) => {
                                println!(
                                    "Poll {}: Status '{}', Found {} results",
                                    i + 1,
                                    resp.status,
                                    resp.total
                                );
                                if resp.status == "Stopped" || resp.total > 0 {
                                    if !resp.results.is_empty() {
                                        println!("First result: {}", resp.results[0].file_name);
                                    }
                                    break;
                                }
                            }
                            Err(e) => println!("Poll failed: {}", e),
                        }
                    }
                    let _ = client.stop_search(id).await;
                    let _ = client.delete_search(id).await;
                    println!("Search cleaned up");
                }
                Err(e) => println!("Failed to start search: {}", e),
            }
        }
        Err(e) => {
            panic!("Failed to connect to real instance: {}", e);
        }
    }
}
