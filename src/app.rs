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

    if config.server_mode == "http" && config.http_auth_token.as_deref().is_none_or(str::is_empty) {
        anyhow::bail!(
            "HTTP mode requires an auth token (QBITTORRENT_HTTP_AUTH_TOKEN or --http-auth-token); without one anyone on the network can control qBittorrent"
        );
    }

    let instances = config.get_instances();
    let mut clients = HashMap::new();

    for inst in instances {
        if inst.host.trim().is_empty() {
            continue;
        }
        let base_url = build_base_url(&inst.host, inst.port)?;

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
        "http" => tokio::spawn(async move {
            run_http_server(
                server_clone,
                &config.http_host,
                config.http_port,
                config.http_auth_token,
            )
            .await
        }),
        _ => tokio::spawn(async move {
            let mut server_mut = server_clone;
            server_mut.run_stdio().await
        }),
    };

    let result = if let Some(rx) = shutdown_rx.take() {
        tokio::select! {
            _ = rx => {
                info!("Shutdown signal received");
                server_handle.abort();
                Ok(())
            }
            res = &mut server_handle => res?,
        }
    } else {
        server_handle.await?
    };
    server.shutdown();
    for client in clients.values().filter(|c| c.has_credentials()) {
        let _ = client.logout().await;
    }
    // A server that fails to start (e.g. port in use) must exit non-zero with the reason.
    result?;

    info!("Shutting down qBittorrent MCP Server");
    Ok(())
}

/// Builds the Web UI URL; a port already in the host URL wins over `port`.
fn build_base_url(host: &str, port: Option<u16>) -> anyhow::Result<String> {
    let with_scheme = if host.starts_with("http://") || host.starts_with("https://") {
        host.to_string()
    } else {
        format!("http://{}", host)
    };
    let mut url = url::Url::parse(&with_scheme)?;
    if url.port().is_none()
        && let Some(p) = port
    {
        url.set_port(Some(p))
            .map_err(|_| anyhow::anyhow!("Cannot set port on {}", host))?;
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
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

#[cfg(test)]
mod tests {
    use super::build_base_url;

    #[test]
    fn base_url_does_not_double_the_port() {
        assert_eq!(
            build_base_url("http://h:9777", Some(8080)).unwrap(),
            "http://h:9777"
        );
        assert_eq!(
            build_base_url("http://h", Some(8080)).unwrap(),
            "http://h:8080"
        );
        assert_eq!(build_base_url("h", None).unwrap(), "http://h");
        assert_eq!(
            build_base_url("https://h/qbt/", None).unwrap(),
            "https://h/qbt"
        );
    }
}
