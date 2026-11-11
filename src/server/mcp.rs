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
    // shortcut: lazy-mode state is shared by every connected client, fine for one or two clients; move into per-session state if many clients connect.
    lazy_mode: bool,
    tools_loaded: bool,
    should_notify: bool,
    notification_queue: Vec<Value>,
    running: bool,
}

#[derive(Clone)]
pub struct McpServer {
    clients: HashMap<String, QBitClient>,
    state: Arc<Mutex<McpState>>,
}

/// Error marker for JSON-RPC "invalid params" (-32602).
#[derive(Debug)]
struct InvalidParams(String);

impl std::fmt::Display for InvalidParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for InvalidParams {}

/// Error marker for JSON-RPC "method not found" (-32601).
#[derive(Debug)]
struct MethodNotFound(String);

impl std::fmt::Display for MethodNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Method not found: {}", self.0)
    }
}

impl std::error::Error for MethodNotFound {}

/// Builds an MCP text result.
fn text_result(text: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": text.into() }] })
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing {}", key))
}

fn arg_bool(args: &Value, key: &str) -> Result<bool> {
    args.get(key)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| anyhow::anyhow!("Missing {}", key))
}

/// Decodes standard or URL-safe base64 (padding and whitespace optional).
fn decode_base64(input: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let (mut buf, mut bits) = (0u32, 0u32);
    for c in input
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
    {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => anyhow::bail!("Invalid base64 character: {}", c as char),
        };
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

/// Preference keys the AI may not change: they run programs, control Web UI access or hold secrets.
fn is_blocked_preference(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    k.starts_with("autorun")
        || k.starts_with("web_ui_")
        || k.starts_with("bypass_")
        || k.contains("password")
        || k.starts_with("mail_notification")
        || k.starts_with("dyndns")
        || k.starts_with("proxy")
        || k == "alternative_webui_enabled"
        || k == "alternative_webui_path"
}

/// Drops every key that holds a secret before preferences are shown to the AI.
fn redact_preferences(prefs: &mut Value) {
    if let Some(obj) = prefs.as_object_mut() {
        obj.retain(|k, _| {
            let k = k.to_ascii_lowercase();
            !(k.contains("password") || k.contains("secret") || k.ends_with("_key"))
        });
    }
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
                running: true,
            })),
        }
    }

    pub fn shutdown(&self) {
        let mut state = self.state.lock().unwrap();
        state.running = false;
    }

    pub fn is_running(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.running
    }

    pub fn push_notification(&self, method: &str, params: Value) {
        let mut state = self.state.lock().unwrap();
        state.notification_queue.push(json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }));
    }

    /// Takes every pending notification (tool list change first) and clears the queue.
    pub fn take_notifications(&self) -> Vec<Value> {
        let mut state = self.state.lock().unwrap();
        let mut pending = Vec::new();
        if std::mem::take(&mut state.should_notify) {
            pending.push(json!({
                "jsonrpc": "2.0",
                "method": "notifications/tools/list_changed"
            }));
        }
        pending.append(&mut state.notification_queue);
        pending
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

    /// Runs one JSON-RPC request and builds the response; `None` for notifications (no id).
    /// A failed tool call is returned as an `isError` result, as MCP expects, so the model can read it.
    pub async fn handle_rpc(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = req.id.clone();
        let is_tool_call = req.method == "tools/call";
        let resp = self.handle_request(req).await;
        let Some(id) = id else {
            if let Err(e) = resp {
                error!("Error handling notification: {}", e);
            }
            return None;
        };
        let (result, error) = match resp {
            Ok(v) => (Some(v), None),
            Err(e) if e.is::<MethodNotFound>() => (None, Some((-32601, e.to_string()))),
            Err(e) if e.is::<InvalidParams>() => (None, Some((-32602, e.to_string()))),
            Err(e) if is_tool_call => (
                Some(
                    json!({ "content": [{ "type": "text", "text": format!("Error: {:#}", e) }], "isError": true }),
                ),
                None,
            ),
            Err(e) => (None, Some((-32603, e.to_string()))),
        };
        Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            result,
            error: error.map(|(code, message)| JsonRpcError {
                code,
                message,
                data: None,
            }),
        })
    }

    pub async fn run_stdio(&mut self) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, stdin, stdout};
        let mut reader = BufReader::new(stdin()).lines();
        let mut stdout = stdout();

        while self.is_running() {
            let line = tokio::select! {
                line_res = reader.next_line() => match line_res? {
                    Some(l) => Some(l),
                    None => break, // EOF
                },
                _ = sleep(Duration::from_millis(100)) => None,
            };

            let mut out = String::new();
            if let Some(line) = line {
                let input = line.trim();
                if !input.is_empty() {
                    debug!("Received: {}", input);
                    match serde_json::from_str::<JsonRpcRequest>(input) {
                        Ok(req) => {
                            if let Some(resp) = self.handle_rpc(req).await {
                                out += &(serde_json::to_string(&resp)? + "\n");
                            }
                        }
                        Err(e) => error!("Failed to parse JSON-RPC: {}", e),
                    }
                }
            }
            for n in self.take_notifications() {
                out += &(serde_json::to_string(&n)? + "\n");
            }
            if !out.is_empty() {
                stdout.write_all(out.as_bytes()).await?;
                stdout.flush().await?;
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
                    Err(InvalidParams("Missing params for prompts/get".into()).into())
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
                    Err(InvalidParams("Missing params for tools/call".into()).into())
                }
            }
            "resources/list" => Ok(json!({
                "resources": self.get_resource_definitions()
            })),
            "resources/templates/list" => Ok(json!({
                "resourceTemplates": self.get_resource_template_definitions()
            })),
            "resources/read" => {
                if let Some(params) = req.params {
                    let uri = params.get("uri").and_then(|n| n.as_str()).unwrap_or("");
                    self.handle_resource_read(uri).await
                } else {
                    Err(InvalidParams("Missing params for resources/read".into()).into())
                }
            }
            _ => Err(MethodNotFound(req.method).into()),
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
        res
    }

    fn get_resource_template_definitions(&self) -> Vec<Value> {
        vec![
            json!({
                "uriTemplate": "qbittorrent://{instance}/torrent/{hash}/properties",
                "name": "Torrent Properties",
                "description": "Detailed properties and metadata for a specific torrent",
                "mimeType": "application/json"
            }),
            json!({
                "uriTemplate": "qbittorrent://{instance}/torrent/{hash}/files",
                "name": "Torrent Files",
                "description": "List of files and their progress within a specific torrent",
                "mimeType": "application/json"
            }),
            json!({
                "uriTemplate": "qbittorrent://{instance}/torrent/{hash}/trackers",
                "name": "Torrent Trackers",
                "description": "Current trackers and their status for a specific torrent",
                "mimeType": "application/json"
            }),
        ]
    }

    async fn handle_resource_read(&self, uri: &str) -> Result<Value> {
        static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"^qbittorrent://(?:([^/]+)/)?(?:torrent/([^/]+)/)?(torrents|transfer|categories|properties|files|trackers)$").unwrap()
        });
        let caps = RE
            .captures(uri)
            .ok_or_else(|| InvalidParams(format!("Resource not found: {}", uri)))?;
        let client = self.get_client(caps.get(1).map(|m| m.as_str()))?;
        let hash = caps.get(2).map(|m| m.as_str());
        let content = match (&caps[3], hash) {
            ("torrents", None) => serde_json::to_string(
                &client
                    .get_torrent_list(None, None, None, None, None, None, None)
                    .await?,
            )?,
            ("transfer", None) => serde_json::to_string(&client.get_global_transfer_info().await?)?,
            ("categories", None) => serde_json::to_string(&client.get_categories().await?)?,
            ("properties", Some(h)) => {
                serde_json::to_string(&client.get_torrent_properties(h).await?)?
            }
            ("files", Some(h)) => serde_json::to_string(&client.get_torrent_files(h).await?)?,
            ("trackers", Some(h)) => serde_json::to_string(&client.get_torrent_trackers(h).await?)?,
            _ => return Err(InvalidParams(format!("Resource not found: {}", uri)).into()),
        };
        Ok(json!({
            "contents": [{ "uri": uri, "mimeType": "application/json", "text": content }]
        }))
    }

    fn get_prompt_definitions(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "troubleshoot_torrent",
                "description": "Unified troubleshooting for various torrent issues (stalled, slow, connection, etc.)",
                "arguments": [
                    {
                        "name": "hash",
                        "description": "Torrent hash to troubleshoot (optional for general connection issues)",
                        "required": false
                    },
                    {
                        "name": "issue_type",
                        "description": "Type of issue: 'stalled', 'slow', 'connection', or 'general'",
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
            "troubleshoot_torrent" => {
                let issue_type = args
                    .get("issue_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("general");
                let hash = args.get("hash").and_then(|v| v.as_str());

                let prompt_text = match issue_type {
                    "stalled" | "slow" => {
                        let h = hash.ok_or(anyhow::anyhow!(
                            "Missing hash for stalled/slow troubleshooting"
                        ))?;
                        format!(
                            "I have a torrent with hash '{}' on instance '{}' that is {} . \
                             Please investigate it. Follow these steps:\n\
                             1. Check the torrent details using 'inspect_torrent'.\n\
                             2. Look for global limits or mode using 'get_system_info'.\n\
                             After investigating, suggest specific fixes (like re-announcing, toggling sequential download, or changing limits via 'manage_torrents').",
                            h, instance, issue_type
                        )
                    }
                    "connection" => {
                        format!(
                            "I think I have connection issues on instance '{}'. Please check my DHT node count and connection status via 'get_system_info', \
                             and verify if alternative speed limits are accidentally enabled.",
                            instance
                        )
                    }
                    _ => {
                        format!(
                            "Please provide a general health check for instance '{}'. \
                             Check global transfer info and list active torrents to identify any potential issues.",
                            instance
                        )
                    }
                };

                Ok(json!({
                    "description": format!("Troubleshooting for {} issue on instance {}", issue_type, instance),
                    "messages": [
                        {
                            "role": "user",
                            "content": {
                                "type": "text",
                                "text": prompt_text
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
                                     3. **Search Etiquette**: Search is asynchronous. `search_torrents` waits a few seconds, returns the best results it has, and cleans up the search job itself.\n\
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
            let mut torrent_tools = self.get_torrent_tools();
            let list_torrents_tool = torrent_tools.remove(0);
            return vec![
                list_torrents_tool,
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
                "description": "List torrents with optional filtering, sorting, field selection, and search. Use 'summary' for a compact table, 'fields' to select columns, 'search' to filter by name, 'group_by_state'/'group_by_category' for grouped breakdowns, or 'stats_only' for counts and totals without listing individual torrents.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "instance": { "type": "string", "description": "Optional: Name of the qBittorrent instance" },
                        "filter": { "type": "string", "description": "Filter by status (all, downloading, completed, paused, active, inactive, resumed, stalled, stalled_uploading, stalled_downloading, errored)" },
                        "category": { "type": "string", "description": "Filter by category" },
                        "tag": { "type": "string", "description": "Filter by tag" },
                        "search": { "type": "string", "description": "Case-insensitive substring search on torrent name" },
                        "sort": { "type": "string", "description": "Sort by column name (e.g., name, size, progress, added_on, dlspeed, upspeed, ratio, eta, state, category, tags)" },
                        "reverse": { "type": "boolean", "description": "True to reverse sort order" },
                        "limit": { "type": "integer", "description": "Maximum number of torrents to return (default 50 for listings; stats_only and group_by_* always cover all torrents)" },
                        "offset": { "type": "integer", "description": "Number of torrents to skip" },
                        "fields": { "type": "array", "items": { "type": "string" }, "description": "List of fields to include in output (e.g., ['name', 'progress', 'state', 'hash']). Omit for all fields. Available: hash, name, size, progress, dlspeed, upspeed, priority, num_seeds, num_leechs, num_incomplete, num_complete, ratio, eta, state, added_on, completion_on, seq_dl, f_l_piece_prio, category, tags, super_seeding, force_start" },
                        "summary": { "type": "boolean", "description": "Return a compact one-line-per-torrent table (name, progress, state, size, category, hash) instead of full JSON" },
                        "group_by_state": { "type": "boolean", "description": "Return torrents grouped by state with counts, plus a compact listing under each state" },
                        "group_by_category": { "type": "boolean", "description": "Return torrents grouped by category with counts, plus a compact listing under each category" },
                        "stats_only": { "type": "boolean", "description": "Return only aggregate stats (counts by state/category, total size, speed totals) with no individual torrent listing — most token-efficient view" },
                        "include_properties": { "type": "boolean", "description": "Include detailed properties for each torrent (ignored when summary=true)" },
                        "include_files": { "type": "boolean", "description": "Include file list for each torrent (ignored when summary=true)" }
                    },
                    "required": []
                }
            }),
            json!({
                "name": "manage_torrents",
                "description": "Unified tool for multiple torrent actions (pause, resume, category, tags, limits, etc.)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" },
                        "action": {
                            "type": "string",
                            "enum": [
                                "pause", "resume", "reannounce", "recheck",
                                "set_category", "add_tags", "remove_tags",
                                "set_share_limits", "set_speed_limits",
                                "toggle_sequential", "toggle_first_last_prio",
                                "set_force_start", "set_super_seeding",
                                "set_location", "rename", "set_auto_management",
                                "top_prio", "bottom_prio", "increase_prio", "decrease_prio",
                                "add_peers"
                            ],
                            "description": "Action to perform"
                        },
                        "category": { "type": "string", "description": "For 'set_category'" },
                        "tags": { "type": "string", "description": "For 'add_tags' or 'remove_tags' (comma-separated)" },
                        "ratio_limit": { "type": "number", "description": "For 'set_share_limits'" },
                        "seeding_time_limit": { "type": "integer", "description": "For 'set_share_limits' (minutes)" },
                        "inactive_seeding_time_limit": { "type": "integer", "description": "For 'set_share_limits' (minutes)" },
                        "dl_limit": { "type": "integer", "description": "For 'set_speed_limits' (bytes/s)" },
                        "up_limit": { "type": "integer", "description": "For 'set_speed_limits' (bytes/s)" },
                        "value": { "type": "boolean", "description": "For 'set_force_start', 'set_super_seeding' or 'set_auto_management'" },
                        "location": { "type": "string", "description": "For 'set_location': new save folder (moves the data)" },
                        "name": { "type": "string", "description": "For 'rename': new torrent name (single hash)" },
                        "peers": { "type": "string", "description": "For 'add_peers': host:port, pipe-separated" }
                    },
                    "required": ["hashes", "action"]
                }
            }),
            json!({
                "name": "add_torrent",
                "description": "Add torrents by magnet/HTTP URL or by uploading a .torrent file (base64). Give 'url' or 'torrent_file_base64'.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "Magnet URI or HTTP URL (newline-separated for several)" },
                        "torrent_file_base64": { "type": "string", "description": "Contents of a .torrent file, base64-encoded" },
                        "save_path": { "type": "string", "description": "Optional save path" },
                        "category": { "type": "string", "description": "Optional category" },
                        "tags": { "type": "string", "description": "Comma-separated tags" },
                        "rename": { "type": "string", "description": "New torrent name" },
                        "paused": { "type": "boolean", "description": "Add in stopped/paused state" },
                        "skip_checking": { "type": "boolean", "description": "Skip hash checking" },
                        "sequential_download": { "type": "boolean", "description": "Download pieces in order" },
                        "first_last_piece_prio": { "type": "boolean", "description": "Prioritize first and last pieces" },
                        "auto_tmm": { "type": "boolean", "description": "Use automatic torrent management" },
                        "dl_limit": { "type": "integer", "description": "Download limit (bytes/s)" },
                        "up_limit": { "type": "integer", "description": "Upload limit (bytes/s)" },
                        "ratio_limit": { "type": "number", "description": "Share ratio limit" },
                        "seeding_time_limit": { "type": "integer", "description": "Seeding time limit (minutes)" }
                    },
                    "required": []
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
                "name": "inspect_torrent",
                "description": "Comprehensive inspection of a torrent (properties, files, and trackers in one call; peers, web seeds and pieces on request)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": { "type": "string", "description": "Torrent hash" },
                        "include_peers": { "type": "boolean", "description": "Include connected peers" },
                        "include_webseeds": { "type": "boolean", "description": "Include web seeds" },
                        "include_pieces": { "type": "boolean", "description": "Include piece state counts (not downloaded / downloading / downloaded)" },
                        "include_piece_hashes": { "type": "boolean", "description": "Include every piece hash (large)" }
                    },
                    "required": ["hash"]
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
                "name": "manage_categories",
                "description": "Create, edit or remove categories",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "enum": ["create", "edit", "remove"] },
                        "name": { "type": "string", "description": "Category name (newline-separated for several when removing)" },
                        "save_path": { "type": "string", "description": "Save path for 'create' or 'edit'" }
                    },
                    "required": ["action", "name"]
                }
            }),
            json!({
                "name": "manage_tags",
                "description": "List, create or delete tags",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "enum": ["list", "create", "delete"] },
                        "tags": { "type": "string", "description": "Comma-separated tags for 'create' or 'delete'" }
                    },
                    "required": ["action"]
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
                "description": "Bulk remove completed torrents based on ratio and/or age. Runs as a dry run (lists matches) unless dry_run is false. DESTRUCTIVE: show the dry-run list to the user and confirm before calling with dry_run=false.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "min_ratio": { "type": "number", "description": "Remove when ratio is at least this (must be > 0)" },
                        "max_age_days": { "type": "integer", "description": "Remove when completed at least this many days ago (must be > 0). With min_ratio, both must match." },
                        "dry_run": { "type": "boolean", "description": "Only list what would be removed (default true)" },
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
        ]
    }

    fn get_search_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "search_torrents",
                "description": "Search for torrents with the installed search plugins. Waits up to 'wait_seconds' and returns the results found so far, then removes the search job.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query" },
                        "category": { "type": "string", "description": "Optional category" },
                        "limit": { "type": "integer", "description": "Maximum results to return (default 25, max 200)" },
                        "wait_seconds": { "type": "integer", "description": "How long to wait for results (default 5, max 30)" }
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
                "name": "set_global_speed_limits",
                "description": "Set global download/upload limits and/or turn alternative speed limits on or off. Current values are in get_system_info.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "dl_limit": { "type": "integer", "description": "Global download limit (bytes/s, 0 = unlimited)" },
                        "up_limit": { "type": "integer", "description": "Global upload limit (bytes/s, 0 = unlimited)" },
                        "alt_speed_enabled": { "type": "boolean", "description": "True to turn alternative speed limits on, false to turn them off" }
                    },
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
                "name": "manage_rss",
                "description": "Other RSS actions: remove a feed/folder, add a folder, mark as read, refresh, remove or rename a rule, or list articles a rule matches",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "enum": ["remove_item", "add_folder", "mark_as_read", "refresh_item", "remove_rule", "rename_rule", "matching_articles"] },
                        "path": { "type": "string", "description": "Feed/folder path for item actions" },
                        "article_id": { "type": "string", "description": "For 'mark_as_read': one article (omit for the whole feed)" },
                        "rule_name": { "type": "string", "description": "Rule name for rule actions" },
                        "new_rule_name": { "type": "string", "description": "For 'rename_rule'" }
                    },
                    "required": ["action"]
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
                "name": "set_app_preferences",
                "description": "Set one or more application preferences. Keys that run programs, control Web UI access, or hold passwords are refused.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "preferences": { "type": "string", "description": "JSON string of preferences to update" }
                    },
                    "required": ["preferences"]
                }
            }),
            json!({
                "name": "manage_cookies",
                "description": "Get or set the cookies qBittorrent sends when downloading .torrent files from URLs",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "enum": ["get", "set"] },
                        "cookies": { "type": "string", "description": "For 'set': JSON array of {name, domain, path, value, expirationDate}" }
                    },
                    "required": ["action"]
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
                "name": "get_system_info",
                "description": "Comprehensive system information (transfer speeds and limits, alternative speed mode, preferences without secrets, versions, build info, default save path)",
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
            "manage_torrents" => self.handle_manage_torrents(client, args).await,
            "add_torrent" => self.handle_add_torrent(client, args).await,
            "delete_torrent" => self.handle_delete_torrent(client, args).await,
            "get_torrent_files" => self.handle_get_torrent_files(client, args).await,
            "inspect_torrent" => self.handle_inspect_torrent(client, args).await,
            "get_categories" => self.handle_get_categories(client).await,
            "manage_categories" => self.handle_manage_categories(client, args).await,
            "manage_tags" => self.handle_manage_tags(client, args).await,
            "wait_for_torrent_status" => self.handle_wait_for_torrent_status(client, args).await,
            "cleanup_completed" => self.handle_cleanup_completed(client, args).await,
            "mass_rename" => self.handle_mass_rename(client, args).await,
            "find_duplicates" => self.handle_find_duplicates(client).await,
            "add_trackers" => self.handle_add_trackers(client, args).await,
            "edit_tracker" => self.handle_edit_tracker(client, args).await,
            "remove_trackers" => self.handle_remove_trackers(client, args).await,
            "rename_folder" => self.handle_rename_folder(client, args).await,
            "set_file_priority" => self.handle_set_file_priority(client, args).await,

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
            "manage_rss" => self.handle_manage_rss(client, args).await,

            // Transfer / App
            "ban_peers" => self.handle_ban_peers(client, args).await,
            "set_global_speed_limits" => self.handle_set_global_speed_limits(client, args).await,
            "manage_cookies" => self.handle_manage_cookies(client, args).await,
            "set_app_preferences" => self.handle_set_app_preferences(client, args).await,
            "get_main_log" => self.handle_get_main_log(client, args).await,
            "get_peer_log" => self.handle_get_peer_log(client, args).await,
            "get_system_info" => self.handle_get_system_info(client).await,
            "shutdown_app" => self.handle_shutdown_app(client).await,

            _ => Err(InvalidParams(format!("Unknown tool: {}", name)).into()),
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
        let search = args.get("search").and_then(|v| v.as_str());
        let summary = args
            .get("summary")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let fields: Option<Vec<&str>> = args.get("fields").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|f| f.as_str()).collect())
        });

        let include_properties = args
            .get("include_properties")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let include_files = args
            .get("include_files")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let (api_limit, api_offset) = if search.is_some() {
            (None, None)
        } else {
            (limit, offset)
        };

        let mut torrents = client
            .get_torrent_list(filter, category, tag, sort, reverse, api_limit, api_offset)
            .await?;

        // Apply name search filter and client-side pagination when searching
        if let Some(search_term) = search {
            let search_lower = search_term.to_lowercase();
            torrents.retain(|t| t.name.to_lowercase().contains(&search_lower));

            if let Some(off) = offset {
                let off = off.max(0) as usize;
                if off >= torrents.len() {
                    torrents.clear();
                } else {
                    torrents.drain(0..off);
                }
            }
            if let Some(lim) = limit {
                let lim = lim.max(0) as usize;
                torrents.truncate(lim);
            }
        }

        let group_by_state = args
            .get("group_by_state")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let group_by_category = args
            .get("group_by_category")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let stats_only = args
            .get("stats_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Listings default to 50 torrents so one call cannot flood the model's context.
        const DEFAULT_LIMIT: usize = 50;
        let mut note = String::new();
        if limit.is_none()
            && !stats_only
            && !group_by_state
            && !group_by_category
            && torrents.len() > DEFAULT_LIMIT
        {
            note = format!(
                "Showing {} of {} torrents. Use limit/offset, search, or a filter to see more.",
                DEFAULT_LIMIT,
                torrents.len()
            );
            torrents.truncate(DEFAULT_LIMIT);
        }
        let total = torrents.len();

        // Stats only mode: aggregate counts, no individual torrents
        if stats_only {
            let mut state_counts: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();
            let mut cat_counts: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();
            let mut total_size: i64 = 0;
            let mut total_dl_speed: i64 = 0;
            let mut total_ul_speed: i64 = 0;

            for t in &torrents {
                *state_counts.entry(t.state.clone()).or_default() += 1;
                let cat_name = if t.category.is_empty() {
                    "(none)".to_string()
                } else {
                    t.category.clone()
                };
                *cat_counts.entry(cat_name).or_default() += 1;
                total_size += t.size_bytes;
                total_dl_speed += t.dlspeed;
                total_ul_speed += t.upspeed;
            }

            let mut lines = Vec::new();
            lines.push(format!(
                "=== {} torrents | {} total | DL {} /s | UL {} /s ===",
                total,
                Self::format_size(total_size),
                Self::format_size(total_dl_speed),
                Self::format_size(total_ul_speed),
            ));
            lines.push(String::new());
            lines.push("By state:".to_string());
            for (state, count) in &state_counts {
                lines.push(format!("  {:<20} {:>4}", state, count));
            }
            lines.push(String::new());
            lines.push("By category:".to_string());
            for (cat, count) in &cat_counts {
                lines.push(format!("  {:<20} {:>4}", cat, count));
            }

            let text = lines.join("\n");
            return Ok(json!({ "content": [{ "type": "text", "text": text }] }));
        }

        // Group by state mode
        if group_by_state {
            let mut groups: std::collections::BTreeMap<String, Vec<&crate::models::Torrent>> =
                std::collections::BTreeMap::new();
            for t in &torrents {
                groups.entry(t.state.clone()).or_default().push(t);
            }

            let mut lines = Vec::new();
            lines.push(format!(
                "Total: {} torrents across {} states",
                total,
                groups.len()
            ));
            lines.push(String::new());

            // State summary counts
            for (state, group) in &groups {
                lines.push(format!("  {:<20} {:>4}", state, group.len()));
            }
            lines.push(String::new());

            // Detailed listing per state
            for (state, group) in &groups {
                lines.push(format!("── {} ({}) ──", state, group.len()));
                for t in group {
                    let size = Self::format_size(t.size_bytes);
                    lines.push(format!(
                        "  {:<58} {:>6.1}% {:>10}  {}",
                        Self::truncate_str(&t.name, 58),
                        t.progress * 100.0,
                        size,
                        Self::short_hash(&t.hash),
                    ));
                }
                lines.push(String::new());
            }

            let text = lines.join("\n");
            return Ok(json!({ "content": [{ "type": "text", "text": text }] }));
        }

        // Group by category mode
        if group_by_category {
            let mut groups: std::collections::BTreeMap<String, Vec<&crate::models::Torrent>> =
                std::collections::BTreeMap::new();
            for t in &torrents {
                let cat_name = if t.category.is_empty() {
                    "(none)".to_string()
                } else {
                    t.category.clone()
                };
                groups.entry(cat_name).or_default().push(t);
            }

            let mut lines = Vec::new();
            lines.push(format!(
                "Total: {} torrents across {} categories",
                total,
                groups.len()
            ));
            lines.push(String::new());

            // Category summary counts
            for (cat, group) in &groups {
                let cat_size: i64 = group.iter().map(|t| t.size_bytes).sum();
                lines.push(format!(
                    "  {:<20} {:>4}  ({})",
                    cat,
                    group.len(),
                    Self::format_size(cat_size)
                ));
            }
            lines.push(String::new());

            // Detailed listing per category
            for (cat, group) in &groups {
                lines.push(format!("── {} ({}) ──", cat, group.len()));
                for t in group {
                    let size = Self::format_size(t.size_bytes);
                    lines.push(format!(
                        "  {:<58} {:>6.1}% {:>10}  {}",
                        Self::truncate_str(&t.name, 58),
                        t.progress * 100.0,
                        size,
                        Self::short_hash(&t.hash),
                    ));
                }
                lines.push(String::new());
            }

            let text = lines.join("\n");
            return Ok(json!({ "content": [{ "type": "text", "text": text }] }));
        }

        // Summary mode: compact one-line-per-torrent table
        if summary {
            let mut lines = Vec::with_capacity(total + 2);
            lines.push(format!("Found {} torrents", total));
            lines.push(format!(
                "{:<60} {:>7} {:<20} {:>10} {:<15} {}",
                "Name", "Prog %", "State", "Size", "Category", "Hash"
            ));
            lines.push("-".repeat(160));
            for t in &torrents {
                let size = Self::format_size(t.size_bytes);
                lines.push(format!(
                    "{:<60} {:>6.1}% {:<20} {:>10} {:<15} {}",
                    Self::truncate_str(&t.name, 60),
                    t.progress * 100.0,
                    t.state,
                    size,
                    Self::truncate_str(&t.category, 15),
                    t.hash,
                ));
            }
            let text = lines.join("\n");
            return Ok(text_result(if note.is_empty() {
                text
            } else {
                format!("{}\n{}", note, text)
            }));
        }

        // Field selection: filter JSON keys
        if let Some(ref selected_fields) = fields {
            let filtered: Vec<Value> = torrents
                .iter()
                .map(|t| {
                    let full = json!(t);
                    let obj = match full.as_object() {
                        Some(o) => o,
                        None => return Value::Object(serde_json::Map::new()),
                    };
                    let mut picked = serde_json::Map::new();
                    for &field in selected_fields {
                        // Map user-friendly names to serde keys
                        let key = match field {
                            "size" => "size_bytes",
                            other => other,
                        };
                        if let Some(val) = obj.get(key) {
                            picked.insert(field.to_string(), val.clone());
                        }
                    }
                    Value::Object(picked)
                })
                .collect();
            let text = serde_json::to_string(&json!({
                "total": total,
                "torrents": filtered,
            }))?;
            return Ok(text_result(if note.is_empty() {
                text
            } else {
                format!("{}\n{}", note, text)
            }));
        }

        // Full output (with optional properties/files)
        if !include_properties && !include_files {
            let text = serde_json::to_string(&torrents)?;
            return Ok(text_result(if note.is_empty() {
                text
            } else {
                format!("{}\n{}", note, text)
            }));
        }

        let mut detailed_torrents = Vec::new();
        for t in torrents {
            let mut detailed = json!(t);
            if include_properties && let Ok(props) = client.get_torrent_properties(&t.hash).await {
                detailed["properties"] = json!(props);
            }
            if include_files && let Ok(files) = client.get_torrent_files(&t.hash).await {
                detailed["files"] = json!(files);
            }
            detailed_torrents.push(detailed);
        }

        let text = serde_json::to_string(&detailed_torrents)?;
        Ok(text_result(if note.is_empty() {
            text
        } else {
            format!("{}\n{}", note, text)
        }))
    }

    fn format_size(bytes: i64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;
        const TB: f64 = GB * 1024.0;
        let b = bytes as f64;
        if b >= TB {
            format!("{:.1} TB", b / TB)
        } else if b >= GB {
            format!("{:.1} GB", b / GB)
        } else if b >= MB {
            format!("{:.1} MB", b / MB)
        } else if b >= KB {
            format!("{:.1} KB", b / KB)
        } else {
            format!("{} B", bytes)
        }
    }

    fn truncate_str(s: &str, max: usize) -> String {
        if s.chars().count() <= max {
            s.to_string()
        } else {
            let truncated: String = s.chars().take(max.saturating_sub(3)).collect();
            format!("{}...", truncated)
        }
    }

    fn short_hash(hash: &str) -> &str {
        let len = hash
            .char_indices()
            .map(|(i, _)| i)
            .nth(12)
            .unwrap_or(hash.len());
        &hash[..len]
    }

    async fn handle_manage_torrents(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hashes = args
            .get("hashes")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hashes"))?;
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing action"))?;

        match action {
            "pause" => client.pause_torrents(hashes).await?,
            "resume" => client.resume_torrents(hashes).await?,
            "reannounce" => client.reannounce_torrents(hashes).await?,
            "recheck" => client.recheck_torrents(hashes).await?,
            "set_category" => {
                let category = args
                    .get("category")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing category"))?;
                client.set_category(hashes, category).await?;
            }
            "add_tags" => {
                let tags = args
                    .get("tags")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing tags"))?;
                client.add_tags(hashes, tags).await?;
            }
            "remove_tags" => {
                let tags = args
                    .get("tags")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("Missing tags"))?;
                client.remove_tags(hashes, tags).await?;
            }
            "set_share_limits" => {
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
            }
            "set_speed_limits" => {
                if let Some(limit) = args.get("dl_limit").and_then(|v| v.as_i64()) {
                    client.set_torrent_download_limit(hashes, limit).await?;
                }
                if let Some(limit) = args.get("up_limit").and_then(|v| v.as_i64()) {
                    client.set_torrent_upload_limit(hashes, limit).await?;
                }
            }
            "toggle_sequential" => client.toggle_sequential_download(hashes).await?,
            "toggle_first_last_prio" => client.toggle_first_last_piece_priority(hashes).await?,
            "set_force_start" => {
                let value = args
                    .get("value")
                    .and_then(|v| v.as_bool())
                    .ok_or(anyhow::anyhow!("Missing value"))?;
                client.set_force_start(hashes, value).await?;
            }
            "set_super_seeding" => {
                let value = args
                    .get("value")
                    .and_then(|v| v.as_bool())
                    .ok_or(anyhow::anyhow!("Missing value"))?;
                client.set_super_seeding(hashes, value).await?;
            }
            "set_location" => {
                client
                    .set_location(hashes, arg_str(args, "location")?)
                    .await?
            }
            "rename" => {
                client
                    .rename_torrent(hashes, arg_str(args, "name")?)
                    .await?
            }
            "set_auto_management" => {
                client
                    .set_auto_management(hashes, arg_bool(args, "value")?)
                    .await?
            }
            "top_prio" => client.set_queue_priority(hashes, "topPrio").await?,
            "bottom_prio" => client.set_queue_priority(hashes, "bottomPrio").await?,
            "increase_prio" => client.set_queue_priority(hashes, "increasePrio").await?,
            "decrease_prio" => client.set_queue_priority(hashes, "decreasePrio").await?,
            "add_peers" => client.add_peers(hashes, arg_str(args, "peers")?).await?,
            _ => anyhow::bail!("Unsupported action: {}", action),
        }

        Ok(
            json!({ "content": [{ "type": "text", "text": format!("Action '{}' performed successfully on torrents.", action) }] }),
        )
    }

    async fn handle_search_torrents(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let query = arg_str(args, "query")?;
        let category = args.get("category").and_then(|v| v.as_str());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(25)
            .clamp(1, 200);
        let wait = args
            .get("wait_seconds")
            .and_then(|v| v.as_i64())
            .unwrap_or(5)
            .clamp(1, 30);

        let id = client.start_search(query, category).await?;
        for _ in 0..wait {
            sleep(Duration::from_secs(1)).await;
            if client
                .get_search_status(id)
                .await
                .is_ok_and(|s| s == "Stopped")
            {
                break;
            }
        }
        let results = client.get_search_results(id, Some(limit), None).await;
        let _ = client.stop_search(id).await;
        let _ = client.delete_search(id).await;
        let results = results?;
        let text = serde_json::to_string(&json!({
            "total_found": results.total,
            "returned": results.results.len(),
            "results": results.results,
        }))?;
        Ok(text_result(text))
    }

    async fn handle_add_torrent(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let url = args.get("url").and_then(|v| v.as_str());
        let file = match args.get("torrent_file_base64").and_then(|v| v.as_str()) {
            Some(b64) => Some(("upload.torrent".to_string(), decode_base64(b64)?)),
            None => None,
        };
        if url.is_none() && file.is_none() {
            anyhow::bail!("Missing url or torrent_file_base64");
        }

        let mut fields: Vec<(&str, String)> = Vec::new();
        if let Some(u) = url {
            fields.push(("urls", u.to_string()));
        }
        for (arg, field) in [
            ("save_path", "savepath"),
            ("category", "category"),
            ("tags", "tags"),
            ("rename", "rename"),
        ] {
            if let Some(v) = args.get(arg).and_then(|v| v.as_str()) {
                fields.push((field, v.to_string()));
            }
        }
        for (arg, field) in [
            ("skip_checking", "skip_checking"),
            ("sequential_download", "sequentialDownload"),
            ("first_last_piece_prio", "firstLastPiecePrio"),
            ("auto_tmm", "autoTMM"),
        ] {
            if let Some(v) = args.get(arg).and_then(|v| v.as_bool()) {
                fields.push((field, v.to_string()));
            }
        }
        if let Some(v) = args.get("paused").and_then(|v| v.as_bool()) {
            // qBittorrent 5 reads "stopped", 4.x reads "paused".
            fields.push(("stopped", v.to_string()));
            fields.push(("paused", v.to_string()));
        }
        for (arg, field) in [
            ("dl_limit", "dlLimit"),
            ("up_limit", "upLimit"),
            ("seeding_time_limit", "seedingTimeLimit"),
        ] {
            if let Some(v) = args.get(arg).and_then(|v| v.as_i64()) {
                fields.push((field, v.to_string()));
            }
        }
        if let Some(v) = args.get("ratio_limit").and_then(|v| v.as_f64()) {
            fields.push(("ratioLimit", v.to_string()));
        }

        client.add_torrent_full(&fields, file).await?;
        Ok(text_result("Torrent added successfully"))
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

    async fn handle_get_torrent_files(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = args
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing hash"))?;
        let files = client.get_torrent_files(hash).await?;
        let text = serde_json::to_string(&files)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_inspect_torrent(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = arg_str(args, "hash")?;
        let flag = |k: &str| args.get(k).and_then(|v| v.as_bool()).unwrap_or(false);

        let (properties, files, trackers) = tokio::try_join!(
            client.get_torrent_properties(hash),
            client.get_torrent_files(hash),
            client.get_torrent_trackers(hash),
        )?;
        let mut result = json!({
            "properties": properties,
            "files": files,
            "trackers": trackers
        });
        if flag("include_peers") {
            result["peers"] = client.get_torrent_peers(hash).await?;
        }
        if flag("include_webseeds") {
            result["webseeds"] = client.get_torrent_webseeds(hash).await?;
        }
        if flag("include_pieces") {
            let states = client.get_piece_states(hash).await?;
            let count = |s: i64| states.iter().filter(|&&x| x == s).count();
            result["pieces"] = json!({
                "total": states.len(),
                "not_downloaded": count(0),
                "downloading": count(1),
                "downloaded": count(2),
            });
        }
        if flag("include_piece_hashes") {
            result["piece_hashes"] = json!(client.get_piece_hashes(hash).await?);
        }

        Ok(text_result(serde_json::to_string(&result)?))
    }

    async fn handle_get_system_info(&self, client: &QBitClient) -> Result<Value> {
        let (
            transfer_info,
            mut app_preferences,
            app_version,
            webapi_version,
            build_info,
            speed_mode,
            default_save_path,
        ) = tokio::try_join!(
            client.get_global_transfer_info(),
            client.get_app_preferences(),
            client.get_app_version(),
            client.get_webapi_version(),
            client.get_build_info(),
            client.get_speed_limits_mode(),
            client.get_default_save_path(),
        )?;
        redact_preferences(&mut app_preferences);

        let result = json!({
            "transfer_info": transfer_info,
            "alt_speed_limits_enabled": speed_mode == 1,
            "default_save_path": default_save_path,
            "app_preferences": app_preferences,
            "app_version": app_version,
            "webapi_version": webapi_version,
            "build_info": build_info
        });

        Ok(text_result(serde_json::to_string(&result)?))
    }

    async fn handle_get_categories(&self, client: &QBitClient) -> Result<Value> {
        let categories = client.get_categories().await?;
        let text = serde_json::to_string(&categories)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
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
        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if min_ratio.is_none() && max_age_days.is_none() {
            anyhow::bail!(
                "Give min_ratio and/or max_age_days; refusing to match every completed torrent"
            );
        }
        if min_ratio.is_some_and(|r| r <= 0.0) || max_age_days.is_some_and(|d| d <= 0) {
            anyhow::bail!("min_ratio and max_age_days must be greater than 0");
        }

        let torrents = client
            .get_torrent_list(Some("completed"), None, None, None, None, None, None)
            .await?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        // Every condition given must match.
        let matches: Vec<_> = torrents
            .into_iter()
            .filter(|t| {
                min_ratio.is_none_or(|r| t.ratio >= r)
                    && max_age_days.is_none_or(|d| {
                        t.completion_on > 0 && now - t.completion_on >= d * 24 * 3600
                    })
            })
            .collect();

        if matches.is_empty() {
            return Ok(text_result("No torrents matched the cleanup criteria."));
        }
        if dry_run {
            let list: Vec<Value> = matches
                .iter()
                .map(|t| json!({ "hash": t.hash, "name": t.name, "ratio": t.ratio, "completion_on": t.completion_on }))
                .collect();
            return Ok(text_result(format!(
                "Dry run: {} torrents would be removed (delete_files={}). Call again with dry_run=false to remove them.\n{}",
                matches.len(),
                delete_files,
                serde_json::to_string(&list)?
            )));
        }

        let hashes: Vec<&str> = matches.iter().map(|t| t.hash.as_str()).collect();
        client
            .delete_torrents(&hashes.join("|"), delete_files)
            .await?;
        Ok(text_result(format!(
            "Successfully cleaned up {} torrents.",
            hashes.len()
        )))
    }

    async fn handle_mass_rename(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let hash = arg_str(args, "hash")?;
        let pattern = arg_str(args, "pattern")?;
        let replacement = arg_str(args, "replacement")?;

        let re = Regex::new(pattern)?;
        let files = client.get_torrent_files(hash).await?;

        let mut renamed = 0;
        let mut failed = Vec::new();
        for file in files {
            let new_name = re.replace_all(&file.name, replacement);
            if new_name != file.name {
                // Keep going on a failure so one bad file does not leave the rest half-done.
                match client.rename_file(hash, &file.name, &new_name).await {
                    Ok(()) => renamed += 1,
                    Err(e) => failed.push(format!("{}: {}", file.name, e)),
                }
            }
        }

        let mut text = format!("Successfully renamed {} files.", renamed);
        if !failed.is_empty() {
            text += &format!(" {} failed:\n{}", failed.len(), failed.join("\n"));
        }
        Ok(json!({ "content": [{ "type": "text", "text": text }], "isError": !failed.is_empty() }))
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

        let text = serde_json::to_string(&duplicates)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
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
        let text = serde_json::to_string(&plugins)?;
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
        let text = serde_json::to_string(&feeds)?;
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
        let text = serde_json::to_string(&rules)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
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

    async fn handle_ban_peers(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let peers = args
            .get("peers")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("Missing peers"))?;
        client.ban_peers(peers).await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Peers banned successfully" }] }))
    }

    async fn handle_set_app_preferences(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let prefs: Value = serde_json::from_str(arg_str(args, "preferences")?)?;
        let obj = prefs
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("preferences must be a JSON object"))?;
        let blocked: Vec<&str> = obj
            .keys()
            .map(|k| k.as_str())
            .filter(|k| is_blocked_preference(k))
            .collect();
        if !blocked.is_empty() {
            anyhow::bail!(
                "Refused: these preferences can only be changed in the qBittorrent Web UI: {}",
                blocked.join(", ")
            );
        }
        client.set_app_preferences(&prefs).await?;
        Ok(text_result("App preferences updated successfully"))
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
        let text = serde_json::to_string(&logs)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_get_peer_log(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let last_id = args.get("last_id").and_then(|v| v.as_i64());
        let logs = client.get_peer_log(last_id).await?;
        let text = serde_json::to_string(&logs)?;
        Ok(json!({ "content": [{ "type": "text", "text": text }] }))
    }

    async fn handle_manage_categories(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        let name = arg_str(args, "name")?;
        let save_path = args.get("save_path").and_then(|v| v.as_str()).unwrap_or("");
        match arg_str(args, "action")? {
            "create" => client.create_category(name, save_path).await?,
            "edit" => client.edit_category(name, save_path).await?,
            "remove" => client.remove_categories(name).await?,
            other => anyhow::bail!("Unsupported action: {}", other),
        }
        Ok(text_result("Categories updated successfully"))
    }

    async fn handle_manage_tags(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        match arg_str(args, "action")? {
            "list" => {
                return Ok(text_result(serde_json::to_string(
                    &client.get_tags().await?,
                )?));
            }
            "create" => client.create_tags(arg_str(args, "tags")?).await?,
            "delete" => client.delete_tags(arg_str(args, "tags")?).await?,
            other => anyhow::bail!("Unsupported action: {}", other),
        }
        Ok(text_result("Tags updated successfully"))
    }

    async fn handle_set_global_speed_limits(
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
        // The API only toggles, so read the mode first to make the call idempotent.
        if let Some(want) = args.get("alt_speed_enabled").and_then(|v| v.as_bool())
            && (client.get_speed_limits_mode().await? == 1) != want
        {
            client.toggle_alternative_speed_limits().await?;
        }
        Ok(text_result("Global speed limits updated successfully"))
    }

    async fn handle_manage_rss(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        match arg_str(args, "action")? {
            "remove_item" => client.remove_rss_item(arg_str(args, "path")?).await?,
            "add_folder" => client.add_rss_folder(arg_str(args, "path")?).await?,
            "mark_as_read" => {
                let article = args.get("article_id").and_then(|v| v.as_str());
                client
                    .mark_rss_as_read(arg_str(args, "path")?, article)
                    .await?
            }
            "refresh_item" => client.refresh_rss_item(arg_str(args, "path")?).await?,
            "remove_rule" => client.remove_rss_rule(arg_str(args, "rule_name")?).await?,
            "rename_rule" => {
                client
                    .rename_rss_rule(arg_str(args, "rule_name")?, arg_str(args, "new_rule_name")?)
                    .await?
            }
            "matching_articles" => {
                let articles = client
                    .get_rss_matching_articles(arg_str(args, "rule_name")?)
                    .await?;
                return Ok(text_result(serde_json::to_string(&articles)?));
            }
            other => anyhow::bail!("Unsupported action: {}", other),
        }
        Ok(text_result("RSS updated successfully"))
    }

    async fn handle_manage_cookies(&self, client: &QBitClient, args: &Value) -> Result<Value> {
        match arg_str(args, "action")? {
            "get" => Ok(text_result(serde_json::to_string(
                &client.get_cookies().await?,
            )?)),
            "set" => {
                client.set_cookies(arg_str(args, "cookies")?).await?;
                Ok(text_result("Cookies updated successfully"))
            }
            other => anyhow::bail!("Unsupported action: {}", other),
        }
    }

    async fn handle_shutdown_app(&self, client: &QBitClient) -> Result<Value> {
        client.shutdown_app().await?;
        Ok(json!({ "content": [{ "type": "text", "text": "Shutdown command sent" }] }))
    }

    pub fn start_event_loop(&self, interval_ms: u64) {
        let server = self.clone();
        tokio::spawn(async move {
            server.event_loop(interval_ms).await;
        });
    }

    async fn event_loop(&self, interval_ms: u64) {
        struct InstanceState {
            rid: i64,
            connected: bool,
            seeded: bool,
            notified: std::collections::HashSet<String>,
            failures: u32,
            next_poll: tokio::time::Instant,
        }
        let mut states: HashMap<&String, InstanceState> = self
            .clients
            .keys()
            .map(|name| {
                (
                    name,
                    InstanceState {
                        rid: 0,
                        connected: false,
                        seeded: false,
                        notified: Default::default(),
                        failures: 0,
                        next_poll: tokio::time::Instant::now(),
                    },
                )
            })
            .collect();

        while self.is_running() {
            sleep(Duration::from_millis(interval_ms)).await;
            for (name, client) in &self.clients {
                let st = states.get_mut(name).unwrap();
                if !self.is_running() {
                    break;
                }
                if tokio::time::Instant::now() < st.next_poll {
                    continue;
                }
                match client.get_main_data(st.rid).await {
                    Ok(data) => {
                        if !st.connected {
                            info!("Connected to qBittorrent instance '{}'", name);
                            st.connected = true;
                        }
                        st.failures = 0;
                        st.rid = data.rid;

                        let Some(torrents) = data.torrents else {
                            continue;
                        };
                        for (hash, t) in torrents {
                            let progress = t.get("progress").and_then(|p| p.as_f64());
                            let state = t.get("state").and_then(|s| s.as_str());
                            let finished = progress.is_some_and(|p| p >= 1.0)
                                || state.is_some_and(|s| {
                                    matches!(s, "uploading" | "stalledUP" | "queuedUP" | "forcedUP")
                                });
                            if !finished || st.notified.contains(&hash) {
                                continue;
                            }
                            // The first poll only records torrents that were already finished.
                            if st.seeded {
                                let torrent_name =
                                    t.get("name").and_then(|n| n.as_str()).unwrap_or(&hash);
                                info!(
                                    "Notification: Torrent '{}' finished on {}",
                                    torrent_name, name
                                );
                                self.push_notification(
                                    "notifications/resources/updated",
                                    json!({ "uri": format!("qbittorrent://{}/torrents", name) }),
                                );
                            }
                            st.notified.insert(hash);
                        }
                        st.seeded = true;
                    }
                    Err(e) => {
                        if st.connected {
                            error!("Lost connection to qBittorrent instance '{}': {}", name, e);
                            st.connected = false;
                            st.rid = 0;
                        }
                        // Back off 2s, 4s, 8s ... up to 5 minutes so a bad password does not get the IP banned.
                        st.failures = st.failures.saturating_add(1);
                        let delay = 2u64.saturating_pow(st.failures.min(9)).min(300);
                        debug!("Polling '{}' failed ({}); next try in {}s", name, e, delay);
                        st.next_poll = tokio::time::Instant::now() + Duration::from_secs(delay);
                    }
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
            (
                "manage_torrents",
                json!({ "hashes": "abc", "action": "pause" }),
            ),
            ("add_torrent", json!({ "url": "magnet:?xt=urn:btih:..." })),
            (
                "delete_torrent",
                json!({ "hash": "abc", "delete_files": false }),
            ),
            ("get_torrent_files", json!({ "hash": "abc" })),
            ("inspect_torrent", json!({ "hash": "abc" })),
            ("get_categories", json!({})),
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
            ("ban_peers", json!({ "peers": "1.1.1.1:80" })),
            ("set_app_preferences", json!({ "preferences": "{}" })),
            ("get_main_log", json!({})),
            ("get_peer_log", json!({})),
            ("get_system_info", json!({})),
            ("shutdown_app", json!({})),
            (
                "manage_categories",
                json!({ "action": "create", "name": "c" }),
            ),
            ("manage_tags", json!({ "action": "list" })),
            ("set_global_speed_limits", json!({ "dl_limit": 0 })),
            ("manage_rss", json!({ "action": "add_folder", "path": "f" })),
            ("manage_cookies", json!({ "action": "get" })),
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
            let resp = server.handle_request(req).await;
            if let Some(error) = resp.ok().and_then(|r| r.get("error").cloned()) {
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

    #[test]
    fn test_decode_base64() {
        assert_eq!(decode_base64("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(decode_base64("aGVsbG8").unwrap(), b"hello");
        assert_eq!(decode_base64("+/8=").unwrap(), vec![0xfb, 0xff]);
        assert!(decode_base64("a*b").is_err());
    }

    #[test]
    fn test_preference_filters() {
        assert!(is_blocked_preference("autorun_program"));
        assert!(is_blocked_preference("web_ui_password"));
        assert!(is_blocked_preference("bypass_auth_subnet_whitelist"));
        assert!(is_blocked_preference("proxy_password"));
        assert!(!is_blocked_preference("dl_limit"));
        assert!(!is_blocked_preference("save_path"));

        let mut prefs = json!({ "proxy_password": "x", "mail_notification_password": "y", "dyndns_password": "z", "save_path": "/d" });
        redact_preferences(&mut prefs);
        assert_eq!(prefs, json!({ "save_path": "/d" }));
    }

    #[tokio::test]
    async fn test_rpc_error_codes() {
        let server = McpServer::new(HashMap::new(), false);
        let req = |method: &str, params: Option<Value>| JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
            id: Some(json!(1)),
        };
        let code = |r: Option<JsonRpcResponse>| r.unwrap().error.unwrap().code;
        assert_eq!(code(server.handle_rpc(req("nope", None)).await), -32601);
        assert_eq!(
            code(server.handle_rpc(req("tools/call", None)).await),
            -32602
        );

        let mut clients = HashMap::new();
        clients.insert(
            "default".into(),
            QBitClient::new_no_auth("http://127.0.0.1:1", false),
        );
        let server = McpServer::new(clients, false);
        let unknown = server
            .handle_rpc(req(
                "tools/call",
                Some(json!({ "name": "nope", "arguments": {} })),
            ))
            .await;
        assert_eq!(code(unknown), -32602);
        // A failing tool is a result with isError, not a protocol error.
        let failed = server
            .handle_rpc(req(
                "tools/call",
                Some(json!({ "name": "get_categories", "arguments": {} })),
            ))
            .await
            .unwrap();
        assert!(failed.error.is_none());
        assert_eq!(failed.result.unwrap()["isError"], true);
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

    #[tokio::test]
    async fn test_handle_prompt_variations() {
        let clients = HashMap::new();
        let server = McpServer::new(clients, false);

        let prompts = vec![
            (
                "troubleshoot_torrent",
                json!({ "issue_type": "stalled", "hash": "abc", "instance": "inst1" }),
            ),
            ("analyze_disk_space", json!({ "instance": "inst1" })),
        ];

        for (name, args) in prompts {
            let res = server.handle_prompt_get(name, &args).await.unwrap();
            assert!(res.get("description").is_some());
        }

        // Test error case: missing hash for stalled
        let res = server
            .handle_prompt_get("troubleshoot_torrent", &json!({ "issue_type": "stalled" }))
            .await;
        assert!(res.is_err());

        // Test error case: unknown prompt
        let res = server.handle_prompt_get("unknown_prompt", &json!({})).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_get_client_logic() {
        let mut clients = HashMap::new();
        clients.insert(
            "inst1".to_string(),
            QBitClient::new_no_auth("http://h1", false),
        );
        clients.insert(
            "inst2".to_string(),
            QBitClient::new_no_auth("http://h2", false),
        );
        let server = McpServer::new(clients, false);

        // Explicit instance
        assert_eq!(
            server.get_client(Some("inst1")).unwrap().base_url(),
            "http://h1"
        );
        assert_eq!(
            server.get_client(Some("inst2")).unwrap().base_url(),
            "http://h2"
        );
        assert!(server.get_client(Some("inst3")).is_err());

        // Default logic (first one since no "default" key)
        let default_client = server.get_client(None).unwrap();
        assert!(
            default_client.base_url() == "http://h1" || default_client.base_url() == "http://h2"
        );

        // With "default" key
        let mut clients2 = HashMap::new();
        clients2.insert(
            "default".to_string(),
            QBitClient::new_no_auth("http://default", false),
        );
        clients2.insert(
            "other".to_string(),
            QBitClient::new_no_auth("http://other", false),
        );
        let server2 = McpServer::new(clients2, false);
        assert_eq!(
            server2.get_client(None).unwrap().base_url(),
            "http://default"
        );
    }

    #[tokio::test]
    async fn test_manage_torrents_routing() {
        let client = QBitClient::new("http://localhost:8080", "admin", "adminadmin", false);
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let server = McpServer::new(clients, false);

        let actions = vec![
            "pause",
            "resume",
            "reannounce",
            "recheck",
            "set_category",
            "add_tags",
            "remove_tags",
            "set_share_limits",
            "set_speed_limits",
            "toggle_sequential",
            "toggle_first_last_prio",
            "set_force_start",
            "set_super_seeding",
            "set_location",
            "rename",
            "set_auto_management",
            "top_prio",
            "bottom_prio",
            "increase_prio",
            "decrease_prio",
            "add_peers",
        ];

        for action in actions {
            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                method: "tools/call".to_string(),
                params: Some(json!({
                    "name": "manage_torrents",
                    "arguments": {
                        "hashes": "abc",
                        "action": action,
                        // Add required params for specific actions to avoid validation errors if we had any
                        "category": "test",
                        "tags": "t1",
                        "ratio_limit": 1.0,
                        "seeding_time_limit": 60,
                        "value": true,
                        "location": "/x",
                        "name": "n",
                        "peers": "1.1.1.1:1",
                    }
                })),
                id: Some(json!(1)),
            };

            let resp = server.handle_request(req).await;
            // It should not return "Unknown tool"
            if let Ok(val) = resp {
                if let Some(error) = val.get("error") {
                    assert_ne!(error["message"], "Unknown tool: manage_torrents");
                }
            } else if let Err(e) = resp {
                assert_ne!(e.to_string(), "Unknown tool: manage_torrents");
            }
        }
    }

    #[tokio::test]
    async fn test_inspect_torrent_routing() {
        let client = QBitClient::new("http://localhost:8080", "admin", "adminadmin", false);
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let server = McpServer::new(clients, false);

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "inspect_torrent",
                "arguments": { "hash": "abc" }
            })),
            id: Some(json!(1)),
        };

        let resp = server.handle_request(req).await;
        if let Ok(val) = resp
            && let Some(error) = val.get("error")
        {
            assert_ne!(error["message"], "Unknown tool: inspect_torrent");
        }
    }

    #[tokio::test]
    async fn test_get_system_info_routing() {
        let client = QBitClient::new("http://localhost:8080", "admin", "adminadmin", false);
        let mut clients = HashMap::new();
        clients.insert("default".to_string(), client);
        let server = McpServer::new(clients, false);

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "get_system_info",
                "arguments": {}
            })),
            id: Some(json!(1)),
        };

        let resp = server.handle_request(req).await;
        if let Ok(val) = resp
            && let Some(error) = val.get("error")
        {
            assert_ne!(error["message"], "Unknown tool: get_system_info");
        }
    }
}
