use crate::client::QBitClient;
use crate::server::MCPServer;
use anyhow::Result;
use async_trait::async_trait;
use axum::{Router, response::IntoResponse, routing::get};
use std::net::SocketAddr;

pub struct HttpServer {
    port: u16,
    #[allow(dead_code)]
    client: QBitClient,
}

impl HttpServer {
    pub fn new(port: u16, client: QBitClient) -> Self {
        Self { port, client }
    }
}

#[async_trait]
impl MCPServer for HttpServer {
    async fn run(&self) -> Result<()> {
        let app = Router::new().route("/sse", get(sse_handler));

        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        println!("HTTP Server listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn sse_handler() -> impl IntoResponse {
    "SSE Handler (Placeholder)"
}
