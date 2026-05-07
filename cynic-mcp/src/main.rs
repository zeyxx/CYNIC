use rmcp::ServiceExt;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
#[allow(clippy::print_stderr)]
// WHY: MCP binary must write to stderr for --version/--help; stdout is the JSON-RPC channel.
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--version") {
        eprintln!("cynic-mcp {}", env!("CYNIC_VERSION"));
        return Ok(());
    }

    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprintln!("cynic-mcp — MCP-to-REST proxy for CYNIC kernel");
        eprintln!();
        eprintln!("USAGE: cynic-mcp");
        eprintln!();
        eprintln!("ENVIRONMENT:");
        eprintln!("  CYNIC_REST_ADDR   Kernel REST address (default: http://127.0.0.1:3030)");
        eprintln!("  CYNIC_API_KEY     Bearer token for kernel auth");
        eprintln!("  RUST_LOG          Log filter (default: cynic_mcp=info,warn)");
        return Ok(());
    }

    // Stdout guard: MCP uses stdio for JSON-RPC. All logging goes to stderr.
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("cynic_mcp=info,warn"));
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().json().with_writer(std::io::stderr))
        .init();

    let raw_addr =
        std::env::var("CYNIC_REST_ADDR").unwrap_or_else(|_| "http://127.0.0.1:3030".into());
    let rest_addr = if raw_addr.starts_with("http://") || raw_addr.starts_with("https://") {
        raw_addr
    } else {
        format!("http://{raw_addr}")
    };
    let api_key = std::env::var("CYNIC_API_KEY").unwrap_or_default();
    let project_root = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .display()
        .to_string();

    tracing::info!(rest_addr, "cynic-mcp starting — forwarding to REST kernel");

    let proxy = cynic_mcp::proxy::CynicMcpProxy::new(rest_addr, api_key, project_root);

    let shutdown = CancellationToken::new();
    {
        let sd = shutdown.clone();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            sd.cancel();
        });
    }

    let transport = rmcp::transport::io::stdio();
    let server = proxy
        .serve(transport)
        .await
        .map_err(|e| format!("MCP proxy error: {e}"))?;

    tokio::select! {
        _ = server.waiting() => {}
        _ = shutdown.cancelled() => {
            tracing::info!("cynic-mcp shutting down");
        }
    }

    Ok(())
}
