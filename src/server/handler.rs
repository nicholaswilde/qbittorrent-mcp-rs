use crate::client::QBitClient;
use anyhow::Result;
use async_trait::async_trait;
use mcp_sdk_rs::Error as McpError;
use mcp_sdk_rs::error::ErrorCode;
use mcp_sdk_rs::server::ServerHandler;
use mcp_sdk_rs::types::{ClientCapabilities, Implementation, ServerCapabilities, Tool, ToolSchema};
use serde_json::{Value, json};

pub struct AppHandler {
    client: QBitClient,
}

impl AppHandler {
    pub fn new(client: QBitClient) -> Self {
        Self { client }
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
                let list_tool = Tool {
                    name: "list_torrents".to_string(),
                    description: "List all torrents".to_string(),
                    input_schema: Some(ToolSchema {
                        properties: Some(json!({})),
                        required: None,
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

                Ok(json!({
                    "tools": [list_tool, add_tool, pause_tool, resume_tool, delete_tool]
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
