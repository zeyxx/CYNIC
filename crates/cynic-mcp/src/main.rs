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

use rmcp::ServiceExt;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[derive(Debug)]
struct ResolvedRestAddr {
    url: String,
    source: String,
}

fn normalize_rest_addr(raw_addr: &str) -> Result<String, Box<dyn std::error::Error>> {
    if raw_addr.trim().is_empty() {
        return Err("CYNIC_REST_ADDR is empty".into());
    }

    let rest_addr = if raw_addr.starts_with("http://") || raw_addr.starts_with("https://") {
        raw_addr.to_string()
    } else {
        format!("http://{raw_addr}")
    };

    Ok(rest_addr)
}

fn rest_addr_host(rest_addr: &str) -> &str {
    let hostport = rest_addr
        .split("://")
        .nth(1)
        .unwrap_or(&rest_addr)
        .split('/')
        .next()
        .unwrap_or("");
    hostport.split(':').next().unwrap_or("")
}

fn is_local_rest_addr(rest_addr: &str) -> bool {
    matches!(rest_addr_host(rest_addr), "127.0.0.1" | "localhost" | "::1")
}

fn project_root() -> std::path::PathBuf {
    std::env::var("CYNIC_PROJECT_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        })
}

fn registry_kernel_rest_addr() -> Result<ResolvedRestAddr, Box<dyn std::error::Error>> {
    let registry_path = project_root().join("infra/registry.json");
    let content = std::fs::read_to_string(&registry_path)
        .map_err(|e| format!("cannot read {}: {e}", registry_path.display()))?;
    let registry: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("cannot parse {}: {e}", registry_path.display()))?;

    let deploy_role = registry
        .pointer("/lifecycle/kernel_release/deploy_target_role")
        .and_then(|v| v.as_str())
        .unwrap_or("kernel_primary");

    let nodes = registry
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or("infra/registry.json has no nodes array")?;

    let node = nodes
        .iter()
        .find(|node| {
            node.get("role").and_then(|v| v.as_str()) == Some(deploy_role)
                && node
                    .get("deploy_target")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
        })
        .or_else(|| {
            nodes
                .iter()
                .find(|node| node.get("role").and_then(|v| v.as_str()) == Some(deploy_role))
        })
        .ok_or_else(|| format!("infra/registry.json has no node with role={deploy_role}"))?;

    let host = node
        .pointer("/network/synapse")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            format!("registry node role={deploy_role} has no network.synapse address")
        })?;

    let port = node
        .get("services")
        .and_then(|v| v.as_array())
        .and_then(|services| {
            services.iter().find_map(|service| {
                if service.get("name").and_then(|v| v.as_str()) == Some("cynic-kernel") {
                    service.get("port").and_then(|v| v.as_u64())
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| {
            format!("registry node role={deploy_role} has no cynic-kernel service port")
        })?;

    Ok(ResolvedRestAddr {
        url: format!("http://{host}:{port}"),
        source: format!("registry:{deploy_role}.network.synapse+cynic-kernel.port"),
    })
}

fn resolve_rest_addr() -> Result<ResolvedRestAddr, Box<dyn std::error::Error>> {
    if let Ok(raw_addr) = std::env::var("CYNIC_REST_ADDR") {
        let env_addr = normalize_rest_addr(&raw_addr)?;
        let local_allowed = std::env::var("CYNIC_ALLOW_LOCAL_KERNEL").unwrap_or_default() == "1";
        if !is_local_rest_addr(&env_addr) || local_allowed {
            return Ok(ResolvedRestAddr {
                url: env_addr,
                source: "env:CYNIC_REST_ADDR".to_string(),
            });
        }

        if let Ok(registry_addr) = registry_kernel_rest_addr() {
            return Ok(registry_addr);
        }

        return Err(format!(
            "SSOT rejected local kernel address: {env_addr}; set CYNIC_ALLOW_LOCAL_KERNEL=1 for dev or fix infra/registry.json discovery"
        )
        .into());
    }

    registry_kernel_rest_addr()
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

    let resolved_rest_addr = resolve_rest_addr()?;
    let rest_addr = resolved_rest_addr.url;
    let api_key = std::env::var("CYNIC_API_KEY").unwrap_or_default();
    let project_root = project_root().display().to_string();

    tracing::info!(
        rest_addr,
        rest_addr_source = resolved_rest_addr.source,
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

    Ok(())
}
