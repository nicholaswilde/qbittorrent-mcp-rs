use crate::client::QBitClient;
use crate::config::AppConfig;
use crate::server::http::run_http_server;
use crate::server::mcp::McpServer;
use std::collections::HashMap;
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

pub async fn run_app(
    config: AppConfig,
    mut shutdown_rx: Option<tokio::sync::oneshot::Receiver<()>>,
) -> anyhow::Result<()> {
    // Keep guard alive for the duration of the program
    let _guard = init_logging(&config);

    info!(
        "Starting qBittorrent MCP Server in {} mode (lazy: {})",
        config.server_mode, config.lazy_mode
    );

    let instances = config.get_instances();
    let mut clients = HashMap::new();

    for inst in instances {
        if inst.host.trim().is_empty() {
            continue;
        }
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

    let server = McpServer::new(clients.clone(), config.lazy_mode);

    // Spawn background polling task for notifications
    server.start_event_loop(config.polling_interval_ms);

    let server_clone = server.clone();
    let mut server_handle = match config.server_mode.as_str() {
        "http" => {
            // Hardcoded port 3000 for now as it was in the original main.rs
            tokio::spawn(async move {
                let _ =
                    run_http_server(server_clone, "0.0.0.0", 3000, config.http_auth_token).await;
            })
        }
        _ => tokio::spawn(async move {
            let mut server_mut = server_clone;
            let _ = server_mut.run_stdio().await;
        }),
    };

    if let Some(rx) = shutdown_rx.take() {
        tokio::select! {
            _ = rx => {
                info!("Shutdown signal received");
                server.shutdown();
                server_handle.abort();
            }
            _ = &mut server_handle => {
                info!("Server task finished");
                server.shutdown();
            }
        }
    } else {
        server_handle.await?;
        server.shutdown();
    }

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
        let _ = registry.with(layer).try_init();
    } else {
        let _ = registry.try_init();
    }

    guard
}
