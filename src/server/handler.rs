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

                    let create_cat_tool = Tool {
                        name: "create_category".to_string(),
                        description: "Create a new category".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "name": { "type": "string", "description": "Category name" },
                                "save_path": { "type": "string", "description": "Save path for category" }
                            })),
                            required: Some(vec!["name".to_string(), "save_path".to_string()]),
                        }),
                        annotations: None,
                    };

                    let set_cat_tool = Tool {
                        name: "set_torrent_category".to_string(),
                        description: "Set category for torrents".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" },
                                "category": { "type": "string", "description": "Category name" }
                            })),
                            required: Some(vec!["hashes".to_string(), "category".to_string()]),
                        }),
                        annotations: None,
                    };

                    let get_cats_tool = Tool {
                        name: "get_categories".to_string(),
                        description: "Get all categories".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({})),
                            required: None,
                        }),
                        annotations: None,
                    };

                    let add_tags_tool = Tool {
                        name: "add_torrent_tags".to_string(),
                        description: "Add tags to torrents".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "hashes": { "type": "string", "description": "Torrent hashes (pipe-separated)" },
                                "tags": { "type": "string", "description": "Comma-separated tags" }
                            })),
                            required: Some(vec!["hashes".to_string(), "tags".to_string()]),
                        }),
                        annotations: None,
                    };

                    let install_plugin_tool = Tool {
                        name: "install_search_plugin".to_string(),
                        description: "Install a search plugin".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "url": { "type": "string", "description": "URL to the plugin file" }
                            })),
                            required: Some(vec!["url".to_string()]),
                        }),
                        annotations: None,
                    };

                    let uninstall_plugin_tool = Tool {
                        name: "uninstall_search_plugin".to_string(),
                        description: "Uninstall a search plugin".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "name": { "type": "string", "description": "Name of the plugin" }
                            })),
                            required: Some(vec!["name".to_string()]),
                        }),
                        annotations: None,
                    };

                    let enable_plugin_tool = Tool {
                        name: "enable_search_plugin".to_string(),
                        description: "Enable or disable a search plugin".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "name": { "type": "string", "description": "Name of the plugin" },
                                "enable": { "type": "boolean", "description": "True to enable, False to disable" }
                            })),
                            required: Some(vec!["name".to_string(), "enable".to_string()]),
                        }),
                        annotations: None,
                    };

                    let update_plugins_tool = Tool {
                        name: "update_search_plugins".to_string(),
                        description: "Update all search plugins".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({})),
                            required: None,
                        }),
                        annotations: None,
                    };

                    let get_plugins_tool = Tool {
                        name: "get_search_plugins".to_string(),
                        description: "List installed search plugins".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({})),
                            required: None,
                        }),
                        annotations: None,
                    };

                    let add_rss_tool = Tool {
                        name: "add_rss_feed".to_string(),
                        description: "Add a new RSS feed".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "url": { "type": "string", "description": "URL of the RSS feed" },
                                "path": { "type": "string", "description": "Internal path/name for the feed" }
                            })),
                            required: Some(vec!["url".to_string(), "path".to_string()]),
                        }),
                        annotations: None,
                    };

                    let get_rss_feeds_tool = Tool {
                        name: "get_rss_feeds".to_string(),
                        description: "Get all RSS feeds and their items".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({})),
                            required: None,
                        }),
                        annotations: None,
                    };

                    let set_rss_rule_tool = Tool {
                        name: "set_rss_rule".to_string(),
                        description: "Create or update an RSS auto-download rule".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "name": { "type": "string", "description": "Name of the rule" },
                                "definition": { "type": "string", "description": "JSON string defining the rule" }
                            })),
                            required: Some(vec!["name".to_string(), "definition".to_string()]),
                        }),
                        annotations: None,
                    };

                    let get_rss_rules_tool = Tool {
                        name: "get_rss_rules".to_string(),
                        description: "Get all RSS auto-download rules".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({})),
                            required: None,
                        }),
                        annotations: None,
                    };

                    let wait_tool = Tool {
                        name: "wait_for_torrent_status".to_string(),
                        description: "Poll a torrent until it reaches a desired state or timeout"
                            .to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "hash": { "type": "string", "description": "Torrent hash" },
                                "target_status": { "type": "string", "description": "Status to wait for (e.g., uploading, stalledUP)" },
                                "timeout_seconds": { "type": "integer", "description": "Max wait time (default 60, max 300)" }
                            })),
                            required: Some(vec!["hash".to_string(), "target_status".to_string()]),
                        }),
                        annotations: None,
                    };

                    let get_app_prefs_tool = Tool {
                        name: "get_app_preferences".to_string(),
                        description: "Get all application preferences".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({})),
                            required: None,
                        }),
                        annotations: None,
                    };

                    let set_app_prefs_tool = Tool {
                        name: "set_app_preferences".to_string(),
                        description: "Set one or more application preferences".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "preferences": { "type": "string", "description": "JSON string of preferences to update" }
                            })),
                            required: Some(vec!["preferences".to_string()]),
                        }),
                        annotations: None,
                    };

                    tools.extend(vec![
                        search_tool,
                        add_tool,
                        pause_tool,
                        resume_tool,
                        delete_tool,
                        files_tool,
                        props_tool,
                        transfer_tool,
                        set_limits_tool,
                        create_cat_tool,
                        set_cat_tool,
                        get_cats_tool,
                        add_tags_tool,
                        install_plugin_tool,
                        uninstall_plugin_tool,
                        enable_plugin_tool,
                        update_plugins_tool,
                        get_plugins_tool,
                        add_rss_tool,
                        get_rss_feeds_tool,
                        set_rss_rule_tool,
                        get_rss_rules_tool,
                        wait_tool,
                        get_app_prefs_tool,
                        set_app_prefs_tool,
                    ]);
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
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let query =
                        args.get("query")
                            .and_then(|v| v.as_str())
                            .ok_or(McpError::protocol(
                                ErrorCode::InvalidParams,
                                "Missing query",
                            ))?;
                    let category = args.get("category").and_then(|v| v.as_str());

                    // Start search
                    let id = self
                        .client
                        .start_search(query, category)
                        .await
                        .map_err(|e| {
                            McpError::protocol(
                                ErrorCode::InternalError,
                                format!("Failed to start search: {}", e),
                            )
                        })?;

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

                    let text = serde_json::to_string_pretty(&final_results)
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "add_torrent" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let url = args
                        .get("url")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing url"))?;
                    let save_path = args.get("save_path").and_then(|v| v.as_str());
                    let category = args.get("category").and_then(|v| v.as_str());

                    self.client
                        .add_torrent(url, save_path, category)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Torrent added successfully" }],
                        "isError": false
                    }))
                } else if name == "pause_torrent" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let hash = args
                        .get("hash")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing hash"))?;

                    self.client
                        .pause_torrents(hash)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Torrent paused successfully" }],
                        "isError": false
                    }))
                } else if name == "resume_torrent" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let hash = args
                        .get("hash")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing hash"))?;

                    self.client
                        .resume_torrents(hash)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Torrent resumed successfully" }],
                        "isError": false
                    }))
                } else if name == "delete_torrent" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let hash = args
                        .get("hash")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing hash"))?;
                    let delete_files = args
                        .get("delete_files")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    self.client
                        .delete_torrents(hash, delete_files)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Torrent deleted successfully" }],
                        "isError": false
                    }))
                } else if name == "get_torrent_files" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let hash = args
                        .get("hash")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing hash"))?;

                    let files =
                        self.client.get_torrent_files(hash).await.map_err(|e| {
                            McpError::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                    let text = serde_json::to_string_pretty(&files)
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "get_torrent_properties" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let hash = args
                        .get("hash")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing hash"))?;

                    let props = self
                        .client
                        .get_torrent_properties(hash)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;
                    let text = serde_json::to_string_pretty(&props)
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "get_global_transfer_info" {
                    let info =
                        self.client.get_global_transfer_info().await.map_err(|e| {
                            McpError::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                    let text = serde_json::to_string_pretty(&info)
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "set_global_transfer_limits" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let dl_limit = args.get("dl_limit").and_then(|v| v.as_i64());
                    let up_limit = args.get("up_limit").and_then(|v| v.as_i64());

                    if let Some(limit) = dl_limit {
                        self.client.set_download_limit(limit).await.map_err(|e| {
                            McpError::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                    }
                    if let Some(limit) = up_limit {
                        self.client.set_upload_limit(limit).await.map_err(|e| {
                            McpError::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                    }

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Transfer limits updated successfully" }],
                        "isError": false
                    }))
                } else if name == "create_category" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let cat_name = args
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing name"))?;
                    let save_path = args.get("save_path").and_then(|v| v.as_str()).ok_or(
                        McpError::protocol(ErrorCode::InvalidParams, "Missing save_path"),
                    )?;

                    self.client
                        .create_category(cat_name, save_path)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Category created successfully" }],
                        "isError": false
                    }))
                } else if name == "set_torrent_category" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let hashes =
                        args.get("hashes")
                            .and_then(|v| v.as_str())
                            .ok_or(McpError::protocol(
                                ErrorCode::InvalidParams,
                                "Missing hashes",
                            ))?;
                    let category =
                        args.get("category")
                            .and_then(|v| v.as_str())
                            .ok_or(McpError::protocol(
                                ErrorCode::InvalidParams,
                                "Missing category",
                            ))?;

                    self.client
                        .set_category(hashes, category)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Category set successfully" }],
                        "isError": false
                    }))
                } else if name == "get_categories" {
                    let categories =
                        self.client.get_categories().await.map_err(|e| {
                            McpError::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                    let text = serde_json::to_string_pretty(&categories)
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "add_torrent_tags" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let hashes =
                        args.get("hashes")
                            .and_then(|v| v.as_str())
                            .ok_or(McpError::protocol(
                                ErrorCode::InvalidParams,
                                "Missing hashes",
                            ))?;
                    let tags = args
                        .get("tags")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing tags"))?;

                    self.client
                        .add_tags(hashes, tags)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Tags added successfully" }],
                        "isError": false
                    }))
                } else if name == "install_search_plugin" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let url = args
                        .get("url")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing url"))?;

                    self.client
                        .install_search_plugin(url)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Search plugin installed successfully" }],
                        "isError": false
                    }))
                } else if name == "uninstall_search_plugin" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let plugin_name = args
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing name"))?;

                    self.client
                        .uninstall_search_plugin(plugin_name)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Search plugin uninstalled successfully" }],
                        "isError": false
                    }))
                } else if name == "enable_search_plugin" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let plugin_name = args
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing name"))?;
                    let enable =
                        args.get("enable")
                            .and_then(|v| v.as_bool())
                            .ok_or(McpError::protocol(
                                ErrorCode::InvalidParams,
                                "Missing enable",
                            ))?;

                    self.client
                        .enable_search_plugin(plugin_name, enable)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Search plugin status updated successfully" }],
                        "isError": false
                    }))
                } else if name == "update_search_plugins" {
                    self.client
                        .update_search_plugins()
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "Search plugins updated successfully" }],
                        "isError": false
                    }))
                } else if name == "get_search_plugins" {
                    let plugins =
                        self.client.get_search_plugins().await.map_err(|e| {
                            McpError::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                    let text = serde_json::to_string_pretty(&plugins)
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "add_rss_feed" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let url = args
                        .get("url")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing url"))?;
                    let path = args
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing path"))?;

                    self.client
                        .add_rss_feed(url, path)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "RSS feed added successfully" }],
                        "isError": false
                    }))
                } else if name == "get_rss_feeds" {
                    let feeds =
                        self.client.get_all_rss_feeds().await.map_err(|e| {
                            McpError::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                    let text = serde_json::to_string_pretty(&feeds)
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "set_rss_rule" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let rule_name = args
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing name"))?;
                    let definition = args.get("definition").and_then(|v| v.as_str()).ok_or(
                        McpError::protocol(ErrorCode::InvalidParams, "Missing definition"),
                    )?;

                    self.client
                        .set_rss_rule(rule_name, definition)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "RSS rule set successfully" }],
                        "isError": false
                    }))
                } else if name == "get_rss_rules" {
                    let rules =
                        self.client.get_all_rss_rules().await.map_err(|e| {
                            McpError::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                    let text = serde_json::to_string_pretty(&rules)
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "get_app_preferences" {
                    let prefs =
                        self.client.get_app_preferences().await.map_err(|e| {
                            McpError::protocol(ErrorCode::InternalError, e.to_string())
                        })?;
                    let text = serde_json::to_string_pretty(&prefs)
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                } else if name == "set_app_preferences" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let prefs_str = args.get("preferences").and_then(|v| v.as_str()).ok_or(
                        McpError::protocol(ErrorCode::InvalidParams, "Missing preferences"),
                    )?;
                    let prefs_val: serde_json::Value =
                        serde_json::from_str(prefs_str).map_err(|e| {
                            McpError::protocol(
                                ErrorCode::InvalidParams,
                                format!("Invalid JSON: {}", e),
                            )
                        })?;

                    self.client
                        .set_app_preferences(&prefs_val)
                        .await
                        .map_err(|e| McpError::protocol(ErrorCode::InternalError, e.to_string()))?;

                    Ok(json!({
                        "content": [{ "type": "text", "text": "App preferences updated successfully" }],
                        "isError": false
                    }))
                } else if name == "wait_for_torrent_status" {
                    let args = arguments.ok_or(McpError::protocol(
                        ErrorCode::InvalidParams,
                        "Missing arguments",
                    ))?;
                    let hash = args
                        .get("hash")
                        .and_then(|v| v.as_str())
                        .ok_or(McpError::protocol(ErrorCode::InvalidParams, "Missing hash"))?;
                    let target_status = args.get("target_status").and_then(|v| v.as_str()).ok_or(
                        McpError::protocol(ErrorCode::InvalidParams, "Missing target_status"),
                    )?;
                    let timeout = args
                        .get("timeout_seconds")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(60);
                    let timeout = timeout.clamp(1, 300);

                    let mut current_status = String::new();
                    let start_time = std::time::Instant::now();

                    while start_time.elapsed().as_secs() < timeout as u64 {
                        let torrents = self.client.get_torrents_info(hash).await.map_err(|e| {
                            McpError::protocol(ErrorCode::InternalError, e.to_string())
                        })?;

                        if let Some(t) = torrents.first() {
                            current_status = t.state.clone();
                            if current_status == target_status {
                                return Ok(json!({
                                    "content": [{ "type": "text", "text": format!("Torrent reached target status: {}", target_status) }],
                                    "isError": false
                                }));
                            }
                        } else {
                            return Err(McpError::protocol(
                                ErrorCode::InvalidParams,
                                format!("Torrent not found: {}", hash),
                            ));
                        }

                        sleep(Duration::from_secs(2)).await;
                    }

                    Ok(json!({
                        "content": [{ "type": "text", "text": format!("Timed out waiting for status {}. Current status: {}", target_status, current_status) }],
                        "isError": true
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
