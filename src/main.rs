use qbittorrent_mcp_rs::app::run_app;
use qbittorrent_mcp_rs::config::AppConfig;
use std::env;
use tracing::error;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let config = match AppConfig::load(None, args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = run_app(config, None).await {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}
