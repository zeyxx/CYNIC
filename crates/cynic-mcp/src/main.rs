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
use rmcp::ServiceExt;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
#[allow(clippy::print_stderr)]
// WHY: MCP binary must write to stderr for --version/--help; stdout is the JSON-RPC channel.
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--contract-json") {
        let report = runtime_contract_report()?;
        println!("{}", serde_json::to_string(&report)?);
        if report.status != cynic_protocol::RuntimeCompatibilityStatus::Compatible {
            std::process::exit(3);
        }
        return Ok(());
    }

    if std::env::args().any(|a| a == "--version") {
        eprintln!("cynic-mcp {}", env!("CYNIC_VERSION"));
        return Ok(());
    }

    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprintln!("cynic-mcp — MCP-to-REST proxy for CYNIC kernel");
        eprintln!();
        eprintln!("USAGE: cynic-mcp");
        eprintln!("       cynic-mcp --contract-json");
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

    Ok(())
}

fn runtime_contract_report() -> Result<cynic_protocol::RuntimeCompatibilityPlan, String> {
    let compiled: serde_json::Value =
        serde_json::from_str(include_str!("../../../infra/agent-exposure.json"))
            .map_err(|error| format!("compiled MCP contract is invalid: {error}"))?;
    let project_root = project_root::resolve_mcp_project_root();
    let desired_bytes = std::fs::read(project_root.join("infra/agent-exposure.json"))
        .map_err(|error| format!("desired MCP contract is unobservable: {error}"))?;
    let desired: serde_json::Value = serde_json::from_slice(&desired_bytes)
        .map_err(|error| format!("desired MCP contract is invalid: {error}"))?;
    let desired_handshake = desired
        .get("session_handshake")
        .ok_or_else(|| "desired MCP handshake is absent".to_string())?;
    let compiled_handshake = compiled
        .get("session_handshake")
        .ok_or_else(|| "compiled MCP handshake is absent".to_string())?;
    let requirement = cynic_protocol::RuntimeContractRequirement {
        contract_id: desired_handshake
            .get("contract_id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        contract_revision: desired_handshake
            .get("contract_revision")
            .and_then(|value| value.as_u64())
            .unwrap_or_default(),
        required_capabilities: desired_handshake
            .get("required_capabilities")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str())
            .map(ToString::to_string)
            .collect(),
    };
    Ok(cynic_protocol::reconcile_runtime_compatibility(
        &requirement,
        &cynic_protocol::RuntimeContractObservation {
            component: "cynic-mcp".to_string(),
            artifact_version: Some(env!("CYNIC_VERSION").to_string()),
            contract_id: compiled_handshake
                .get("contract_id")
                .and_then(|value| value.as_str())
                .map(ToString::to_string),
            contract_revision: compiled_handshake
                .get("contract_revision")
                .and_then(|value| value.as_u64()),
            capabilities: Some(active_mcp_tools(&compiled)),
            evidence_refs: vec![format!("artifact:cynic-mcp:{}", env!("CYNIC_VERSION"))],
            unknowns: Vec::new(),
        },
    ))
}

fn active_mcp_tools(exposure: &serde_json::Value) -> Vec<String> {
    exposure
        .get("mcp_servers")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .find(|server| server.get("name").and_then(|value| value.as_str()) == Some("cynic"))
        .and_then(|server| server.get("tools"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|tool| tool.get("name").and_then(|value| value.as_str()))
        .map(ToString::to_string)
        .collect()
}

async fn resolve_rest_addr() -> Result<ResolvedKernelEndpoint, Box<dyn std::error::Error>> {
    let api_key = std::env::var("CYNIC_API_KEY").unwrap_or_default();
    resolve_kernel_endpoint(&api_key)
        .await
        .map_err(|error| error.into())
}
