use clap::Parser;
use qbittorrent_mcp_rs::config::AppConfig;
use std::env;
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<String>,

    /// qBittorrent Host
    #[arg(long)]
    qbittorrent_host: Option<String>,

    /// qBittorrent Port
    #[arg(long)]
    qbittorrent_port: Option<u16>,

    /// Server Mode (stdio or http)
    #[arg(long)]
    server_mode: Option<String>,

    /// qBittorrent Username
    #[arg(long)]
    qbittorrent_username: Option<String>,

    /// qBittorrent Password
    #[arg(long)]
    qbittorrent_password: Option<String>,

    /// Lazy mode (show fewer tools initially)
    #[arg(long, action)]
    lazy: bool,
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // We need to pass args to AppConfig::load.
    // AppConfig::load expects Vec<String> for clap parsing inside it,
    // OR we can refactor AppConfig to take parsed args or just use the Args struct here
    // and pass values to AppConfig.
    // However, AppConfig::load currently duplicates the clap logic for the sake of "loading config".
    // It's better if `AppConfig::load` handles the merging logic.
    // `AppConfig::load` calls `parse_args` internally.
    // So we just collect env::args().

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

    // Simple pre-parse to find config file if possible, or we let AppConfig handle it.
    // I'll update AppConfig to handle --config.

    let config = AppConfig::load(None, args)?;

    info!(
        "Starting qBittorrent MCP Server in {} mode (lazy: {})",
        config.server_mode, config.lazy_mode
    );
    info!(
        "Connecting to qBittorrent at {}:{}",
        config.qbittorrent_host, config.qbittorrent_port
    );
    info!("Press Ctrl+C to stop");

    use qbittorrent_mcp_rs::client::QBitClient;
    use qbittorrent_mcp_rs::server::{MCPServer, http::HttpServer, stdio::StdioServer};

    let base_url = if config.qbittorrent_host.starts_with("http://") || config.qbittorrent_host.starts_with("https://") {
        config.qbittorrent_host.clone()
    } else {
        format!(
            "http://{}:{}",
            config.qbittorrent_host, config.qbittorrent_port
        )
    };

    let client =
        if let (Some(u), Some(p)) = (&config.qbittorrent_username, &config.qbittorrent_password) {
            QBitClient::new(base_url, u, p)
        } else {
            QBitClient::new_no_auth(base_url)
        };

    // We should probably login immediately to verify creds?
    // Or let the tools do it. Tools should probably ensure login.
    // For now, we assume client logic handles it (stateless or re-login).
    // QBitClient::login needs to be called.
    // Let's call login if auth is provided.

    if config.qbittorrent_username.is_some() {
        if let Err(e) = client.login().await {
            error!("Failed to login to qBittorrent: {}", e);
            // Should we exit? Maybe just warn.
        } else {
            info!("Logged in to qBittorrent successfully");
        }
    }

    let server: Box<dyn MCPServer> = match config.server_mode.as_str() {
        "http" => Box::new(HttpServer::new(3000, client)),
        _ => Box::new(StdioServer::new(client, config.lazy_mode)),
    };

    server.run().await.map_err(|e| anyhow::anyhow!(e))?;

    info!("Shutting down qBittorrent MCP Server");
    Ok(())
}
