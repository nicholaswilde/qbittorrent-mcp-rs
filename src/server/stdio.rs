use crate::client::QBitClient;
use crate::server::MCPServer;
use crate::server::handler::AppHandler;
use anyhow::Result;
use async_trait::async_trait;
use mcp_sdk_rs::server::Server;
use mcp_sdk_rs::transport::stdio::StdioTransport;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

pub struct StdioServer {
    client: QBitClient,
    lazy_mode: bool,
}

impl Default for StdioServer {
    fn default() -> Self {
        panic!("StdioServer requires a client");
    }
}

impl StdioServer {
    pub fn new(client: QBitClient, lazy_mode: bool) -> Self {
        Self { client, lazy_mode }
    }
}

#[async_trait]
impl MCPServer for StdioServer {
    async fn run(&self) -> Result<()> {
        let (read_tx, read_rx) = mpsc::channel::<String>(100);
        let (write_tx, mut write_rx) = mpsc::channel::<String>(100);

        // Task to read stdin
        tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let mut reader = BufReader::new(stdin).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if read_tx.send(line).await.is_err() {
                    break;
                }
            }
        });

        // Task to write stdout
        tokio::spawn(async move {
            let mut stdout = tokio::io::stdout();
            while let Some(msg) = write_rx.recv().await {
                let _ = stdout.write_all(msg.as_bytes()).await;
                let _ = stdout.write_all(b"\n").await;
                let _ = stdout.flush().await;
            }
        });

        let transport = Arc::new(StdioTransport::new(read_rx, write_tx));
        let handler = Arc::new(AppHandler::new(self.client.clone(), self.lazy_mode));
        let server = Server::new(transport, handler);

        server.start().await.map_err(|e| anyhow::anyhow!(e))
    }
}
