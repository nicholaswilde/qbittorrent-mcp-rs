use crate::client::QBitClient;
use anyhow::Result;
use async_trait::async_trait;
use mcp_sdk_rs::Error as McpError;
use mcp_sdk_rs::error::ErrorCode;
use mcp_sdk_rs::server::ServerHandler;
use mcp_sdk_rs::types::{ClientCapabilities, Implementation, ServerCapabilities, Tool, ToolSchema};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};

use std::time::Duration;
use tokio::time::sleep;

pub struct AppHandler {
    client: QBitClient,
    lazy_mode: AtomicBool,
}

impl AppHandler {
    pub fn new(client: QBitClient, lazy_mode: bool) -> Self {
        Self {
            client,
            lazy_mode: AtomicBool::new(lazy_mode),
        }
    }
}

#[async_trait]
impl ServerHandler for AppHandler {
    async fn initialize(
        &self,
        _implementation: Implementation,
        _capabilities: ClientCapabilities,
    ) -> Result<ServerCapabilities, McpError> {
        let capabilities = ServerCapabilities::default();
        Ok(capabilities)
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        Ok(())
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, McpError> {
        match method {
            "tools/list" => {
                let is_lazy = self.lazy_mode.load(Ordering::Relaxed);

                let list_tool = Tool {
                    name: "list_torrents".to_string(),
                    description: "List all torrents".to_string(),
                    input_schema: Some(ToolSchema {
                        properties: Some(json!({})),
                        required: None,
                    }),
                    annotations: None,
                };

                let show_all_tool = Tool {
                    name: "show_all_tools".to_string(),
                    description: "Enable all available tools".to_string(),
                    input_schema: Some(ToolSchema {
                        properties: Some(json!({})),
                        required: None,
                    }),
                    annotations: None,
                };

                let mut tools = vec![list_tool];

                if is_lazy {
                    tools.push(show_all_tool);
                } else {
                    let search_tool = Tool {
                        name: "search_torrents".to_string(),
                        description: "Search for torrents (waits 5s for results)".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "query": { "type": "string", "description": "Search query" },
                                "category": { "type": "string", "description": "Optional category" }
                            })),
                            required: Some(vec!["query".to_string()]),
                        }),
                        annotations: None,
                    };

