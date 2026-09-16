//! CYNIC Observer Daemon Entrypoint

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    cynic_observer::runner::run_argo_node().await
}
