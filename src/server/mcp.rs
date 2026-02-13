use crate::client::QBitClient;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
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
    notification_queue: Vec<Value>,
}

#[derive(Clone)]
pub struct McpServer {
    clients: HashMap<String, QBitClient>,
    state: Arc<Mutex<McpState>>,
}

impl McpServer {
    pub fn new(clients: HashMap<String, QBitClient>, lazy_mode: bool) -> Self {
        Self {
            clients,
            state: Arc::new(Mutex::new(McpState {
                lazy_mode,
                tools_loaded: !lazy_mode,
                should_notify: false,
                notification_queue: Vec::new(),
            })),
        }
    }

    pub fn push_notification(&self, method: &str, params: Value) {
        let mut state = self.state.lock().unwrap();
        state.notification_queue.push(json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }));
    }

    async fn flush_notifications_async<W: tokio::io::AsyncWrite + Unpin>(
        &self,
        writer: &mut W,
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let mut pending = Vec::new();
        {
            let mut state = self.state.lock().unwrap();
            if state.should_notify {
                state.should_notify = false;
                pending.push(json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/tools/list_changed"
                }));
            }
            while !state.notification_queue.is_empty() {
                pending.push(state.notification_queue.remove(0));
            }
        }

        for n in pending {
            let out = serde_json::to_string(&n)? + "\n";
            writer.write_all(out.as_bytes()).await?;
        }
        writer.flush().await?;
        Ok(())
    }

    fn get_client(&self, instance: Option<&str>) -> Result<&QBitClient> {
        if let Some(name) = instance {
            self.clients
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("Instance not found: {}", name))
        } else {
            // Use "default" if exists, otherwise first one
            if let Some(client) = self.clients.get("default") {
                Ok(client)
            } else {
                self.clients
                    .values()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("No instances configured"))
            }
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
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, stdin, stdout};
        let mut reader = BufReader::new(stdin()).lines();
        let mut stdout = stdout();

        loop {
            tokio::select! {
                line_res = reader.next_line() => {
                    let line = match line_res? {
                        Some(l) => l,
                        None => break, // EOF
                    };
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

                                let out = serde_json::to_string(&json_resp)? + "\n";
                                stdout.write_all(out.as_bytes()).await?;
                                stdout.flush().await?;
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
                    self.flush_notifications_async(&mut stdout).await?;
                }
                _ = sleep(Duration::from_millis(500)) => {
                    self.flush_notifications_async(&mut stdout).await?;
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
                    },
                    "prompts": {
                        "listChanged": false
                    }
                }
            })),
            "notifications/initialized" => {
                info!("Client initialized");
                Ok(Value::Null)
            }
            "ping" => Ok(json!({})),
            "prompts/list" => Ok(json!({
                "prompts": self.get_prompt_definitions()
            })),
            "prompts/get" => {
                if let Some(params) = req.params {
                    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let args = params.get("arguments").unwrap_or(&Value::Null);
                    self.handle_prompt_get(name, args).await
                } else {
                    anyhow::bail!("Missing params for prompts/get");
                }
            }
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
        let mut res = Vec::new();
        for name in self.clients.keys() {
            res.push(json!({
                "uri": format!("qbittorrent://{}/torrents", name),
                "name": format!("Torrent List ({})", name),
                "description": format!("A live list of all torrents on instance: {}", name),
                "mimeType": "application/json"
            }));
            res.push(json!({
                "uri": format!("qbittorrent://{}/transfer", name),
                "name": format!("Global Transfer Info ({})", name),
                "description": format!("Current speeds and limits on instance: {}", name),
                "mimeType": "application/json"
            }));
            res.push(json!({
                "uri": format!("qbittorrent://{}/categories", name),
                "name": format!("Categories ({})", name),
                "description": format!("All defined categories on instance: {}", name),
                "mimeType": "application/json"
            }));
        }

        // Templates
        res.push(json!({
            "uriTemplate": "qbittorrent://{instance}/torrent/{hash}/properties",
            "name": "Torrent Properties",
            "description": "Detailed properties and metadata for a specific torrent",
            "mimeType": "application/json"
        }));
        res.push(json!({
            "uriTemplate": "qbittorrent://{instance}/torrent/{hash}/files",
            "name": "Torrent Files",
            "description": "List of files and their progress within a specific torrent",
            "mimeType": "application/json"
        }));
        res.push(json!({
            "uriTemplate": "qbittorrent://{instance}/torrent/{hash}/trackers",
            "name": "Torrent Trackers",
            "description": "Current trackers and their status for a specific torrent",
            "mimeType": "application/json"
        }));
        res
    }

    async fn handle_resource_read(&self, uri: &str) -> Result<Value> {
        let re_torrents = Regex::new(r"qbittorrent://([^/]+)/torrents")?;
        let re_transfer = Regex::new(r"qbittorrent://([^/]+)/transfer")?;
        let re_categories = Regex::new(r"qbittorrent://([^/]+)/categories")?;
        let re_props = Regex::new(r"qbittorrent://([^/]+)/torrent/([^/]+)/properties")?;
        let re_files = Regex::new(r"qbittorrent://([^/]+)/torrent/([^/]+)/files")?;
        let re_trackers = Regex::new(r"qbittorrent://([^/]+)/torrent/([^/]+)/trackers")?;

        if let Some(caps) = re_torrents.captures(uri) {
            let instance = &caps[1];
            let client = self.get_client(Some(instance))?;
            let torrents = client
                .get_torrent_list(None, None, None, None, None, None, None)
                .await?;
            let content = serde_json::to_string_pretty(&torrents)?;
            return Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": content
                }]
            }));
        }

        if let Some(caps) = re_transfer.captures(uri) {
            let instance = &caps[1];
            let client = self.get_client(Some(instance))?;
            let info = client.get_global_transfer_info().await?;
            let content = serde_json::to_string_pretty(&info)?;
            return Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": content
                }]
            }));
        }

        if let Some(caps) = re_categories.captures(uri) {
            let instance = &caps[1];
            let client = self.get_client(Some(instance))?;
            let categories = client.get_categories().await?;
            let content = serde_json::to_string_pretty(&categories)?;
            return Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": content
                }]
            }));
        }

        if let Some(caps) = re_props.captures(uri) {
            let instance = &caps[1];
            let hash = &caps[2];
            let client = self.get_client(Some(instance))?;
            let props = client.get_torrent_properties(hash).await?;
            let content = serde_json::to_string_pretty(&props)?;
            return Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": content
                }]
            }));
        }

        if let Some(caps) = re_files.captures(uri) {
            let instance = &caps[1];
            let hash = &caps[2];
            let client = self.get_client(Some(instance))?;
            let files = client.get_torrent_files(hash).await?;
            let content = serde_json::to_string_pretty(&files)?;
            return Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": content
                }]
            }));
        }

        if let Some(caps) = re_trackers.captures(uri) {
            let instance = &caps[1];
            let hash = &caps[2];
            let client = self.get_client(Some(instance))?;
            let trackers = client.get_torrent_trackers(hash).await?;
            let content = serde_json::to_string_pretty(&trackers)?;
            return Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": content
                }]
            }));
        }

        // Fallback for legacy URIs (without instance) - use default client
        match uri {
            "qbittorrent://torrents" => {
                let client = self.get_client(None)?;
                let torrents = client
                    .get_torrent_list(None, None, None, None, None, None, None)
                    .await?;
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
                let client = self.get_client(None)?;
                let info = client.get_global_transfer_info().await?;
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
                let client = self.get_client(None)?;
                let categories = client.get_categories().await?;
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

    fn get_prompt_definitions(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "fix_stalled_torrent",
                "description": "Get instructions and context to troubleshoot a stalled or slow torrent",
                "arguments": [
                    {
                        "name": "hash",
                        "description": "Torrent hash to troubleshoot",
                        "required": true
                    },
                    {
                        "name": "instance",
                        "description": "Instance name (optional)",
                        "required": false
                    }
                ]
            }),
            json!({
                "name": "analyze_disk_space",
                "description": "Check if there is enough disk space for current downloads",
                "arguments": [
                    {
                        "name": "instance",
                        "description": "Instance name (optional)",
                        "required": false
                    }
                ]
            }),
            json!({
                "name": "optimize_speed",
                "description": "Suggest optimizations for slow downloads",
                "arguments": [
                    {
                        "name": "instance",
                        "description": "Instance name (optional)",
                        "required": false
                    }
                ]
            }),
            json!({
                "name": "troubleshoot_connection",
                "description": "Diagnose connection and connectivity issues",
                "arguments": [
                    {
                        "name": "instance",
                        "description": "Instance name (optional)",
                        "required": false
                    }
                ]
            }),
            json!({
                "name": "rules-of-engagement",
                "description": "Get the behavioral rules and best practices for interacting with this qBittorrent MCP server",
                "arguments": []
            }),
        ]
    }

    async fn handle_prompt_get(&self, name: &str, args: &Value) -> Result<Value> {
        let instance = args
            .get("instance")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        match name {
            "fix_stalled_torrent" => {
                let hash = args
                    .get("hash")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing hash"))?;

                Ok(json!({
                    "description": format!("Troubleshooting for torrent {} on instance {}", hash, instance),
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": format!(
                                    "I have a torrent with hash '{}' on instance '{}' that is stalled or slow. \
                                     Please investigate it. Follow these steps:\n\
                                     1. Check the torrent properties using 'qbittorrent://{}/torrent/{}/properties'.\n\
                                     2. Check tracker status using 'qbittorrent://{}/torrent/{}/trackers'.\n\
                                     3. Check for specific file issues using 'qbittorrent://{}/torrent/{}/files'.\n\
                                     4. Look for global limits or mode using 'get_global_transfer_info' and 'get_speed_limits_mode'.\n\
                                     After investigating, suggest specific fixes (like re-announcing, toggling sequential download, or changing limits).",
                                    hash, instance, instance, hash, instance, hash, instance, hash
                                )
                            }
                        }
                    ]
                }))
            }
            "analyze_disk_space" => Ok(json!({
                "description": format!("Analyze disk space on instance {}", instance),
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": format!(
                                "I want to check if I have enough disk space for my downloads on instance '{}'. \
                                 Please check the current free space on disk and compare it with the total size of active/downloading torrents. \
                                 You can get global transfer info and list all torrents to calculate the required space.",
                                instance
                            )
                        }
                    }
                ]
            })),
            "optimize_speed" => Ok(json!({
                "description": format!("Optimize download speeds on instance {}", instance),
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": format!(
                                "My downloads are slow on instance '{}'. Please analyze my current global limits, \
                                 alternative speed limits mode, and connection status (firewalled state, DHT nodes) to suggest optimizations.",
                                instance
                            )
                        }
                    }
                ]
            })),
            "troubleshoot_connection" => Ok(json!({
                "description": format!("Troubleshoot connection issues on instance {}", instance),
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": format!(
                                "I think I have connection issues on instance '{}'. Please check my DHT node count and connection status, \
                                 and verify if alternative speed limits are accidentally enabled.",
                                instance
                            )
                        }
                    }
                ]
            })),
            "rules-of-engagement" => Ok(json!({
                "description": "Rules of Engagement for qBittorrent MCP",
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": "Please provide the Rules of Engagement for this MCP server."
                        }
                    },
                    {
                        "role": "assistant",
                        "content": {
                            "type": "text",
                            "text": "As an AI agent interacting with the qBittorrent MCP server, you must adhere to the following Rules of Engagement:\n\n\
                                     1. **State Verification**: Always verify the current state of a torrent (via `list_torrents` or resources) before performing actions like pause, resume, or delete.\n\
                                     2. **Destructive Actions**: Clearly inform the user and obtain confirmation before calling `delete_torrent` or `shutdown_app`. For these \"destructive\" actions, use the `destructiveHint` annotation or require a separate confirmation step.\n\
                                     3. **Search Etiquette**: Search is asynchronous. Use `get_search_results` for polling and always call `stop_search` once finished to save resources.\n\
                                     4. **Error Handling**: Treat errors as information for self-correction. Return helpful hints and use `isError: true` to prevent hallucination.\n\
                                     5. **Idempotency**: Avoid redundant commands (e.g., do not pause an already paused torrent).\n\
                                     6. **Semantic Feedback**: Translate technical tool results into meaningful context for the user.\n\
                                     7. **Security**: Never expose sensitive credentials or session cookies in logs or to the user."
                        }
                    }
                ]
            })),
            _ => anyhow::bail!("Prompt not found: {}", name),
        }
    }

    fn get_tool_definitions(&self) -> Vec<Value> {
        let state = self.state.lock().unwrap();
        if state.lazy_mode && !state.tools_loaded {
            return vec![
                json!({
                    "name": "list_torrents",
                    "description": "List all torrents with optional filtering and sorting",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "instance": { "type": "string", "description": "Optional: Name of the qBittorrent instance" },
                            "filter": { "type": "string", "description": "Filter by status (all, downloading, completed, paused, active, inactive, resumed, stalled, stalled_uploading, stalled_downloading, errored)" },
                            "category": { "type": "string", "description": "Filter by category" },
                            "tag": { "type": "string", "description": "Filter by tag" },
                            "sort": { "type": "string", "description": "Sort by column name" },
                            "reverse": { "type": "boolean", "description": "True to reverse sort order" },
                            "limit": { "type": "integer", "description": "Maximum number of torrents to return" },
                            "offset": { "type": "integer", "description": "Number of torrents to skip" }
                        },
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

        // Inject instance parameter into all tools
        tools
            .into_iter()
            .map(|mut t| {
                let props = t
                    .get_mut("inputSchema")
                    .and_then(|s| s.get_mut("properties"));

                if let Some(obj) = props.and_then(|p| p.as_object_mut()) {
                    obj.insert(
                        "instance".to_string(),
                        json!({ "type": "string", "description": "Optional: Name of the qBittorrent instance to target" }),
                    );
                }
                t
            })
            .collect()
    }

    fn get_torrent_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "list_torrents",
                "description": "List all torrents with optional filtering and sorting",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "filter": { "type": "string", "description": "Filter by status (all, downloading, completed, paused, active, inactive, resumed, stalled, stalled_uploading, stalled_downloading, errored)" },
                        "category": { "type": "string", "description": "Filter by category" },
                        "tag": { "type": "string", "description": "Filter by tag" },
                        "sort": { "type": "string", "description": "Sort by column name (e.g., name, size, progress, added_on, dlspeed, upspeed, ratio, eta, state, category, tags)" },
                        "reverse": { "type": "boolean", "description": "True to reverse sort order" },
                        "limit": { "type": "integer", "description": "Maximum number of torrents to return" },
                        "offset": { "type": "integer", "description": "Number of torrents to skip" }
                    },
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
                "description": "Pause a torrent. ALWAYS verify torrent exists and its current state via list_torrents before calling this.",
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
                "description": "Resume a torrent. ALWAYS verify torrent exists and its current state via list_torrents before calling this.",
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
                "description": "Delete a torrent. DESTRUCTIVE: Inform the user and confirm before calling, especially if delete_files is true.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash (pipe-separated for multiple)" },
                        "delete_files": { "type": "boolean", "description": "Also delete files from disk" }
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
            json!({
                "name": "cleanup_completed",
                "description": "Bulk remove completed torrents based on ratio or age. DESTRUCTIVE: Inform the user and confirm before calling.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "min_ratio": { "type": "number", "description": "Minimum ratio to trigger removal" },
                        "max_age_days": { "type": "integer", "description": "Maximum age in days since completion to trigger removal" },
                        "delete_files": { "type": "boolean", "description": "Also delete downloaded files from disk" }
                    },
                    "required": ["delete_files"]
                }
            }),
            json!({
                "name": "mass_rename",
                "description": "Rename files in a torrent using a regex pattern",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash" },
                        "pattern": { "type": "string", "description": "Regex pattern to match" },
                        "replacement": { "type": "string", "description": "Replacement string (supports $1, $2, etc.)" }
                    },
                    "required": ["hash", "pattern", "replacement"]
                }
            }),
            json!({
                "name": "find_duplicates",
                "description": "Find duplicate torrents by name",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "set_torrent_share_limits",
                "description": "Set share limits for specific torrents",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" },
                        "ratio_limit": { "type": "number", "description": "Ratio limit (-2 for global, -1 for unlimited)" },
                        "seeding_time_limit": { "type": "integer", "description": "Seeding time limit in minutes (-2 for global, -1 for unlimited)" },
                        "inactive_seeding_time_limit": { "type": "integer", "description": "Inactive seeding time limit in minutes (-2 for global, -1 for unlimited)" }
                    },
                    "required": ["hashes", "ratio_limit", "seeding_time_limit"]
                }
            }),
            json!({
                "name": "set_torrent_speed_limits",
                "description": "Set download and/or upload limits for specific torrents",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" },
                        "dl_limit": { "type": "integer", "description": "Download limit in bytes per second (0 for unlimited)" },
                        "up_limit": { "type": "integer", "description": "Upload limit in bytes per second (0 for unlimited)" }
                    },
                    "required": ["hashes"]
                }
            }),
            json!({
                "name": "toggle_sequential_download",
                "description": "Toggle sequential download for torrents",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" }
                    },
                    "required": ["hashes"]
                }
            }),
            json!({
                "name": "toggle_first_last_piece_priority",
                "description": "Toggle first/last piece priority for torrents",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" }
                    },
                    "required": ["hashes"]
                }
            }),
            json!({
                "name": "set_force_start",
                "description": "Set force start for torrents",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" },
                        "value": { "type": "boolean", "description": "True to force start, False otherwise" }
                    },
                    "required": ["hashes", "value"]
                }
            }),
            json!({
                "name": "set_super_seeding",
                "description": "Set super seeding for torrents",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" },
                        "value": { "type": "boolean", "description": "True to enable super seeding, False otherwise" }
                    },
                    "required": ["hashes", "value"]
                }
            }),
            json!({
                "name": "add_trackers",
                "description": "Add trackers to torrents",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" },
                        "urls": { "type": "string", "description": "URLs of the trackers (newline-separated)" }
                    },
                    "required": ["hashes", "urls"]
                }
            }),
            json!({
                "name": "edit_tracker",
                "description": "Edit a tracker URL for a torrent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash" },
                        "orig_url": { "type": "string", "description": "Original tracker URL" },
                        "new_url": { "type": "string", "description": "New tracker URL" }
                    },
                    "required": ["hash", "orig_url", "new_url"]
                }
            }),
            json!({
                "name": "remove_trackers",
                "description": "Remove trackers from torrents",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" },
                        "urls": { "type": "string", "description": "URLs of the trackers to remove (newline-separated)" }
                    },
                    "required": ["hashes", "urls"]
                }
            }),
            json!({
                "name": "rename_folder",
                "description": "Rename a folder in a torrent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash" },
                        "old_path": { "type": "string", "description": "Current folder path" },
                        "new_path": { "type": "string", "description": "New folder path" }
                    },
                    "required": ["hash", "old_path", "new_path"]
                }
            }),
            json!({
                "name": "set_file_priority",
                "description": "Set priority for files in a torrent",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash" },
                        "id": { "type": "string", "description": "File IDs (pipe-separated)" },
                        "priority": { "type": "integer", "description": "Priority (0: Do not download, 1: Normal, 6: High, 7: Maximal)" }
                    },
                    "required": ["hash", "id", "priority"]
                }
            }),
            json!({
                "name": "remove_categories",
                "description": "Remove one or more categories",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "categories": { "type": "string", "description": "Category names (newline-separated)" }
                    },
                    "required": ["categories"]
                }
            }),
            json!({
                "name": "remove_tags",
                "description": "Remove tags from torrents",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" },
                        "tags": { "type": "string", "description": "Tags to remove (comma-separated)" }
                    },
                    "required": ["hashes", "tags"]
                }
            }),
            json!({
                "name": "create_tags",
                "description": "Create one or more tags",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tags": { "type": "string", "description": "Tags to create (comma-separated)" }
                    },
                    "required": ["tags"]
                }
            }),
            json!({
                "name": "delete_tags",
                "description": "Delete one or more tags",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tags": { "type": "string", "description": "Tags to delete (comma-separated)" }
                    },
                    "required": ["tags"]
                }
            }),
        ]
    }

    fn get_search_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "search_torrents",
                "description": "Search for torrents. ASYNCHRONOUS: Results might be incomplete on the first call. Use get_search_results for polling if needed.",
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
            json!({
                "name": "move_rss_item",
                "description": "Move an RSS item (feed or folder)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "item_path": { "type": "string", "description": "Current path of the item" },
                        "dest_path": { "type": "string", "description": "Destination path" }
                    },
                    "required": ["item_path", "dest_path"]
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
                "description": "Shutdown qBittorrent. DESTRUCTIVE: Inform the user and confirm before calling as this terminates the service.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
        ]
    }

    pub async fn call_tool(&self, name: &str, args: &Value) -> Result<Value> {
        if name == "show_all_tools" {
            return self.handle_show_all_tools();
        }

        let instance = args.get("instance").and_then(|v| v.as_str());
        let client = self.get_client(instance)?;

        match name {
            // Torrent Management
            "list_torrents" => self.handle_list_torrents(client, args).await,
            "add_torrent" => self.handle_add_torrent(client, args).await,
            "pause_torrent" => self.handle_pause_torrent(client, args).await,
            "resume_torrent" => self.handle_resume_torrent(client, args).await,
            "delete_torrent" => self.handle_delete_torrent(client, args).await,
            "reannounce_torrent" => self.handle_reannounce_torrent(client, args).await,
            "recheck_torrent" => self.handle_recheck_torrent(client, args).await,
            "get_torrent_files" => self.handle_get_torrent_files(client, args).await,
            "get_torrent_properties" => self.handle_get_torrent_properties(client, args).await,
            "create_category" => self.handle_create_category(client, args).await,
            "set_torrent_category" => self.handle_set_torrent_category(client, args).await,
            "get_categories" => self.handle_get_categories(client).await,
            "add_torrent_tags" => self.handle_add_torrent_tags(client, args).await,
            "wait_for_torrent_status" => self.handle_wait_for_torrent_status(client, args).await,
            "cleanup_completed" => self.handle_cleanup_completed(client, args).await,
            "mass_rename" => self.handle_mass_rename(client, args).await,
            "find_duplicates" => self.handle_find_duplicates(client).await,
            "set_torrent_share_limits" => self.handle_set_torrent_share_limits(client, args).await,
            "set_torrent_speed_limits" => self.handle_set_torrent_speed_limits(client, args).await,
            "toggle_sequential_download" => {
                self.handle_toggle_sequential_download(client, args).await
            }
            "toggle_first_last_piece_priority" => {
                self.handle_toggle_first_last_piece_priority(client, args)
                    .await
            }
            "set_force_start" => self.handle_set_force_start(client, args).await,
            "set_super_seeding" => self.handle_set_super_seeding(client, args).await,
            "add_trackers" => self.handle_add_trackers(client, args).await,
            "edit_tracker" => self.handle_edit_tracker(client, args).await,
            "remove_trackers" => self.handle_remove_trackers(client, args).await,
            "rename_folder" => self.handle_rename_folder(client, args).await,
            "set_file_priority" => self.handle_set_file_priority(client, args).await,
            "remove_categories" => self.handle_remove_categories(client, args).await,
            "remove_tags" => self.handle_remove_tags(client, args).await,
            "create_tags" => self.handle_create_tags(client, args).await,
            "delete_tags" => self.handle_delete_tags(client, args).await,

            // Search
            "search_torrents" => self.handle_search_torrents(client, args).await,
            "install_search_plugin" => self.handle_install_search_plugin(client, args).await,
            "uninstall_search_plugin" => self.handle_uninstall_search_plugin(client, args).await,
            "enable_search_plugin" => self.handle_enable_search_plugin(client, args).await,
            "update_search_plugins" => self.handle_update_search_plugins(client).await,
            "get_search_plugins" => self.handle_get_search_plugins(client).await,

            // RSS
            "add_rss_feed" => self.handle_add_rss_feed(client, args).await,
            "get_rss_feeds" => self.handle_get_rss_feeds(client).await,
            "set_rss_rule" => self.handle_set_rss_rule(client, args).await,
            "get_rss_rules" => self.handle_get_rss_rules(client).await,
            "move_rss_item" => self.handle_move_rss_item(client, args).await,

            // Transfer / App
            "get_global_transfer_info" => self.handle_get_global_transfer_info(client).await,
            "set_global_transfer_limits" => {
                self.handle_set_global_transfer_limits(client, args).await
            }
            "toggle_alternative_speed_limits" => {
                self.handle_toggle_alternative_speed_limits(client).await
            }
            "get_speed_limits_mode" => self.handle_get_speed_limits_mode(client).await,
            "ban_peers" => self.handle_ban_peers(client, args).await,
            "get_app_preferences" => self.handle_get_app_preferences(client).await,
            "set_app_preferences" => self.handle_set_app_preferences(client, args).await,
            "get_main_log" => self.handle_get_main_log(client, args).await,
            "get_peer_log" => self.handle_get_peer_log(client, args).await,
            "get_app_version" => self.handle_get_app_version(client).await,
            "get_build_info" => self.handle_get_build_info(client).await,
            "shutdown_app" => self.handle_shutdown_app(client).await,

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

    async fn handle_list_torrents(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let filter = args.get("filter").and_then(|v| v.as_str());
        let category = args.get("category").and_then(|v| v.as_str());
        let tag = args.get("tag").and_then(|v| v.as_str());
        let sort = args.get("sort").and_then(|v| v.as_str());
        let reverse = args.get("reverse").and_then(|v| v.as_bool());
        let limit = args.get("limit").and_then(|v| v.as_i64());
        let offset = args.get("offset").and_then(|v| v.as_i64());

        let torrents = client
            .get_torrent_list(filter, category, tag, sort, reverse, limit, offset)
            .await?;
        let text = serde_json::to_string_pretty(&torrents)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_search_torrents(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing query"))?;
        let category = args.get("category").and_then(|v| v.as_str());

        let id = client.start_search(query, category).await?;

        let mut final_results = Vec::new();
        for _ in 0..5 {
            sleep(Duration::from_secs(1)).await;
            let resp = client.get_search_results(id, None, None).await;
            if let Ok(r) = resp {
                if r.status == "Stopped" {
                    final_results = r.results;
                    break;
                }
                final_results = r.results;
            }
        }
        let _ = client.stop_search(id).await;
        let _ = client.delete_search(id).await;
        let text = serde_json::to_string_pretty(&final_results)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_add_torrent(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing url"))?;
        let save_path = args.get("save_path").and_then(|v| v.as_str());
        let category = args.get("category").and_then(|v| v.as_str());
        client.add_torrent(url, save_path, category).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Torrent added successfully" }] }))
    }

    async fn handle_pause_torrent(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        client.pause_torrents(hash).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Torrent paused successfully" }] }))
    }

    async fn handle_resume_torrent(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        client.resume_torrents(hash).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Torrent resumed successfully" }] }))
    }

    async fn handle_delete_torrent(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        let delete_files = args
            .get("delete_files")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        client.delete_torrents(hash, delete_files).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Torrent deleted successfully" }] }))
    }

    async fn handle_reannounce_torrent(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        client.reannounce_torrents(hash).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Torrent reannounced successfully" }] }))
    }

    async fn handle_recheck_torrent(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        client.recheck_torrents(hash).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Torrent recheck started successfully" }] }),
        )
    }

    async fn handle_get_torrent_files(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        let files = client.get_torrent_files(hash).await?;
        let text = serde_json::to_string_pretty(&files)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_get_torrent_properties(
        &self,
        client: &QBitClient,
        args: &Value,
    ) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        let props = client.get_torrent_properties(hash).await?;
        let text = serde_json::to_string_pretty(&props)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_get_global_transfer_info(&self, client: &QBitClient) -> Result<Value> {
        let info = client.get_global_transfer_info().await?;
        let text = serde_json::to_string_pretty(&info)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_set_global_transfer_limits(
        &self,
        client: &QBitClient,
        args: &Value,
    ) -> Result<Value> {
        if let Some(limit) = args.get("dl_limit").and_then(|v| v.as_i64()) {
            client.set_download_limit(limit).await?;
        }
        if let Some(limit) = args.get("up_limit").and_then(|v| v.as_i64()) {
            client.set_upload_limit(limit).await?;
        }
        Ok(
            json!({ "content": [{ "type": "text", "text": "Transfer limits updated successfully" }] }),
        )
    }

    async fn handle_toggle_alternative_speed_limits(&self, client: &QBitClient) -> Result<Value> {
        client.toggle_alternative_speed_limits().await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Alternative speed limits toggled" }] }))
    }

    async fn handle_get_speed_limits_mode(&self, client: &QBitClient) -> Result<Value> {
        let mode = client.get_speed_limits_mode().await?;
        Ok(json!({ "content": [{ "type": "text", "text": mode.to_string() }] }))
    }

    async fn handle_ban_peers(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let peers = args
            .get("peers")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing peers"))?;
        client.ban_peers(peers).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Peers banned successfully" }] }))
    }

    async fn handle_create_category(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let save_path = args
            .get("save_path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing save_path"))?;
        client.create_category(name, save_path).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Category created successfully" }] }))
    }

    async fn handle_set_torrent_category(
        &self,
        client: &QBitClient,
        args: &Value,
    ) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        let category = args
            .get("category")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing category"))?;
        client.set_category(hashes, category).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Category set successfully" }] }))
    }

    async fn handle_get_categories(&self, client: &QBitClient) -> Result<Value> {
        let categories = client.get_categories().await?;
        let text = serde_json::to_string_pretty(&categories)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_add_torrent_tags(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        let tags = args
            .get("tags")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing tags"))?;
        client.add_tags(hashes, tags).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Tags added successfully" }] }))
    }

    async fn handle_install_search_plugin(
        &self,
        client: &QBitClient,
        args: &Value,
    ) -> Result<Value> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing url"))?;
        client.install_search_plugin(url).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Search plugin installed successfully" }] }),
        )
    }

    async fn handle_uninstall_search_plugin(
        &self,
        client: &QBitClient,
        args: &Value,
    ) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        client.uninstall_search_plugin(name).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Search plugin uninstalled successfully" }] }),
        )
    }

    async fn handle_enable_search_plugin(
        &self,
        client: &QBitClient,
        args: &Value,
    ) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let enable = args
            .get("enable")
            .and_then(|v| v.as_bool())
            .ok_or(anyhow::anyhow!("Missing enable"))?;
        client.enable_search_plugin(name, enable).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Search plugin status updated successfully" }] }),
        )
    }

    async fn handle_update_search_plugins(&self, client: &QBitClient) -> Result<Value> {
        client.update_search_plugins().await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Search plugins updated successfully" }] }),
        )
    }

    async fn handle_get_search_plugins(&self, client: &QBitClient) -> Result<Value> {
        let plugins = client.get_search_plugins().await?;
        let text = serde_json::to_string_pretty(&plugins)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_add_rss_feed(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing url"))?;
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing path"))?;
        client.add_rss_feed(url, path).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "RSS feed added successfully" }] }))
    }

    async fn handle_get_rss_feeds(&self, client: &QBitClient) -> Result<Value> {
        let feeds = client.get_all_rss_feeds().await?;
        let text = serde_json::to_string_pretty(&feeds)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_set_rss_rule(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing name"))?;
        let definition = args
            .get("definition")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing definition"))?;
        client.set_rss_rule(name, definition).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "RSS rule set successfully" }] }))
    }

    async fn handle_get_rss_rules(&self, client: &QBitClient) -> Result<Value> {
        let rules = client.get_all_rss_rules().await?;
        let text = serde_json::to_string_pretty(&rules)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_get_app_preferences(&self, client: &QBitClient) -> Result<Value> {
        let prefs = client.get_app_preferences().await?;
        let text = serde_json::to_string_pretty(&prefs)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_set_app_preferences(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let prefs_str = args
            .get("preferences")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing preferences"))?;
        let prefs_val: serde_json::Value = serde_json::from_str(prefs_str)?;
        client.set_app_preferences(&prefs_val).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "App preferences updated successfully" }] }),
        )
    }

    async fn handle_get_main_log(&self, client: &QBitClient, args: &Value) -> Result<Value> {
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
        let logs = client
            .get_main_log(normal, info, warning, critical, last_id)
            .await?;
        let text = serde_json::to_string_pretty(&logs)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_get_peer_log(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let last_id = args.get("last_id").and_then(|v| v.as_i64());
        let logs = client.get_peer_log(last_id).await?;
        let text = serde_json::to_string_pretty(&logs)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_get_app_version(&self, client: &QBitClient) -> Result<Value> {
        let version = client.get_app_version().await?;
        Ok(json!({ "content": [{ "type": "text", "text": version }] }))
    }

    async fn handle_get_build_info(&self, client: &QBitClient) -> Result<Value> {
        let info = client.get_build_info().await?;
        let text = serde_json::to_string_pretty(&info)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_shutdown_app(&self, client: &QBitClient) -> Result<Value> {
        client.shutdown_app().await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Shutdown command sent" }] }))
    }

    async fn handle_wait_for_torrent_status(
        &self,
        client: &QBitClient,
        args: &Value,
    ) -> Result<Value> {
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
            let torrents = client.get_torrents_info(hash).await?;
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

    async fn handle_cleanup_completed(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let min_ratio = args.get("min_ratio").and_then(|v| v.as_f64());
        let max_age_days = args.get("max_age_days").and_then(|v| v.as_i64());
        let delete_files = args
            .get("delete_files")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let torrents = client
            .get_torrent_list(Some("completed"), None, None, None, None, None, None)
            .await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let to_delete: Vec<String> = torrents
            .into_iter()
            .filter(|t| {
                let mut should_delete = false;
                if let Some(r) = min_ratio {
                    should_delete = t.ratio >= r;
                }
                if let Some(age_days) = max_age_days {
                    let age_secs = age_days * 24 * 3600;
                    if t.completion_on > 0 && (now - t.completion_on) >= age_secs {
                        should_delete = true;
                    }
                }
                // If neither is specified, we don't delete anything automatically
                // to avoid accidental wipes of all completed torrents.
                // UNLESS the user explicitly wants to delete all completed.
                // But let's require at least one condition for safety in this macro.
                if min_ratio.is_none() && max_age_days.is_none() {
                    return false;
                }
                should_delete
            })
            .map(|t| t.hash)
            .collect();

        if to_delete.is_empty() {
            return Ok(
                json!({ "content": [{ "type": "text", "text": "No torrents matched the cleanup criteria." }] }),
            );
        }

        let count = to_delete.len();
        let hashes = to_delete.join("|");
        client.delete_torrents(&hashes, delete_files).await?;

        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Successfully cleaned up {} torrents.", count) }] }),
        )
    }

    async fn handle_mass_rename(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing pattern"))?;
        let replacement = args
            .get("replacement")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing replacement"))?;

        let re = Regex::new(pattern)?;
        let files = client.get_torrent_files(hash).await?;

        let mut rename_count = 0;
        for file in files {
            if re.is_match(&file.name) {
                let new_name = re.replace_all(&file.name, replacement).to_string();
                if new_name != file.name {
                    client.rename_file(hash, &file.name, &new_name).await?;
                    rename_count += 1;
                }
            }
        }

        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Successfully renamed {} files.", rename_count) }] }),
        )
    }

    async fn handle_set_torrent_share_limits(
        &self,
        client: &QBitClient,
        args: &Value,
    ) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        let ratio_limit = args
            .get("ratio_limit")
            .and_then(|v| v.as_f64())
            .ok_or(anyhow::anyhow!("Missing ratio_limit"))?;
        let seeding_time_limit = args
            .get("seeding_time_limit")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing seeding_time_limit"))?;
        let inactive_seeding_time_limit = args
            .get("inactive_seeding_time_limit")
            .and_then(|v| v.as_i64());

        client
            .set_torrent_share_limits(
                hashes,
                ratio_limit,
                seeding_time_limit,
                inactive_seeding_time_limit,
            )
            .await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Torrent share limits updated successfully" }] }),
        )
    }

    async fn handle_set_torrent_speed_limits(
        &self,
        client: &QBitClient,
        args: &Value,
    ) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;

        if let Some(limit) = args.get("dl_limit").and_then(|v| v.as_i64()) {
            client.set_torrent_download_limit(hashes, limit).await?;
        }
        if let Some(limit) = args.get("up_limit").and_then(|v| v.as_i64()) {
            client.set_torrent_upload_limit(hashes, limit).await?;
        }

        Ok(
            json!({ "content": [{ "type": "text", "text": "Torrent speed limits updated successfully" }] }),
        )
    }

    async fn handle_find_duplicates(&self, client: &QBitClient) -> Result<Value> {
        let torrents = client
            .get_torrent_list(None, None, None, None, None, None, None)
            .await?;

        let mut names: std::collections::HashMap<String, Vec<crate::models::Torrent>> =
            std::collections::HashMap::new();

        for t in torrents {
            names.entry(t.name.clone()).or_default().push(t);
        }

        let duplicates: Vec<Value> = names
            .into_iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|(name, v)| {
                json!({
                    "name": name,
                    "count": v.len(),
                    "torrents": v.into_iter().map(|t| json!({
                        "hash": t.hash,
                        "size": t.size_bytes,
                        "progress": t.progress,
                        "state": t.state
                    })).collect::<Vec<Value>>()
                })
            })
            .collect();

        if duplicates.is_empty() {
            return Ok(
                json!({ "content": [{ "type": "text", "text": "No duplicate torrents found." }] }),
            );
        }

        let text = serde_json::to_string_pretty(&duplicates)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_toggle_sequential_download(
        &self,
        client: &QBitClient,
        args: &Value,
    ) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        client.toggle_sequential_download(hashes).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Sequential download toggled successfully" }] }),
        )
    }

    async fn handle_toggle_first_last_piece_priority(
        &self,
        client: &QBitClient,
        args: &Value,
    ) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        client.toggle_first_last_piece_priority(hashes).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "First/last piece priority toggled successfully" }] }),
        )
    }

    async fn handle_set_force_start(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        let value = args
            .get("value")
            .and_then(|v| v.as_bool())
            .ok_or(anyhow::anyhow!("Missing value"))?;
        client.set_force_start(hashes, value).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Force start status updated successfully" }] }),
        )
    }

    async fn handle_set_super_seeding(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        let value = args
            .get("value")
            .and_then(|v| v.as_bool())
            .ok_or(anyhow::anyhow!("Missing value"))?;
        client.set_super_seeding(hashes, value).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Super seeding status updated successfully" }] }),
        )
    }

    async fn handle_add_trackers(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        let urls = args
            .get("urls")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing urls"))?;
        client.add_trackers(hashes, urls).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Trackers added successfully" }] }))
    }

    async fn handle_edit_tracker(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        let orig_url = args
            .get("orig_url")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing orig_url"))?;
        let new_url = args
            .get("new_url")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing new_url"))?;
        client.edit_tracker(hash, orig_url, new_url).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Tracker edited successfully" }] }))
    }

    async fn handle_remove_trackers(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        let urls = args
            .get("urls")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing urls"))?;
        client.remove_trackers(hashes, urls).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Trackers removed successfully" }] }))
    }

    async fn handle_rename_folder(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        let old_path = args
            .get("old_path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing old_path"))?;
        let new_path = args
            .get("new_path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing new_path"))?;
        client.rename_folder(hash, old_path, new_path).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Folder renamed successfully" }] }))
    }

    async fn handle_set_file_priority(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing id"))?;
        let priority = args
            .get("priority")
            .and_then(|v| v.as_i64())
            .ok_or(anyhow::anyhow!("Missing priority"))? as i32;
        client.set_file_priority(hash, id, priority).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "File priority updated successfully" }] }))
    }

    async fn handle_remove_categories(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let categories = args
            .get("categories")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing categories"))?;
        client.remove_categories(categories).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Categories removed successfully" }] }))
    }

    async fn handle_remove_tags(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        let tags = args
            .get("tags")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing tags"))?;
        client.remove_tags(hashes, tags).await?;
        Ok(
            json!({ "content": [{ "type": "text", "text": "Tags removed from torrents successfully" }] }),
        )
    }

    async fn handle_create_tags(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let tags = args
            .get("tags")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing tags"))?;
        client.create_tags(tags).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Tags created successfully" }] }))
    }

    async fn handle_delete_tags(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let tags = args
            .get("tags")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing tags"))?;
        client.delete_tags(tags).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Tags deleted successfully" }] }))
    }

    async fn handle_move_rss_item(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let item_path = args
            .get("item_path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing item_path"))?;
        let dest_path = args
            .get("dest_path")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing dest_path"))?;
        client.move_rss_item(item_path, dest_path).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "RSS item moved successfully" }] }))
    }

    pub fn start_event_loop(&self, interval_ms: u64) {
        let server = self.clone();
        tokio::spawn(async move {
            server.event_loop(interval_ms).await;
        });
    }

    async fn event_loop(&self, interval_ms: u64) {
        let mut last_rids: HashMap<String, i64> = HashMap::new();
        let mut notified_finished: HashMap<String, std::collections::HashSet<String>> =
            HashMap::new();

        for name in self.clients.keys() {
            last_rids.insert(name.clone(), 0);
            notified_finished.insert(name.clone(), std::collections::HashSet::new());
        }

        loop {
            sleep(Duration::from_millis(interval_ms)).await;
            for (name, client) in &self.clients {
                let rid = *last_rids.get(name).unwrap_or(&0);
                match client.get_main_data(rid).await {
                    Ok(data) => {
                        last_rids.insert(name.clone(), data.rid);

                        // Track finished torrents to notify only once
                        if let Some(torrents) = data.torrents {
                            for (hash, torrent_val) in torrents {
                                let progress = torrent_val.get("progress").and_then(|p| p.as_f64());
                                let state = torrent_val.get("state").and_then(|s| s.as_str());

                                // "uploading", "stalledUP", "queuedUP", "forcedUP" usually mean finished downloading
                                let is_finished_state = state.is_some_and(|s| {
                                    s == "uploading"
                                        || s == "stalledUP"
                                        || s == "queuedUP"
                                        || s == "forcedUP"
                                });

                                if progress.is_some_and(|p| p >= 1.0) || is_finished_state {
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

                                        // Custom notification
                                        self.push_notification(
                                            "notifications/torrent_finished",
                                            json!({
                                                "instance": name,
                                                "hash": hash,
                                                "name": torrent_name
                                            }),
                                        );

                                        // Standard resource update notification
                                        self.push_notification(
                                                    "notifications/resources/updated",
                                                    json!({ "uri": format!("qbittorrent://{}/torrents", name) }),
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rules_of_engagement_prompt() {
        let client = QBitClient::new("http://localhost:8080", "admin", "adminadmin", false);
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let server = McpServer::new(clients, false);

        // 1. Verify prompt is listed
        let list_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "prompts/list".to_string(),
            params: None,
            id: Some(json!(1)),
        };
        let list_resp = server.handle_request(list_req).await.unwrap();
        let prompts = list_resp.get("prompts").unwrap().as_array().unwrap();
        let rules_prompt = prompts.iter().find(|p| p["name"] == "rules-of-engagement");
        assert!(rules_prompt.is_some());

        // 2. Verify prompt content
        let get_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "prompts/get".to_string(),
            params: Some(json!({ "name": "rules-of-engagement" })),
            id: Some(json!(2)),
        };
        let get_resp = server.handle_request(get_req).await.unwrap();
        let messages = get_resp.get("messages").unwrap().as_array().unwrap();

        // Check for the rules message
        let rules_msg = messages.iter().find(|m| m["role"] == "assistant").unwrap();
        let text = rules_msg["content"]["text"].as_str().unwrap();

        assert!(text.contains("State Verification"));
        assert!(text.contains("Destructive Actions"));
        assert!(text.contains("Search Etiquette"));
        assert!(text.contains("Error Handling"));
        assert!(text.contains("Idempotency"));
        assert!(text.contains("Semantic Feedback"));
        assert!(text.contains("Security"));
    }

    #[tokio::test]
    async fn test_tool_call_routing() {
        let client = QBitClient::new("http://localhost:8080", "admin", "adminadmin", false);
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let server = McpServer::new(clients, false);

        // Test all available tools to verify routing logic
        let tools_to_test = vec![
            ("list_torrents", json!({})),
            ("add_torrent", json!({ "url": "magnet:?xt=urn:btih:..." })),
            ("pause_torrent", json!({ "hash": "abc" })),
            ("resume_torrent", json!({ "hash": "abc" })),
            (
                "delete_torrent",
                json!({ "hash": "abc", "delete_files": false }),
            ),
            ("reannounce_torrent", json!({ "hash": "abc" })),
            ("recheck_torrent", json!({ "hash": "abc" })),
            ("get_torrent_files", json!({ "hash": "abc" })),
            ("get_torrent_properties", json!({ "hash": "abc" })),
            (
                "create_category",
                json!({ "name": "test", "save_path": "/tmp" }),
            ),
            (
                "set_torrent_category",
                json!({ "hashes": "abc", "category": "test" }),
            ),
            ("get_categories", json!({})),
            ("add_torrent_tags", json!({ "hashes": "abc", "tags": "t1" })),
            (
                "wait_for_torrent_status",
                json!({ "hash": "abc", "target_status": "downloading" }),
            ),
            ("cleanup_completed", json!({ "delete_files": false })),
            (
                "mass_rename",
                json!({ "hash": "abc", "pattern": ".*", "replacement": "new" }),
            ),
            ("find_duplicates", json!({})),
            (
                "set_torrent_share_limits",
                json!({ "hashes": "abc", "ratio_limit": 1.0, "seeding_time_limit": 60 }),
            ),
            ("set_torrent_speed_limits", json!({ "hashes": "abc" })),
            ("toggle_sequential_download", json!({ "hashes": "abc" })),
            (
                "toggle_first_last_piece_priority",
                json!({ "hashes": "abc" }),
            ),
            ("set_force_start", json!({ "hashes": "abc", "value": true })),
            (
                "set_super_seeding",
                json!({ "hashes": "abc", "value": true }),
            ),
            (
                "add_trackers",
                json!({ "hashes": "abc", "urls": "http://t.com" }),
            ),
            (
                "edit_tracker",
                json!({ "hash": "abc", "orig_url": "u1", "new_url": "u2" }),
            ),
            ("remove_trackers", json!({ "hashes": "abc", "urls": "u1" })),
            (
                "rename_folder",
                json!({ "hash": "abc", "old_path": "p1", "new_path": "p2" }),
            ),
            (
                "set_file_priority",
                json!({ "hash": "abc", "id": "0", "priority": 1 }),
            ),
            ("remove_categories", json!({ "categories": "c1" })),
            ("remove_tags", json!({ "hashes": "abc", "tags": "t1" })),
            ("create_tags", json!({ "tags": "t1" })),
            ("delete_tags", json!({ "tags": "t1" })),
            ("search_torrents", json!({ "query": "linux" })),
            ("install_search_plugin", json!({ "url": "http://p.com" })),
            ("uninstall_search_plugin", json!({ "name": "p1" })),
            (
                "enable_search_plugin",
                json!({ "name": "p1", "enable": true }),
            ),
            ("update_search_plugins", json!({})),
            ("get_search_plugins", json!({})),
            (
                "add_rss_feed",
                json!({ "url": "http://r.com", "path": "p1" }),
            ),
            ("get_rss_feeds", json!({})),
            ("set_rss_rule", json!({ "name": "r1", "definition": "{}" })),
            ("get_rss_rules", json!({})),
            (
                "move_rss_item",
                json!({ "item_path": "p1", "dest_path": "p2" }),
            ),
            ("get_global_transfer_info", json!({})),
            ("set_global_transfer_limits", json!({})),
            ("toggle_alternative_speed_limits", json!({})),
            ("get_speed_limits_mode", json!({})),
            ("ban_peers", json!({ "peers": "1.1.1.1:80" })),
            ("get_app_preferences", json!({})),
            ("set_app_preferences", json!({ "preferences": "{}" })),
            ("get_main_log", json!({})),
            ("get_peer_log", json!({})),
            ("get_app_version", json!({})),
            ("get_build_info", json!({})),
            ("shutdown_app", json!({})),
        ];

        for (name, args) in tools_to_test {
            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                method: "tools/call".to_string(),
                params: Some(json!({ "name": name, "arguments": args })),
                id: Some(json!(1)),
            };

            let resp = server.handle_request(req).await;
            if let Some(error) = resp.ok().and_then(|r| r.get("error").cloned()) {
                assert_ne!(
                    error["message"], "Method not found",
                    "Tool {} not found in routing",
                    name
                );
                assert_ne!(
                    error["message"],
                    format!("Unknown tool: {}", name),
                    "Tool {} not found in tool mapping",
                    name
                );
            }
        }
    }

    #[tokio::test]
    async fn test_resource_routing() {
        let client = QBitClient::new("http://localhost:8080", "admin", "adminadmin", false);
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let server = McpServer::new(clients, false);

        let uris = vec![
            "qbittorrent://default/torrents",
            "qbittorrent://default/transfer",
            "qbittorrent://default/categories",
        ];

        for uri in uris {
            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                method: "resources/read".to_string(),
                params: Some(json!({ "uri": uri })),
                id: Some(json!(1)),
            };
            let resp = server.handle_request(req).await.unwrap();
            if let Some(error) = resp.get("error") {
                assert_ne!(error["message"], format!("Resource not found: {}", uri));
            }
        }
    }

    #[tokio::test]
    async fn test_event_loop_init() {
        let client = QBitClient::new("http://localhost:8080", "admin", "adminadmin", false);
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let server = McpServer::new(clients, false);

        // Test start_event_loop doesn't panic
        server.start_event_loop(10);
        // Give it a tiny bit of time to run
        sleep(Duration::from_millis(50)).await;
    }

    #[test]
    fn test_push_notification() {
        let clients = HashMap::new();
        let server = McpServer::new(clients, false);
        server.push_notification("test_method", json!({"param": "val"}));

        let state = server.state.lock().unwrap();
        assert_eq!(state.notification_queue.len(), 1);
        assert_eq!(state.notification_queue[0]["method"], "test_method");
    }

    #[tokio::test]
    async fn test_handle_request_errors() {
        let clients = HashMap::new();
        let server = McpServer::new(clients, false);

        // Unknown method
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "unknown_method".to_string(),
            params: None,
            id: Some(json!(1)),
        };
        let result = server.handle_request(req).await;
        assert!(result.is_err());

        // Missing params for prompts/get
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "prompts/get".to_string(),
            params: None,
            id: Some(json!(1)),
        };
        let result = server.handle_request(req).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_resource_definitions() {
        let mut clients = HashMap::new();
        clients.insert(
            "test".to_string(),
            QBitClient::new("http://localhost", "a", "b", false),
        );
        let server = McpServer::new(clients, false);
        let res = server.get_resource_definitions();
        assert!(!res.is_empty());
        // Should have qbittorrent://test/torrents etc.
        let found = res
            .iter()
            .any(|r| r["uri"] == "qbittorrent://test/torrents");
        assert!(found);
    }
}
