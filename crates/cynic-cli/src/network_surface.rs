use std::process::ExitCode;

use clap::Subcommand;
use cynic_node::NetworkCommands;
use serde_json::json;

use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};
use crate::{emit_json, fail, registry_path, runtime_paths};

#[derive(Subcommand)]
pub(crate) enum NetworkCommand {
    Chain {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        resolve_dns: bool,
        #[arg(long)]
        check_provider_connectivity: bool,
    },
    Runtime {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        resolve_dns: bool,
        #[arg(long)]
        check_provider_connectivity: bool,
    },
    Proof {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        resolve_dns: bool,
        #[arg(long)]
        check_provider_connectivity: bool,
    },
    Evidence {
        #[arg(long)]
        registry: Option<String>,
    },
    Dns {
        #[command(subcommand)]
        command: NetworkDnsCommand,
    },
    Mesh {
        #[command(subcommand)]
        command: NetworkMeshCommand,
    },
    ProviderFallback {
        #[command(subcommand)]
        command: NetworkProviderFallbackCommand,
    },
    Actuator {
        #[command(subcommand)]
        command: NetworkActuatorCommand,
    },
}

#[derive(Subcommand)]
pub(crate) enum NetworkDnsCommand {
    Status {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        resolve_dns: bool,
        #[arg(long)]
        hosts_path: Option<String>,
        #[arg(long)]
        corefile_path: Option<String>,
        #[arg(long)]
        service_path: Option<String>,
        #[arg(long)]
        resolved_dropin_path: Option<String>,
    },
    Bundle {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        output_dir: String,
    },
    Plan,
}