                    let add_tool = Tool {
                        name: "add_torrent".to_string(),
                        description: "Add a new torrent".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "url": { "type": "string", "description": "Magnet URI or HTTP URL" },
                                "save_path": { "type": "string", "description": "Optional save path" },
                                "category": { "type": "string", "description": "Optional category" }
                            })),
                            required: Some(vec!["url".to_string()]),
                        }),
                        annotations: None,
                    };

                    let pause_tool = Tool {
                        name: "pause_torrent".to_string(),
                        description: "Pause a torrent".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "hash": { "type": "string", "description": "Torrent hash (pipe-separated for multiple)" }
                            })),
                            required: Some(vec!["hash".to_string()]),
                        }),
                        annotations: None,
                    };

                    let resume_tool = Tool {
                        name: "resume_torrent".to_string(),
                        description: "Resume a torrent".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "hash": { "type": "string", "description": "Torrent hash (pipe-separated for multiple)" }
                            })),
                            required: Some(vec!["hash".to_string()]),
                        }),
                        annotations: None,
                    };

                    let delete_tool = Tool {
                        name: "delete_torrent".to_string(),
                        description: "Delete a torrent".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "hash": { "type": "string", "description": "Torrent hash (pipe-separated for multiple)" },
                                "delete_files": { "type": "boolean", "description": "Also delete files" }
                            })),
                            required: Some(vec!["hash".to_string(), "delete_files".to_string()]),
                        }),
                        annotations: None,
                    };

                    let files_tool = Tool {
                        name: "get_torrent_files".to_string(),
                        description: "Get file list of a torrent".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "hash": { "type": "string", "description": "Torrent hash" }
                            })),
                            required: Some(vec!["hash".to_string()]),
                        }),
                        annotations: None,
                    };

                    let props_tool = Tool {
                        name: "get_torrent_properties".to_string(),
                        description: "Get properties of a torrent".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "hash": { "type": "string", "description": "Torrent hash" }
                            })),
                            required: Some(vec!["hash".to_string()]),
                        }),
                        annotations: None,
                    };

                    let transfer_tool = Tool {
                        name: "get_global_transfer_info".to_string(),
                        description: "Get global transfer information".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({})),
                            required: None,
                    }),
                    annotations: None,
                    };

                    let set_limits_tool = Tool {
                        name: "set_global_transfer_limits".to_string(),
                        description: "Set global download and/or upload limits".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "dl_limit": { "type": "integer", "description": "Download limit in bytes per second (0 for unlimited)" },
                                "up_limit": { "type": "integer", "description": "Upload limit in bytes per second (0 for unlimited)" }
                            })),
                            required: None,
                        }),
                        annotations: None,
                    };

                    tools.extend(vec![search_tool, add_tool, pause_tool, resume_tool, delete_tool, files_tool, props_tool, transfer_tool, set_limits_tool]);
                }

                Ok(json!({
                    "tools": tools
                }))
            }
            "tools/call" => {
                let params = params.ok_or(McpError::protocol(
                    ErrorCode::InvalidParams,
                    "Missing params",
                ))?;
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing name"))?;
                
                let arguments = params.get("arguments");

                if name == "list_torrents" {
                    let torrents =
                        self.client.get_torrent_list().await.map_err(|e| {
                            McpError::protocol(ErrorCode::InternalError, e.to_string())
                        })?;

                    let text = serde_json::to_string_pretty(&torrents)
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [
                            {
                                "type": "text",
                                "text": text
                            }
                        ],
                        "isError": false
                    }))
                } else if name == "search_torrents" {
                    let args = arguments.ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing arguments"))?;
                    let query = args.get("query").and_then(|v| v.as_str()).ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing query"))?;
                    let category = args.get("category").and_then(|v| v.as_str());

                    // Start search
                    let id = self.client.start_search(query, category).await.map_err(|e| McpError::protocol(ErrorCode::InternalError, format!("Failed to start search: {}", e)))?;

                    // Poll results for 5 seconds (5 checks)
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

                    // Stop and delete
                    let _ = self.client.stop_search(id).await;
                    let _ = self.client.delete_search(id).await;

                    let text = serde_json::to_string_pretty(&final_results).map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "add_torrent" {
                    let args = arguments.ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing arguments"))?;
                    let url = args.get("url").and_then(|v| v.as_str()).ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing url"))?;
                    let save_path = args.get("save_path").and_then(|v| v.as_str());
                    let category = args.get("category").and_then(|v| v.as_str());

                    self.client.add_torrent(url, save_path, category).await.map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Torrent added successfully" }],
                        "isError": false
                    }))
                } else if name == "pause_torrent" {
                    let args = arguments.ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing arguments"))?;
                    let hash = args.get("hash").and_then(|v| v.as_str()).ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing hash"))?;

                    self.client.pause_torrents(hash).await.map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Torrent paused successfully" }],
                        "isError": false
                    }))
                } else if name == "resume_torrent" {
                    let args = arguments.ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing arguments"))?;
                    let hash = args.get("hash").and_then(|v| v.as_str()).ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing hash"))?;

                    self.client.resume_torrents(hash).await.map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Torrent resumed successfully" }],
                        "isError": false
                    }))
                } else if name == "delete_torrent" {
                    let args = arguments.ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing arguments"))?;
                    let hash = args.get("hash").and_then(|v| v.as_str()).ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing hash"))?;
                    let delete_files = args.get("delete_files").and_then(|v| v.as_bool()).unwrap_or(false);

                    self.client.delete_torrents(hash, delete_files).await.map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Torrent deleted successfully" }],
                        "isError": false
                    }))
                } else if name == "get_torrent_files" {
                    let args = arguments.ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing arguments"))?;
                    let hash = args.get("hash").and_then(|v| v.as_str()).ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing hash"))?;

                    let files = self.client.get_torrent_files(hash).await.map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;
                    let text = serde_json::to_string_pretty(&files).map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "get_torrent_properties" {
                    let args = arguments.ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing arguments"))?;
                    let hash = args.get("hash").and_then(|v| v.as_str()).ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing hash"))?;

                    let props = self.client.get_torrent_properties(hash).await.map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;
                    let text = serde_json::to_string_pretty(&props).map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "get_global_transfer_info" {
                    let info = self.client.get_global_transfer_info().await.map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;
                    let text = serde_json::to_string_pretty(&info).map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "set_global_transfer_limits" {
                    let args = arguments.ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing arguments"))?;
                    let dl_limit = args.get("dl_limit").and_then(|v| v.as_i64());
                    let up_limit = args.get("up_limit").and_then(|v| v.as_i64());

                    if let Some(limit) = dl_limit {
                        self.client.set_download_limit(limit).await.map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;
                    }
                    if let Some(limit) = up_limit {
                        self.client.set_upload_limit(limit).await.map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;
                    }

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Transfer limits updated successfully" }],
                        "isError": false
                    }))
                } else if name == "show_all_tools" {
                    self.lazy_mode.store(false, Ordering::Relaxed);
                    // We should ideally send a notification here: `notifications/tools/list_changed`
                    // But mcp-sdk-rs doesn't easily expose the notification mechanism here yet.
                    // The client will need to refresh tools manually if possible, or we rely on the client noticing.
                    // For now, we return a message telling the agent to refresh.
                    Ok(json!({
                        "content": [{ "type": "text", "text": "All tools enabled. Please refresh your tool list." }],
                        "isError": false
                    }))
                } else {
                    Err(McpError::protocol(
                        ErrorCode::MethodNotFound,
                        format!("Tool not found: {}", name),
                    ))
                }
            }
            _ => Err(McpError::protocol(
                ErrorCode::MethodNotFound,
                method.to_string(),
            )),
        }
    }
}
