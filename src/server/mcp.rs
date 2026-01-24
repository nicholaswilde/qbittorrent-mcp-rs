use crate::client::QBitClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info};

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

struct McpState {
    lazy_mode: bool,
    tools_loaded: bool,
    should_notify: bool,
}

#[derive(Clone)]
pub struct McpServer {
    client: QBitClient,
    state: Arc<Mutex<McpState>>,
}

impl McpServer {
    pub fn new(client: QBitClient, lazy_mode: bool) -> Self {
        Self {
            client,
            state: Arc::new(Mutex::new(McpState {
                lazy_mode,
                tools_loaded: !lazy_mode,
                should_notify: false,
            })),
        }
    }

    pub fn check_notification(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.should_notify {
            state.should_notify = false;
            true
        } else {
            false
        }
    }

    pub async fn run_stdio(&mut self) -> Result<()> {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let mut line = String::new();

        loop {
            line.clear();
            let bytes = reader.read_line(&mut line)?;
            if bytes == 0 {
                break; // EOF
            }

            let input = line.trim();
            if input.is_empty() {
                continue;
            }

            debug!("Received: {}", input);

            match serde_json::from_str::<JsonRpcRequest>(input) {
                Ok(req) => {
                    let id = req.id.clone();
                    let resp = self.handle_request(req).await;

                    if let Some(req_id) = id {
                        let json_resp = match resp {
                            Ok(result) => JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: Some(req_id),
                                result: Some(result),
                                error: None,
                            },
                            Err(e) => JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: Some(req_id),
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32603,
                                    message: e.to_string(),
                                    data: None,
                                }),
                            },
                        };

                        let out = serde_json::to_string(&json_resp)?;
                        println!("{}", out);
                        io::stdout().flush()?;

                        // Check for notification (e.g. tool list changed)
                        if self.check_notification() {
                            let notification = json!({
                                "jsonrpc": "2.0",
                                "method": "notifications/tools/list_changed"
                            });
                            let out = serde_json::to_string(&notification)?;
                            println!("{}", out);
                            io::stdout().flush()?;
                        }
                    } else {
                        // Notification, no response expected
                        if let Err(e) = resp {
                            error!("Error handling notification: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse JSON-RPC: {}", e);
                }
            }
        }
        Ok(())
    }

    pub async fn handle_request(&self, req: JsonRpcRequest) -> Result<Value> {
        match req.method.as_str() {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "qbittorrent-mcp-rs",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {
                        "listChanged": true
                    },
                    "resources": {
                        "listChanged": false,
                        "subscribe": false
                    }
                }
            })),
            "notifications/initialized" => {
                info!("Client initialized");
                Ok(Value::Null)
            }
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({
                "tools": self.get_tool_definitions()
            })),
            "tools/call" => {
                if let Some(params) = req.params {
                    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let args = params.get("arguments").unwrap_or(&Value::Null);
                    self.call_tool(name, args).await
                } else {
                    anyhow::bail!("Missing params for tools/call");
                }
            }
            "resources/list" => Ok(json!({
                "resources": self.get_resource_definitions()
            })),
            "resources/read" => {
                if let Some(params) = req.params {
                    let uri = params.get("uri").and_then(|n| n.as_str()).unwrap_or("");
                    self.handle_resource_read(uri).await
                } else {
                    anyhow::bail!("Missing params for resources/read");
                }
            }
            _ => {
                anyhow::bail!("Method not found: {}", req.method);
            }
        }
    }

    fn get_resource_definitions(&self) -> Vec<Value> {
        vec![
            json!({
                "uri": "qbittorrent://torrents",
                "name": "Torrent List",
                "description": "A live list of all torrents and their current status",
                "mimeType": "application/json"
            }),
            json!({
                "uri": "qbittorrent://transfer",
                "name": "Global Transfer Info",
                "description": "Current global download/upload speeds and limits",
                "mimeType": "application/json"
            }),
            json!({
                "uri": "qbittorrent://categories",
                "name": "Categories",
                "description": "List of all defined torrent categories",
                "mimeType": "application/json"
            }),
        ]
    }

    async fn handle_resource_read(&self, uri: &str) -> Result<Value> {
        match uri {
            "qbittorrent://torrents" => {
                let torrents = self.client.get_torrent_list().await?;
                let content = serde_json::to_string_pretty(&torrents)?;
                Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": content
                    }]
                }))
            }
            "qbittorrent://transfer" => {
                let info = self.client.get_global_transfer_info().await?;
                let content = serde_json::to_string_pretty(&info)?;
                Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": content
                    }]
                }))
            }
            "qbittorrent://categories" => {
                let categories = self.client.get_categories().await?;
                let content = serde_json::to_string_pretty(&categories)?;
                Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": content
                    }]
                }))
            }
            _ => anyhow::bail!("Resource not found: {}", uri),
        }
    }

    fn get_tool_definitions(&self) -> Vec<Value> {
        let state = self.state.lock().unwrap();
        if state.lazy_mode && !state.tools_loaded {
            return vec![
                json!({
                    "name": "list_torrents",
                    "description": "List all torrents",
                    "inputSchema": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }),
                json!({
                    "name": "show_all_tools",
                    "description": "Enable all available tools",
                    "inputSchema": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }),
            ];
        }

        let mut tools = Vec::new();
        tools.extend(self.get_torrent_tools());
        tools.extend(self.get_search_tools());
        tools.extend(self.get_transfer_tools());
        tools.extend(self.get_rss_tools());
        tools.extend(self.get_app_tools());
        tools
    }

    fn get_torrent_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_torrents",
                "description": "List all torrents",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "add_torrent",
                "description": "Add a new torrent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "Magnet URI or HTTP URL" },
                        "save_path": { "type": "string", "description": "Optional save path" },
                        "category": { "type": "string", "description": "Optional category" }
                    },
                    "required": ["url"]
                }
            }),
            json!({
                "name": "pause_torrent",
                "description": "Pause a torrent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash (pipe-separated for multiple)" }
                    },
                    "required": ["hash"]
                }
            }),
            json!({
                "name": "resume_torrent",
                "description": "Resume a torrent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash (pipe-separated for multiple)" }
                    },
                    "required": ["hash"]
                }
            }),
            json!({
                "name": "delete_torrent",
                "description": "Delete a torrent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash (pipe-separated for multiple)" },
                        "delete_files": { "type": "boolean", "description": "Also delete files" }
                    },
                    "required": ["hash", "delete_files"]
                }
            }),
            json!({
                "name": "reannounce_torrent",
                "description": "Reannounce a torrent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash (pipe-separated for multiple)" }
                    },
                    "required": ["hash"]
                }
            }),
            json!({
                "name": "recheck_torrent",
                "description": "Recheck a torrent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash (pipe-separated for multiple)" }
                    },
                    "required": ["hash"]
                }
            }),
            json!({
                "name": "get_torrent_files",
                "description": "Get file list of a torrent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash" }
                    },
                    "required": ["hash"]
                }
            }),
            json!({
                "name": "get_torrent_properties",
                "description": "Get properties of a torrent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash" }
                    },
                    "required": ["hash"]
                }
            }),
            json!({
                "name": "create_category",
                "description": "Create a new category",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Category name" },
                        "save_path": { "type": "string", "description": "Save path for category" }
                    },
                    "required": ["name", "save_path"]
                }
            }),
            json!({
                "name": "set_torrent_category",
                "description": "Set category for torrents",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" },
                        "category": { "type": "string", "description": "Category name" }
                    },
                    "required": ["hashes", "category"]
                }
            }),
            json!({
                "name": "get_categories",
                "description": "Get all categories",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "add_torrent_tags",
                "description": "Add tags to torrents",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" },
                        "tags": { "type": "string", "description": "Comma-separated tags" }
                    },
                    "required": ["hashes", "tags"]
                }
            }),
            json!({
                "name": "wait_for_torrent_status",
                "description": "Poll a torrent until it reaches a desired state or timeout",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash" },
                        "target_status": { "type": "string", "description": "Status to wait for (e.g., uploading, stalledUP)" },
                        "timeout_seconds": { "type": "integer", "description": "Max wait time (default 60, max 300)" }
                    },
                    "required": ["hash", "target_status"]
                }
            }),
        ]
    }

    fn get_search_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "search_torrents",
                "description": "Search for torrents (waits 5s for results)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query" },
                        "category": { "type": "string", "description": "Optional category" }
                    },
                    "required": ["query"]
                }
            }),
            json!({
                "name": "install_search_plugin",
                "description": "Install a search plugin",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "URL to the plugin file" }
                    },
                    "required": ["url"]
                }
            }),
            json!({
                "name": "uninstall_search_plugin",
                "description": "Uninstall a search plugin",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Name of the plugin" }
                    },
                    "required": ["name"]
                }
            }),
            json!({
                "name": "enable_search_plugin",
                "description": "Enable or disable a search plugin",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Name of the plugin" },
                        "enable": { "type": "boolean", "description": "True to enable, False to disable" }
                    },
                    "required": ["name", "enable"]
                }
            }),
            json!({
                "name": "update_search_plugins",
                "description": "Update all search plugins",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "get_search_plugins",
                "description": "List installed search plugins",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
        ]
    }

    fn get_transfer_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "get_global_transfer_info",
                "description": "Get global transfer information",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "set_global_transfer_limits",
                "description": "Set global download and/or upload limits",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "dl_limit": { "type": "integer", "description": "Download limit in bytes per second (0 for unlimited)" },
                        "up_limit": { "type": "integer", "description": "Upload limit in bytes per second (0 for unlimited)" }
                    },
                    "required": []
                }
            }),
            json!({
                "name": "toggle_alternative_speed_limits",
                "description": "Toggle alternative speed limits",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "get_speed_limits_mode",
                "description": "Get the current state of alternative speed limits (0 for disabled, 1 for enabled)",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "ban_peers",
                "description": "Ban a list of peers",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "peers": { "type": "string", "description": "Peers to ban (host:port, pipe-separated)" }
                    },
                    "required": ["peers"]
                }
            }),
        ]
    }

    fn get_rss_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "add_rss_feed",
                "description": "Add a new RSS feed",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "URL of the RSS feed" },
                        "path": { "type": "string", "description": "Internal path/name for the feed" }
                    },
                    "required": ["url", "path"]
                }
            }),
            json!({
                "name": "get_rss_feeds",
                "description": "Get all RSS feeds and their items",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "set_rss_rule",
                "description": "Create or update an RSS auto-download rule",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Name of the rule" },
                        "definition": { "type": "string", "description": "JSON string defining the rule" }
                    },
                    "required": ["name", "definition"]
                }
            }),
            json!({
                "name": "get_rss_rules",
                "description": "Get all RSS auto-download rules",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
        ]
    }

    fn get_app_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "get_app_preferences",
                "description": "Get all application preferences",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "set_app_preferences",
                "description": "Set one or more application preferences",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "preferences": { "type": "string", "description": "JSON string of preferences to update" }
                    },
                    "required": ["preferences"]
                }
            }),
            json!({
                "name": "get_main_log",
                "description": "Get the main application log",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "severity": { "type": "string", "description": "Filter by severity (all, info, warning, critical)" },
                        "last_id": { "type": "integer", "description": "Exclude logs with ID less than or equal to this" }
                    },
                    "required": []
                }
            }),
            json!({
                "name": "get_peer_log",
                "description": "Get the peer connection log",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "last_id": { "type": "integer", "description": "Exclude logs with ID less than or equal to this" }
                    },
                    "required": []
                }
            }),
            json!({
                "name": "get_app_version",
                "description": "Get application version",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "get_build_info",
                "description": "Get build information",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "shutdown_app",
                "description": "Shutdown qBittorrent",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
        ]
    }

    pub async fn call_tool(&self, name: &str, args: &Value) -> Result<Value> {
        match name {
            "show_all_tools" => self.handle_show_all_tools(),

            // Torrent Management
            "list_torrents" => self.handle_list_torrents().await,
            "add_torrent" => self.handle_add_torrent(args).await,
            "pause_torrent" => self.handle_pause_torrent(args).await,
            "resume_torrent" => self.handle_resume_torrent(args).await,
            "delete_torrent" => self.handle_delete_torrent(args).await,
            "reannounce_torrent" => self.handle_reannounce_torrent(args).await,
            "recheck_torrent" => self.handle_recheck_torrent(args).await,
            "get_torrent_files" => self.handle_get_torrent_files(args).await,
            "get_torrent_properties" => self.handle_get_torrent_properties(args).await,
            "create_category" => self.handle_create_category(args).await,
            "set_torrent_category" => self.handle_set_torrent_category(args).await,
            "get_categories" => self.handle_get_categories().await,
            "add_torrent_tags" => self.handle_add_torrent_tags(args).await,
            "wait_for_torrent_status" => self.handle_wait_for_torrent_status(args).await,

            // Search
            "search_torrents" => self.handle_search_torrents(args).await,
            "install_search_plugin" => self.handle_install_search_plugin(args).await,
            "uninstall_search_plugin" => self.handle_uninstall_search_plugin(args).await,
            "enable_search_plugin" => self.handle_enable_search_plugin(args).await,
            "update_search_plugins" => self.handle_update_search_plugins().await,
            "get_search_plugins" => self.handle_get_search_plugins().await,

            // RSS
            "add_rss_feed" => self.handle_add_rss_feed(args).await,
            "get_rss_feeds" => self.handle_get_rss_feeds().await,
            "set_rss_rule" => self.handle_set_rss_rule(args).await,
            "get_rss_rules" => self.handle_get_rss_rules().await,

            // Transfer / App
            "get_global_transfer_info" => self.handle_get_global_transfer_info().await,
            "set_global_transfer_limits" => self.handle_set_global_transfer_limits(args).await,
            "toggle_alternative_speed_limits" => {
                self.handle_toggle_alternative_speed_limits().await
            }
            "get_speed_limits_mode" => self.handle_get_speed_limits_mode().await,
            "ban_peers" => self.handle_ban_peers(args).await,
            "get_app_preferences" => self.handle_get_app_preferences().await,
            "set_app_preferences" => self.handle_set_app_preferences(args).await,
            "get_main_log" => self.handle_get_main_log(args).await,
            "get_peer_log" => self.handle_get_peer_log(args).await,
            "get_app_version" => self.handle_get_app_version().await,
            "get_build_info" => self.handle_get_build_info().await,
            "shutdown_app" => self.handle_shutdown_app().await,

            _ => anyhow::bail!("Unknown tool: {}", name),
        }
    }

    fn handle_show_all_tools(&self) -> Result<Value> {
        let mut state = self.state.lock().unwrap();
        state.lazy_mode = false;
        state.tools_loaded = true;
        state.should_notify = true;
        Ok(
            json!({ "content": [{ "type": "text", "text": "All tools enabled. Please refresh your tool list." }] }),
        )
    }

    async fn handle_list_torrents(&self) -> Result<Value> {
        let torrents = self.client.get_torrent_list().await?;
        let text = serde_json::to_string_pretty(&torrents)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_search_torrents(&self, args: &Value) -> Result<Value> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing query"))?;
        let category = args.get("category").and_then(|v| v.as_str());

        let id = self.client.start_search(query, category).await?;

        let mut final_results = Vec::new();
        for _ in 0..5 {
            sleep(Duration::from_secs(1)).await;
            let resp = self.client.get_search_results(id, None, None).await;
            if let Ok(r) = resp {
                if r.status == "Stopped" {
                    final_results = r.results;
                    break;
                }
                final_results = r.results;
            }
        }
        let _ = self.client.stop_search(id).await;
        let _ = self.client.delete_search(id).await;
        let text = serde_json::to_string_pretty(&final_results)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_add_torrent(&self, args: &Value) -> Result<Value> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing url"))?;
        let save_path = args.get("save_path").and_then(|v| v.as_str());
        let category = args.get("category").and_then(|v| v.as_str());
        self.client.add_torrent(url, save_path, category).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Torrent added successfully" }] }))
    }

    async fn handle_pause_torrent(&self, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        self.client.pause_torrents(hash).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Torrent paused successfully" }] }))
    }

    async fn handle_resume_torrent(&self, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        self.client.resume_torrents(hash).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Torrent resumed successfully" }] }))
    }

    async fn handle_delete_torrent(&self, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        let delete_files = args
            .get("delete_files")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.client.delete_torrents(hash, delete_files).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Torrent deleted successfully" }] }))
    }

    async fn handle_reannounce_torrent(&self, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        self.client.reannounce_torrents(hash).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Torrent reannounced successfully" }] }))
    }

    async fn handle_recheck_torrent(&self, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        self.client.recheck_torrents(hash).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Torrent recheck started successfully" }] }),
        )
    }

    async fn handle_get_torrent_files(&self, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        let files = self.client.get_torrent_files(hash).await?;
        let text = serde_json::to_string_pretty(&files)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_get_torrent_properties(&self, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        let props = self.client.get_torrent_properties(hash).await?;
        let text = serde_json::to_string_pretty(&props)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_get_global_transfer_info(&self) -> Result<Value> {
        let info = self.client.get_global_transfer_info().await?;
        let text = serde_json::to_string_pretty(&info)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_set_global_transfer_limits(&self, args: &Value) -> Result<Value> {
        if let Some(limit) = args.get("dl_limit").and_then(|v| v.as_i64()) {
            self.client.set_download_limit(limit).await?;
        }
        if let Some(limit) = args.get("up_limit").and_then(|v| v.as_i64()) {
            self.client.set_upload_limit(limit).await?;
        }
        Ok(
            json!({ "content": [{ "type": "text", "text": "Transfer limits updated successfully" }] }),
        )
    }

    async fn handle_toggle_alternative_speed_limits(&self) -> Result<Value> {
        self.client.toggle_alternative_speed_limits().await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Alternative speed limits toggled" }] }))
    }

    async fn handle_get_speed_limits_mode(&self) -> Result<Value> {
        let mode = self.client.get_speed_limits_mode().await?;
        Ok(json!({ "content": [{ "type": "text", "text": mode.to_string() }] }))
    }

    async fn handle_ban_peers(&self, args: &Value) -> Result<Value> {
        let peers = args
            .get("peers")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing peers"))?;
        self.client.ban_peers(peers).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Peers banned successfully" }] }))
    }

    async fn handle_create_category(&self, args: &Value) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let save_path = args
            .get("save_path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing save_path"))?;
        self.client.create_category(name, save_path).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Category created successfully" }] }))
    }

    async fn handle_set_torrent_category(&self, args: &Value) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        let category = args
            .get("category")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing category"))?;
        self.client.set_category(hashes, category).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Category set successfully" }] }))
    }

    async fn handle_get_categories(&self) -> Result<Value> {
        let categories = self.client.get_categories().await?;
        let text = serde_json::to_string_pretty(&categories)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_add_torrent_tags(&self, args: &Value) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        let tags = args
            .get("tags")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing tags"))?;
        self.client.add_tags(hashes, tags).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Tags added successfully" }] }))
    }

    async fn handle_install_search_plugin(&self, args: &Value) -> Result<Value> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing url"))?;
        self.client.install_search_plugin(url).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Search plugin installed successfully" }] }),
        )
    }

    async fn handle_uninstall_search_plugin(&self, args: &Value) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        self.client.uninstall_search_plugin(name).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Search plugin uninstalled successfully" }] }),
        )
    }

    async fn handle_enable_search_plugin(&self, args: &Value) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let enable = args
            .get("enable")
            .and_then(|v| v.as_bool())
            .ok_or(anyhow::anyhow!("Missing enable"))?;
        self.client.enable_search_plugin(name, enable).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Search plugin status updated successfully" }] }),
        )
    }

    async fn handle_update_search_plugins(&self) -> Result<Value> {
        self.client.update_search_plugins().await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Search plugins updated successfully" }] }),
        )
    }

    async fn handle_get_search_plugins(&self) -> Result<Value> {
        let plugins = self.client.get_search_plugins().await?;
        let text = serde_json::to_string_pretty(&plugins)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_add_rss_feed(&self, args: &Value) -> Result<Value> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing url"))?;
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing path"))?;
        self.client.add_rss_feed(url, path).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "RSS feed added successfully" }] }))
    }

    async fn handle_get_rss_feeds(&self) -> Result<Value> {
        let feeds = self.client.get_all_rss_feeds().await?;
        let text = serde_json::to_string_pretty(&feeds)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_set_rss_rule(&self, args: &Value) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let definition = args
            .get("definition")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing definition"))?;
        self.client.set_rss_rule(name, definition).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "RSS rule set successfully" }] }))
    }

    async fn handle_get_rss_rules(&self) -> Result<Value> {
        let rules = self.client.get_all_rss_rules().await?;
        let text = serde_json::to_string_pretty(&rules)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_get_app_preferences(&self) -> Result<Value> {
        let prefs = self.client.get_app_preferences().await?;
        let text = serde_json::to_string_pretty(&prefs)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_set_app_preferences(&self, args: &Value) -> Result<Value> {
        let prefs_str = args
            .get("preferences")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing preferences"))?;
        let prefs_val: serde_json::Value = serde_json::from_str(prefs_str)?;
        self.client.set_app_preferences(&prefs_val).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "App preferences updated successfully" }] }),
        )
    }

    async fn handle_get_main_log(&self, args: &Value) -> Result<Value> {
        let severity = args
            .get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or("all");
        let last_id = args.get("last_id").and_then(|v| v.as_i64());
        let (normal, info, warning, critical) = match severity {
            "info" => (false, true, false, false),
            "warning" => (false, false, true, false),
            "critical" => (false, false, false, true),
            _ => (true, true, true, true),
        };
        let logs = self
            .client
            .get_main_log(normal, info, warning, critical, last_id)
            .await?;
        let text = serde_json::to_string_pretty(&logs)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_get_peer_log(&self, args: &Value) -> Result<Value> {
        let last_id = args.get("last_id").and_then(|v| v.as_i64());
        let logs = self.client.get_peer_log(last_id).await?;
        let text = serde_json::to_string_pretty(&logs)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_get_app_version(&self) -> Result<Value> {
        let version = self.client.get_app_version().await?;
        Ok(json!({ "content": [{ "type": "text", "text": version }] }))
    }

    async fn handle_get_build_info(&self) -> Result<Value> {
        let info = self.client.get_build_info().await?;
        let text = serde_json::to_string_pretty(&info)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_shutdown_app(&self) -> Result<Value> {
        self.client.shutdown_app().await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Shutdown command sent" }] }))
    }

    async fn handle_wait_for_torrent_status(&self, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        let target_status = args
            .get("target_status")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing target_status"))?;
        let timeout = args
            .get("timeout_seconds")
            .and_then(|v| v.as_i64())
            .unwrap_or(60)
            .clamp(1, 300);

        let start_time = std::time::Instant::now();
        while start_time.elapsed().as_secs() < timeout as u64 {
            let torrents = self.client.get_torrents_info(hash).await?;
            if let Some(t) = torrents.first() {
                if t.state == target_status {
                    return Ok(
                        json!({ "content": [{ "type": "text", "text": format!("Torrent reached target status: {}", target_status) }] }),
                    );
                }
            } else {
                anyhow::bail!("Torrent not found: {}", hash);
            }
            sleep(Duration::from_secs(2)).await;
        }
        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Timed out waiting for status {}", target_status) }], "isError": true }),
        )
    }
}
