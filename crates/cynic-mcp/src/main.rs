use rmcp::ServiceExt;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

fn resolve_rest_addr() -> Result<String, Box<dyn std::error::Error>> {
    let raw_addr = std::env::var("CYNIC_REST_ADDR")
        .map_err(|_err| "CYNIC_REST_ADDR is required by SSOT (no localhost fallback)")?;

    if raw_addr.trim().is_empty() {
        return Err("CYNIC_REST_ADDR is empty".into());
    }

    let rest_addr = if raw_addr.starts_with("http://") || raw_addr.starts_with("https://") {
        raw_addr
    } else {
        format!("http://{raw_addr}")
    };

    let hostport = rest_addr
        .split("://")
        .nth(1)
        .unwrap_or(&rest_addr)
        .split('/')
        .next()
        .unwrap_or("");
    let host = hostport.split(':').next().unwrap_or("");
    if (host == "127.0.0.1" || host == "localhost")
        && std::env::var("CYNIC_ALLOW_LOCAL_KERNEL").unwrap_or_default() != "1"
    {
        return Err(format!("SSOT rejected local kernel address: {rest_addr}").into());
    }

    Ok(rest_addr)
}

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
        eprintln!("  CYNIC_REST_ADDR   Kernel REST address (no localhost default under SSOT)");
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

    let rest_addr = resolve_rest_addr()?;
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
