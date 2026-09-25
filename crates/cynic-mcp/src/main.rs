<<<<<<< HEAD
//! cynic-mcp binary: thin facade. All logic lives in the library.
=======
#![allow(
    clippy::needless_borrow,
    clippy::collapsible_if,
    clippy::redundant_closure,
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    clippy::redundant_clone,
    clippy::map_err_ignore,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::manual_let_else,
    clippy::uninlined_format_args
)]

mod kernel_discovery;
mod project_root;

use kernel_discovery::{ResolvedKernelEndpoint, resolve_kernel_endpoint};
>>>>>>> github/main
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
<<<<<<< HEAD
    let server = cynic_mcp::server::CynicMcpServer::default();
    let running = server.serve(rmcp::transport::io::stdio()).await?;
    running.waiting().await?;
=======
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
        eprintln!("  CYNIC_REST_ADDR   Explicit kernel origin, verified before MCP startup");
        eprintln!("  CYNIC_KERNEL_SEEDS   Comma-separated kernel origins to scan");
        eprintln!("  CYNIC_KERNEL_DISCOVERY_TIMEOUT_MS   Probe timeout (100-10000, default 1500)");
        eprintln!("  CYNIC_API_KEY     Bearer token for kernel auth");
        eprintln!("  CYNIC_AGENT_ID    Agent principal bound to this MCP process");
        eprintln!(
            "  CYNIC_MCP_PROJECT_ROOT   Optional explicit MCP project root for local diagnostics tooling"
        );
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

    let resolved_kernel = resolve_rest_addr().await?;
    let rest_addr = resolved_kernel.url;
    let project_root = project_root::resolve_mcp_project_root()
        .display()
        .to_string();
    let api_key = std::env::var("CYNIC_API_KEY").unwrap_or_default();

    tracing::info!(
        rest_addr,
        rest_addr_source = resolved_kernel.source,
        kernel_node_id = resolved_kernel.node_id,
        kernel_node_name = resolved_kernel.node_name,
        kernel_role = resolved_kernel.role,
        kernel_registry_status = resolved_kernel.status,
        kernel_discovery_proof = resolved_kernel.proof,
        kernel_hostname = resolved_kernel.hostname,
        kernel_pid = resolved_kernel.pid,
        kernel_version = resolved_kernel.version,
        kernel_git_sha = resolved_kernel.git_sha,
        kernel_build_profile = resolved_kernel.build_profile,
        "cynic-mcp starting — forwarding to REST kernel"
    );

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

>>>>>>> github/main
    Ok(())
}

async fn resolve_rest_addr() -> Result<ResolvedKernelEndpoint, Box<dyn std::error::Error>> {
    let api_key = std::env::var("CYNIC_API_KEY").unwrap_or_default();
    resolve_kernel_endpoint(&api_key)
        .await
        .map_err(|error| error.into())
}
