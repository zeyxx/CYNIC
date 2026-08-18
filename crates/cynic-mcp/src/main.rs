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

mod project_root;

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
        eprintln!("  CYNIC_KERNEL_SOCKET   Injected local kernel Unix socket path");
        eprintln!("  CYNIC_AGENT_SESSION_TTL_MS   Bounded local session TTL (10000-3600000)");
        eprintln!("  CYNIC_AGENT_CAPABILITIES_JSON   Explicit requested capability grants");
        eprintln!("  CYNIC_AGENT_ID    Agent principal bound to this MCP process");
        eprintln!("  CYNIC_AGENT_TYPE  Agent kind used when generating a process principal");
        eprintln!("  CYNIC_MCP_TRANSPORT  stdio (default) or streamable_http");
        eprintln!("  CYNIC_MCP_LISTEN  Bind address for streamable_http (default 127.0.0.1:8787)");
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

    let project_root = project_root::resolve_mcp_project_root()
        .display()
        .to_string();
    let session_identity =
        cynic_mcp::session_identity::SessionIdentity::resolve().map_err(std::io::Error::other)?;
    let socket_path = cynic_mcp::kernel_session::declared_kernel_socket();
    let kernel =
        cynic_mcp::kernel_session::KernelSessionClient::bootstrap(&socket_path, &session_identity)
            .await
            .map_err(|error| std::io::Error::other(format!("blocked_disconnected: {error}")))?;

    tracing::info!(
        socket_path = %socket_path.display(),
        kernel_node_id = kernel.node_id(),
        agent_id = %session_identity.agent_id,
        "cynic-mcp starting with a bounded local agent session"
    );

    let session_for_shutdown = kernel.clone();
    let proxy = cynic_mcp::proxy::CynicMcpProxy::new(kernel, session_identity, project_root);

    let shutdown = CancellationToken::new();
    {
        let sd = shutdown.clone();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            sd.cancel();
        });
    }

    let renewal_task = {
        let session = session_for_shutdown.clone();
        let renewal_shutdown = shutdown.clone();
        tokio::spawn(async move {
            if let Err(error) = session.maintain(renewal_shutdown.clone()).await {
                tracing::error!(error, "agent session renewal stopped");
                renewal_shutdown.cancel();
            }
        })
    };

    let transport_mode = std::env::var("CYNIC_MCP_TRANSPORT")
        .unwrap_or_else(|_| "stdio".to_string())
        .trim()
        .to_ascii_lowercase();
    match transport_mode.as_str() {
        "stdio" => {
            let transport = rmcp::transport::io::stdio();
            let server = proxy
                .serve(transport)
                .await
                .map_err(|error| std::io::Error::other(format!("MCP proxy error: {error}")))?;

            tokio::select! {
                _ = server.waiting() => {}
                _ = shutdown.cancelled() => {
                    tracing::info!("cynic-mcp shutting down");
                }
            }
        }
        cynic_mcp::transport::STREAMABLE_HTTP_MODE => {
            let listen =
                std::env::var("CYNIC_MCP_LISTEN").unwrap_or_else(|_| "127.0.0.1:8787".to_string());
            cynic_mcp::transport::serve_streamable_http(proxy, &listen, shutdown.clone())
                .await
                .map_err(|error| {
                    std::io::Error::other(format!("MCP HTTP transport error: {error}"))
                })?;
        }
        other => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "unsupported CYNIC_MCP_TRANSPORT={other}; expected stdio or streamable_http"
                ),
            )
            .into());
        }
    }

    shutdown.cancel();
    if let Err(error) = renewal_task.await {
        tracing::warn!(%error, "agent session renewal task join failed");
    }

    if let Err(error) = session_for_shutdown.revoke().await {
        tracing::warn!(error, "agent session revocation read-back failed");
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
    let desired_contract_digest = cynic_protocol::digest_json(&desired)
        .map_err(|error| format!("cannot digest desired MCP contract: {error}"))?;
    let compiled_contract_digest = cynic_protocol::digest_json(&compiled)
        .map_err(|error| format!("cannot digest compiled MCP contract: {error}"))?;
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
        contract_digest: Some(desired_contract_digest),
    };
    let mut compatibility_capabilities = cynic_mcp::protocol::exposed_tool_names(&compiled);
    compatibility_capabilities.extend(cynic_mcp::protocol::source_capabilities(&compiled));
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
            capabilities: Some(compatibility_capabilities),
            contract_digest: Some(compiled_contract_digest),
            evidence_refs: vec![format!("artifact:cynic-mcp:{}", env!("CYNIC_VERSION"))],
            unknowns: Vec::new(),
        },
    ))
}
