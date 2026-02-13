use clap::ArgMatches;
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct QBitInstance {
    pub name: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub no_verify_ssl: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub instances: Option<Vec<QBitInstance>>,
    pub qbittorrent_host: String,
    pub qbittorrent_port: Option<u16>,
    pub qbittorrent_username: Option<String>,
    pub qbittorrent_password: Option<String>,
    pub server_mode: String,
    pub lazy_mode: bool,
    pub no_verify_ssl: bool,
    pub log_level: String,
    pub log_file_enable: bool,
    pub log_dir: String,
    pub log_filename: String,
    pub log_rotate: String,
    pub http_auth_token: Option<String>,
    #[serde(default)]
    pub polling_interval_ms: u64,
}

impl AppConfig {
    pub fn get_instances(&self) -> Vec<QBitInstance> {
        let instances = self.instances.as_ref().filter(|i| !i.is_empty());
        if let Some(instances) = instances {
            return instances.clone();
        }

        // Fallback to legacy single instance
        vec![QBitInstance {
            name: "default".to_string(),
            host: self.qbittorrent_host.clone(),
            port: self.qbittorrent_port,
            username: self.qbittorrent_username.clone(),
            password: self.qbittorrent_password.clone(),
            no_verify_ssl: Some(self.no_verify_ssl),
        }]
    }

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
            .set_default("server_mode", "stdio")?
            .set_default("lazy_mode", false)?
            .set_default("no_verify_ssl", false)?
            .set_default("log_level", "info")?
            .set_default("log_file_enable", false)?
            .set_default("log_dir", ".")?
            .set_default("log_filename", "qbittorrent-mcp-rs.log")?
            .set_default("log_rotate", "daily")?
            .set_default("polling_interval_ms", 2000)?;

        // 3. Load from File
        if let Some(path) = path_to_load {
            builder = builder.add_source(File::with_name(&path));
        } else {
            builder = builder.add_source(File::with_name("config").required(false));
        }

        // 4. Load from Environment Variables
        builder = builder.add_source(
            Environment::with_prefix("QBITTORRENT")
                .prefix_separator("_")
                .separator("__")
                .try_parsing(true),
        );

        // 5. Apply CLI overrides
        if let Some(host) = matches.get_one::<String>("qbittorrent_host") {
            builder = builder.set_override("qbittorrent_host", host.as_str())?;
        }
        if let Some(port) = matches.get_one::<u16>("qbittorrent_port") {
            builder = builder.set_override("qbittorrent_port", *port)?;
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
        if matches.get_flag("lazy_mode") {
            builder = builder.set_override("lazy_mode", true)?;
        }
        if matches.get_flag("no_verify_ssl") {
            builder = builder.set_override("no_verify_ssl", true)?;
        }
        if let Some(level) = matches.get_one::<String>("log_level") {
            builder = builder.set_override("log_level", level.as_str())?;
        }
        if matches.get_flag("log_file_enable") {
            builder = builder.set_override("log_file_enable", true)?;
        }
        if let Some(dir) = matches.get_one::<String>("log_dir") {
            builder = builder.set_override("log_dir", dir.as_str())?;
        }
        if let Some(filename) = matches.get_one::<String>("log_filename") {
            builder = builder.set_override("log_filename", filename.as_str())?;
        }
        if let Some(rotate) = matches.get_one::<String>("log_rotate") {
            builder = builder.set_override("log_rotate", rotate.as_str())?;
        }
        if let Some(token) = matches.get_one::<String>("http_auth_token") {
            builder = builder.set_override("http_auth_token", token.as_str())?;
        }
        if let Some(interval) = matches.get_one::<u64>("polling_interval_ms") {
            builder = builder.set_override("polling_interval_ms", *interval)?;
        }

        builder.build()?.try_deserialize()
    }
}

fn parse_args(args: Vec<String>) -> ArgMatches {
    use clap::{Arg, ArgAction, Command};

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
                .help("Port of the qBittorrent Web UI")
                .value_parser(clap::value_parser!(u16)),
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
        )
        .arg(
            Arg::new("lazy_mode")
                .long("lazy")
                .action(ArgAction::SetTrue)
                .help("Enable lazy mode (show fewer tools initially)"),
        )
        .arg(
            Arg::new("no_verify_ssl")
                .long("no-verify-ssl")
                .action(ArgAction::SetTrue)
                .help("Disable SSL certificate verification (insecure)"),
        )
        .arg(
            Arg::new("log_level")
                .short('L')
                .long("log-level")
                .help("Log level (error, warn, info, debug, trace)")
                .default_value("info"),
        )
        .arg(
            Arg::new("log_file_enable")
                .long("log-file-enable")
                .action(ArgAction::SetTrue)
                .help("Enable logging to a file"),
        )
        .arg(
            Arg::new("log_dir")
                .long("log-dir")
                .help("Log file directory")
                .default_value("."),
        )
        .arg(
            Arg::new("log_filename")
                .long("log-filename")
                .help("Log filename prefix")
                .default_value("qbittorrent-mcp-rs.log"),
        )
        .arg(
            Arg::new("log_rotate")
                .long("log-rotate")
                .help("Log rotation strategy (daily, hourly, never)")
                .default_value("daily"),
        )
        .arg(
            Arg::new("http_auth_token")
                .long("http-auth-token")
                .help("Authentication token for HTTP server mode"),
        )
        .arg(
            Arg::new("polling_interval_ms")
                .long("polling-interval-ms")
                .help("Polling interval for notifications (ms)")
                .value_parser(clap::value_parser!(u64)),
        );

    if args.is_empty() {
        cmd.get_matches_from(vec!["qbittorrent-mcp-rs"])
    } else {
        cmd.get_matches_from(args)
    }
}
