use mcp_sdk_rs::transport::stdio::StdioTransport;
pub fn check() {
    let _ = StdioTransport::new();
}
