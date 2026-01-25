use qbittorrent_mcp_rs::config::AppConfig;
use serde_json::json;
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => {}
        Err(e) => {
            error!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let config = AppConfig::load(None, args)?;

    // Keep guard alive for the duration of the program
    let _guard = init_logging(&config);

    info!(
        "Starting qBittorrent MCP Server in {} mode (lazy: {})",
        config.server_mode, config.lazy_mode
    );

    use qbittorrent_mcp_rs::client::QBitClient;
    use qbittorrent_mcp_rs::server::http::run_http_server;
    use qbittorrent_mcp_rs::server::mcp::McpServer;
    use std::collections::HashMap;

    let instances = config.get_instances();
    let mut clients = HashMap::new();

    for inst in instances {
        let base_url = if inst.host.starts_with("http://") || inst.host.starts_with("https://") {
            if let Some(port) = inst.port {
                format!("{}:{}", inst.host, port)
            } else {
                inst.host.clone()
            }
        } else {
            let port = inst.port.unwrap_or(80);
            format!("http://{}:{}", inst.host, port)
        };

        let no_verify_ssl =
            inst.no_verify_ssl.unwrap_or(config.no_verify_ssl) && base_url.starts_with("https://");

        info!("Initializing client '{}' at {}", inst.name, base_url);

        let client = if let (Some(u), Some(p)) = (&inst.username, &inst.password) {
            QBitClient::new(base_url, u, p, no_verify_ssl)
        } else {
            QBitClient::new_no_auth(base_url, no_verify_ssl)
        };

        if inst.username.is_some() {
            if let Err(e) = client.login().await {
                error!(
                    "Failed to login to qBittorrent instance '{}': {}",
                    inst.name, e
                );
            } else {
                info!(
                    "Logged in to qBittorrent instance '{}' successfully",
                    inst.name
                );
            }
        }
        clients.insert(inst.name, client);
    }

    if clients.is_empty() {
        anyhow::bail!("No qBittorrent instances configured");
    }

    let mut server = McpServer::new(clients.clone(), config.lazy_mode);

    // Spawn background polling task for notifications
    let server_for_polling = server.clone();
    let clients_for_polling = clients.clone();
    tokio::spawn(async move {
        let mut last_rids: HashMap<String, i64> = HashMap::new();
        let mut notified_finished: HashMap<String, std::collections::HashSet<String>> =
            HashMap::new();

        for name in clients_for_polling.keys() {
            last_rids.insert(name.clone(), 0);
            notified_finished.insert(name.clone(), std::collections::HashSet::new());
        }

        loop {
            sleep(Duration::from_secs(10)).await;
            for (name, client) in &clients_for_polling {
                let rid = *last_rids.get(name).unwrap_or(&0);
                match client.get_main_data(rid).await {
                    Ok(data) => {
                        last_rids.insert(name.clone(), data.rid);

                        // Track finished torrents to notify only once
                        if let Some(torrents) = data.torrents {
                            for (hash, torrent_val) in torrents {
                                let progress = torrent_val.get("progress").and_then(|p| p.as_f64());
                                let state = torrent_val.get("state").and_then(|s| s.as_str());

                                if progress.is_some_and(|p| p >= 1.0 || state == Some("uploading"))
                                {
                                    let already_notified =
                                        notified_finished.get_mut(name).unwrap().contains(&hash);
                                    if !already_notified {
                                        let torrent_name = torrent_val
                                            .get("name")
                                            .and_then(|n| n.as_str())
                                            .unwrap_or(&hash);
                                        info!(
                                            "Notification: Torrent '{}' finished on {}",
                                            torrent_name, name
                                        );
                                        server_for_polling.push_notification(
                                            "notifications/torrent_finished",
                                            json!({
                                                "instance": name,
                                                "hash": hash,
                                                "name": torrent_name
                                            }),
                                        );
                                        notified_finished
                                            .get_mut(name)
                                            .unwrap()
                                            .insert(hash.clone());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => error!("Polling error for instance {}: {}", name, e),
                }
            }
        }
    });

    match config.server_mode.as_str() {
        "http" => {
            // Hardcoded port 3000 for now as it was in the original main.rs
            run_http_server(server, "0.0.0.0", 3000, config.http_auth_token).await?;
        }
        _ => {
            server.run_stdio().await?;
        }
    };

    info!("Shutting down qBittorrent MCP Server");
    Ok(())
}

fn init_logging(config: &AppConfig) -> Option<WorkerGuard> {
    let filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(filter_layer.clone());

    let (file_layer, guard) = if config.log_file_enable {
        let rotation = match config.log_rotate.to_lowercase().as_str() {
            "hourly" => Rotation::HOURLY,
            "never" => Rotation::NEVER,
            _ => Rotation::DAILY,
        };

        let file_appender = RollingFileAppender::builder()
            .rotation(rotation)
            .filename_prefix(&config.log_filename)
            .build(&config.log_dir)
            .expect("Failed to create log file appender");

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        (
            Some(
                tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .with_filter(filter_layer),
            ),
            Some(guard),
        )
    } else {
        (None, None)
    };

    let registry = tracing_subscriber::registry().with(stdout_layer);

    if let Some(layer) = file_layer {
        registry.with(layer).init();
    } else {
        registry.init();
    }

    guard
}
