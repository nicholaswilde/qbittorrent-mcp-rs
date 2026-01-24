use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait MCPServer {
    async fn run(&self) -> Result<()>;
}

// We can have a ServerFactory or similar if needed.
// For now, simple trait is enough.
pub mod handler;
pub mod http;
pub mod stdio;
