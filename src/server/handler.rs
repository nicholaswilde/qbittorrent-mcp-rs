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
                let tool = Tool {
                    name: "list_torrents".to_string(),
                    description: "List all torrents".to_string(),
                    input_schema: Some(ToolSchema {
                        // type_str? If not available, maybe it's implicitly object or I need to check ToolSchema def.
                        // Error said available fields: `required`. Maybe I missed something or `properties` is there but not listed?
                        // Let's assume properties exists but needs Option.
                        // And type? Maybe it's `type_: String`?
                        // I'll try to omit type and just provide properties.
                        properties: Some(json!({})),
                        required: None,
                    }),
                    annotations: None,
                };

                Ok(json!({
                    "tools": [tool]
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