#[derive(Subcommand)]
pub(crate) enum NetworkMeshCommand {
    Plan {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        provider: Option<String>,
    },
    Bundle {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        output_dir: String,
    },
    ApplyPlan {
        #[arg(long)]
        bundle_dir: String,
    },
    ApplyProfile {
        #[arg(long)]
        bundle_dir: String,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        closed_environment: bool,
    },
    Status {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        resolve_dns: bool,
        #[arg(long)]
        include_records: bool,
        #[arg(long)]
        observe_runtime: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum NetworkProviderFallbackCommand {
    Status {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        hosts_path: Option<String>,
        #[arg(long)]
        check_connectivity: bool,
    },
    Hosts {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        output_dir: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum NetworkActuatorCommand {
    Status {
        #[arg(long)]
        registry: Option<String>,
    },
    Bundle {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        output_dir: String,
    },
    Plan {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long, default_value = "/tmp/cynic-network-actuator")]
        bundle_dir: String,
    },
    Apply {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        apply: bool,
    },
}

pub(crate) fn execute_network(command: NetworkCommand) -> ExitCode {
    let runtime = resolve_runtime_paths(&RuntimeContext::from_env(None));
    match command {
        NetworkCommand::Chain {
            registry,
            resolve_dns,
            check_provider_connectivity,
        } => cynic_node::execute_network_command(&NetworkCommands::ActivationChain {
            json: true,
            registry: registry_path(registry),
            resolve_dns,
            check_provider_connectivity,
            provider_hosts_path: runtime.provider_hosts_path.to_string_lossy().to_string(),
            hosts_path: runtime.dns_hosts_path.to_string_lossy().to_string(),
            corefile_path: runtime.dns_corefile_path.to_string_lossy().to_string(),
            service_path: runtime.dns_service_path.to_string_lossy().to_string(),
            resolved_dropin_path: runtime.resolved_dropin_path.to_string_lossy().to_string(),
        }),
        NetworkCommand::Runtime {
            registry,
            resolve_dns,
            check_provider_connectivity,
        } => cynic_node::execute_network_command(&NetworkCommands::RuntimeStatus {
            json: true,
            registry: registry_path(registry),
            resolve_dns,
            check_provider_connectivity,
            provider_hosts_path: runtime.provider_hosts_path.to_string_lossy().to_string(),
            hosts_path: runtime.dns_hosts_path.to_string_lossy().to_string(),
            corefile_path: runtime.dns_corefile_path.to_string_lossy().to_string(),
            service_path: runtime.dns_service_path.to_string_lossy().to_string(),
            resolved_dropin_path: runtime.resolved_dropin_path.to_string_lossy().to_string(),
        }),
        NetworkCommand::Proof {
            registry,
            resolve_dns,
            check_provider_connectivity,
        } => cynic_node::execute_network_command(&NetworkCommands::RuntimeProof {
            json: true,
            registry: registry_path(registry),
            resolve_dns,
            check_provider_connectivity,
            provider_hosts_path: runtime.provider_hosts_path.to_string_lossy().to_string(),
            hosts_path: runtime.dns_hosts_path.to_string_lossy().to_string(),
            corefile_path: runtime.dns_corefile_path.to_string_lossy().to_string(),
            service_path: runtime.dns_service_path.to_string_lossy().to_string(),
            resolved_dropin_path: runtime.resolved_dropin_path.to_string_lossy().to_string(),
        }),
        NetworkCommand::Evidence { registry } => {
            cynic_node::execute_network_command(&NetworkCommands::Evidence {
                json: true,
                registry: registry_path(registry),
            })
        }
        NetworkCommand::Dns { command } => match command {
            NetworkDnsCommand::Status {
                registry,
                resolve_dns,
                hosts_path,
                corefile_path,
                service_path,
                resolved_dropin_path,
            } => cynic_node::execute_network_command(&NetworkCommands::DnsActivationStatus {
                json: true,
                registry: registry_path(registry),
                hosts_path: hosts_path
                    .unwrap_or_else(|| runtime.dns_hosts_path.to_string_lossy().to_string()),
                corefile_path: corefile_path
                    .unwrap_or_else(|| runtime.dns_corefile_path.to_string_lossy().to_string()),
                service_path: service_path
                    .unwrap_or_else(|| runtime.dns_service_path.to_string_lossy().to_string()),
                resolved_dropin_path: resolved_dropin_path
                    .unwrap_or_else(|| runtime.resolved_dropin_path.to_string_lossy().to_string()),
                resolve_dns,
            }),
            NetworkDnsCommand::Bundle {
                registry,
                output_dir,
            } => cynic_node::execute_network_command(&NetworkCommands::DnsBundle {
                json: true,
                registry: registry_path(registry),
                output_dir,
            }),
            NetworkDnsCommand::Plan => {
                cynic_node::execute_network_command(&NetworkCommands::DnsPlan { json: true })
            }
        },
        NetworkCommand::Mesh { command } => match command {
            NetworkMeshCommand::Plan { registry, provider } => {
                cynic_node::execute_network_command(&NetworkCommands::MeshPlan {
                    json: true,
                    registry: registry_path(registry),
                    provider,
                })
            }
            NetworkMeshCommand::Bundle {
                registry,
                provider,
                output_dir,
            } => cynic_node::execute_network_command(&NetworkCommands::MeshBundle {
                json: true,
                registry: registry_path(registry),
                provider,
                output_dir,
            }),
            NetworkMeshCommand::ApplyPlan { bundle_dir } => {
                cynic_node::execute_network_command(&NetworkCommands::MeshApplyPlan {
                    json: true,
                    bundle_dir,
                })
            }
            NetworkMeshCommand::ApplyProfile {
                bundle_dir,
                profile,
                apply,
                closed_environment,
            } => cynic_node::execute_network_command(&NetworkCommands::MeshApplyProfile {
                json: true,
                bundle_dir,
                profile,
                apply,
                closed_environment,
            }),
            NetworkMeshCommand::Status {
                registry,
                resolve_dns,
                include_records,
                observe_runtime,
            } => emit_mesh_status(
                &runtime,
                registry_path(registry),
                resolve_dns,
                include_records,
                observe_runtime,
            ),
        },
        NetworkCommand::ProviderFallback { command } => match command {
            NetworkProviderFallbackCommand::Status {
                registry,
                hosts_path,
                check_connectivity,
            } => cynic_node::execute_network_command(&NetworkCommands::ProviderFallbackStatus {
                json: true,
                registry: registry_path(registry),
                hosts_path: hosts_path
                    .unwrap_or_else(|| runtime.provider_hosts_path.to_string_lossy().to_string()),
                check_connectivity,
            }),
            NetworkProviderFallbackCommand::Hosts {
                registry,
                output_dir,
            } => cynic_node::execute_network_command(&NetworkCommands::ProviderFallbackHosts {
                json: true,
                registry: registry_path(registry),
                output_dir,
            }),
        },
        NetworkCommand::Actuator { command } => match command {
            NetworkActuatorCommand::Status { registry } => {
                cynic_node::execute_network_command(&NetworkCommands::ActuatorStatus {
                    json: true,
                    registry: registry_path(registry),
                })
            }
            NetworkActuatorCommand::Bundle {
                registry,
                output_dir,
            } => cynic_node::execute_network_command(&NetworkCommands::ActuatorBundle {
                registry: registry_path(registry),
                output_dir,
            }),
            NetworkActuatorCommand::Plan {
                registry,
                bundle_dir,
            } => cynic_node::execute_network_command(&NetworkCommands::ActuatorPlan {
                registry: registry_path(registry),
                bundle_dir,
            }),
            NetworkActuatorCommand::Apply {
                registry,
                profile,
                apply,
            } => cynic_node::execute_network_command(&NetworkCommands::Apply {
                registry: registry_path(registry),
                profile,
                apply,
            }),
        },
    }
}

fn emit_mesh_status(
    runtime: &runtime_paths::RuntimePaths,
    registry: String,
    resolve_dns: bool,
    include_records: bool,
    observe_runtime: bool,
) -> ExitCode {
    let status = match cynic_node::build_network_runtime_status_value(
        &registry,
        resolve_dns,
        false,
        &runtime.provider_hosts_path.to_string_lossy(),
        &runtime.dns_hosts_path.to_string_lossy(),
        &runtime.dns_corefile_path.to_string_lossy(),
        &runtime.dns_service_path.to_string_lossy(),
        &runtime.resolved_dropin_path.to_string_lossy(),
    ) {
        Ok(value) => value,
        Err(error) => return fail(error.to_string()),
    };

    let observation = status
        .get("observation")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let runtime_observation = if observe_runtime {
        observation
            .get("runtime_observation")
            .cloned()
            .unwrap_or_else(|| json!({}))
    } else {
        json!({})
    };
    let runtime_status = observation
        .get("network_mesh_status")
        .and_then(|mesh| mesh.get("runtime_status"))
        .and_then(|value| value.as_str())
        .unwrap_or("not_observed");
    let dns_status = if resolve_dns {
        status
            .get("dns_activation")
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str())
            .map(|value| if value == "active" { "ok" } else { "violation" })
            .unwrap_or("violation")
    } else {
        observation
            .get("network_mesh_status")
            .and_then(|mesh| mesh.get("dns_status"))
            .and_then(|value| value.as_str())
            .unwrap_or("not_checked")
    };
    let topology = observation
        .get("topology")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let payload = json!({
        "schema": "cynic.infrastructure_mcp.network_mesh_status.v1",
        "zone": topology
            .get("dns_zone")
            .cloned()
            .unwrap_or_else(|| json!("unknown")),
        "registry_ref": registry,
        "canonical_service_plane": topology
            .get("canonical_service_plane")
            .cloned()
            .unwrap_or_else(|| json!("unknown")),
        "synapse_cidr": topology
            .get("synapse_cidr")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "provider_fallback_planes": [],
        "controller_status": serde_json::Value::Null,
        "policy_status": observation
            .get("network_mesh_status")
            .and_then(|mesh| mesh.get("policy_status"))
            .cloned()
            .unwrap_or(json!("ok")),
        "runtime_status": runtime_status,
        "resolve_dns": resolve_dns,
        "peers": 0,
        "dns_status": dns_status,
        "records": if include_records { json!([]) } else { json!([]) },
        "dns_checks": status.get("dns_activation")
            .and_then(|value| value.get("checks"))
            .cloned()
            .map(|checks| json!([checks]))
            .unwrap_or_else(|| json!([])),
        "runtime_observation": runtime_observation,
        "drift": runtime_observation
            .get("drift")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "blocking": runtime_observation
            .get("blocking")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "missing": status.get("dns_activation")
            .and_then(|value| value.get("missing"))
            .cloned()
            .unwrap_or_else(|| json!([])),
        "offline": [],
    });
    let should_fail = matches!(runtime_status, "blocked" | "provider_fallback_active")
        || dns_status == "violation";
    emit_json(&payload, should_fail)
}
