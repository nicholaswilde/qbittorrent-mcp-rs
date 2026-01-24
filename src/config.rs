use clap::ArgMatches;
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

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
        let matches = parse_args(cli_args);

        // 1. Determine Config File Path
        let path_to_load = if let Some(p) = file_path {
            Some(p)
        } else {
            matches.get_one::<String>("config").cloned()
        };

        // 2. Set Defaults
        builder = builder
            .set_default("qbittorrent_host", "localhost")?
            .set_default("qbittorrent_port", 8080)?
            .set_default("server_mode", "stdio")?;

        // 3. Load from File
        if let Some(path) = path_to_load {
            builder = builder.add_source(File::with_name(&path));
        } else {
            builder = builder.add_source(File::with_name("config").required(false));
        }

        // 4. Load from Environment Variables
        builder = builder.add_source(Environment::default().try_parsing(true));

        // 5. Apply CLI overrides
        if let Some(host) = matches.get_one::<String>("qbittorrent_host") {
            builder = builder.set_override("qbittorrent_host", host.as_str())?;
        }
        if let Some(port) = matches.get_one::<String>("qbittorrent_port") {
            builder = builder.set_override("qbittorrent_port", port.as_str())?;
        }
        if let Some(mode) = matches.get_one::<String>("server_mode") {
            builder = builder.set_override("server_mode", mode.as_str())?;
        }
        if let Some(user) = matches.get_one::<String>("qbittorrent_username") {
            builder = builder.set_override("qbittorrent_username", user.as_str())?;
        }
        if let Some(pass) = matches.get_one::<String>("qbittorrent_password") {
            builder = builder.set_override("qbittorrent_password", pass.as_str())?;
        }

        builder.build()?.try_deserialize()
    }
}

fn parse_args(args: Vec<String>) -> ArgMatches {
    use clap::{Arg, Command};

    let cmd = Command::new("qbittorrent-mcp-rs")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("Path to configuration file"),
        )
        .arg(
            Arg::new("qbittorrent_host")
                .long("qbittorrent-host")
                .help("Host of the qBittorrent Web UI"),
        )
        .arg(
            Arg::new("qbittorrent_port")
                .long("qbittorrent-port")
                .help("Port of the qBittorrent Web UI"),
        )
        .arg(
            Arg::new("server_mode")
                .long("server-mode")
                .help("Server mode: stdio or http"),
        )
        .arg(
            Arg::new("qbittorrent_username")
                .long("qbittorrent-username")
                .help("qBittorrent Username"),
        )
        .arg(
            Arg::new("qbittorrent_password")
                .long("qbittorrent-password")
                .help("qBittorrent Password"),
        );

    if args.is_empty() {
        cmd.get_matches_from(vec!["qbittorrent-mcp-rs"])
    } else {
        cmd.get_matches_from(args)
    }
}
