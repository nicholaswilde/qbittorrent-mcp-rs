use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use clap::ArgMatches;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub qbittorrent_host: String,
    pub qbittorrent_port: u16,
    pub qbittorrent_username: Option<String>,
    pub qbittorrent_password: Option<String>,
    pub server_mode: String,
}

impl AppConfig {
    pub fn load(file_path: Option<String>, cli_args: Vec<String>) -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        // 1. Set Defaults
        builder = builder
            .set_default("qbittorrent_host", "localhost")?
            .set_default("qbittorrent_port", 8080)?
            .set_default("server_mode", "stdio")?;

        // 2. Load from File (if provided)
        if let Some(path) = file_path {
            builder = builder.add_source(File::with_name(&path));
        } else {
            // Try default locations? For now, explicit path or implicit "config"
            builder = builder.add_source(File::with_name("config").required(false));
        }

        // 3. Load from Environment Variables
        // QBITTORRENT_HOST, QBITTORRENT_PORT, etc.
        builder = builder.add_source(Environment::default().try_parsing(true));

        // 4. Build to get intermediate config (to merge CLI overrides manually? or use a source?)
        
        // We need to parse CLI args to get overrides.
        // Since `cli_args` is passed as a Vec<String> (for testing), we should parse it.
        // Use clap to parse `cli_args`.
        
        let matches = parse_args(cli_args);

        // Apply overrides
        if let Some(host) = matches.get_one::<String>("qbittorrent_host") {
             builder = builder.set_override("qbittorrent_host", host.as_str())?;
        }
        if let Some(port) = matches.get_one::<String>("qbittorrent_port") {
             builder = builder.set_override("qbittorrent_port", port.as_str())?;
        }
        if let Some(mode) = matches.get_one::<String>("server_mode") {
             builder = builder.set_override("server_mode", mode.as_str())?;
        }

        builder.build()?.try_deserialize()
    }
}

fn parse_args(args: Vec<String>) -> ArgMatches {
    use clap::{Command, Arg};

    // If args is empty, it might mean "no args passed" (except binary name usually).
    // In our test, we pass "app", "flag".
    // If we call this from main, we might pass env::args().
    
    let cmd = Command::new("qbittorrent-mcp-rs")
        .arg(Arg::new("qbittorrent_host")
            .long("qbittorrent-host")
            .help("Host of the qBittorrent Web UI"))
        .arg(Arg::new("qbittorrent_port")
            .long("qbittorrent-port")
            .help("Port of the qBittorrent Web UI"))
        .arg(Arg::new("server_mode")
            .long("server-mode")
            .help("Server mode: stdio or http"))
        .arg(Arg::new("qbittorrent_username")
            .long("qbittorrent-username")
            .help("qBittorrent Username"))
        .arg(Arg::new("qbittorrent_password")
            .long("qbittorrent-password")
            .help("qBittorrent Password"));

    if args.is_empty() {
        cmd.get_matches_from(vec!["qbittorrent-mcp-rs"])
    } else {
        cmd.get_matches_from(args)
    }
}
