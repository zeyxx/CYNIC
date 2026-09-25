//! cynic-mcp binary: thin facade. All logic lives in the library.
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = cynic_mcp::server::CynicMcpServer::default();
    let running = server.serve(rmcp::transport::io::stdio()).await?;
    running.waiting().await?;
    Ok(())
}
