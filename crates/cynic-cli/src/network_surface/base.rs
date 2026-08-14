use std::process::ExitCode;

use cynic_node::NetworkCommands;

use crate::network_surface::NetworkCommand;
use crate::registry_path;
use crate::runtime_paths::RuntimePaths;

pub(crate) fn try_execute_base(
    command: NetworkCommand,
    runtime: &RuntimePaths,
) -> Result<ExitCode, NetworkCommand> {
    match command {
        NetworkCommand::Chain {
            registry,
            resolve_dns,
            check_provider_connectivity,
        } => Ok(cynic_node::execute_network_command(
            &NetworkCommands::ActivationChain {
                json: true,
                registry: registry_path(registry),
                resolve_dns,
                check_provider_connectivity,
                provider_hosts_path: runtime.provider_hosts_path.to_string_lossy().to_string(),
                hosts_path: runtime.dns_hosts_path.to_string_lossy().to_string(),
                corefile_path: runtime.dns_corefile_path.to_string_lossy().to_string(),
                service_path: runtime.dns_service_path.to_string_lossy().to_string(),
                resolved_dropin_path: runtime.resolved_dropin_path.to_string_lossy().to_string(),
            },
        )),
        NetworkCommand::Runtime {
            registry,
            resolve_dns,
            check_provider_connectivity,
        } => Ok(cynic_node::execute_network_command(
            &NetworkCommands::RuntimeStatus {
                json: true,
                registry: registry_path(registry),
                resolve_dns,
                check_provider_connectivity,
                provider_hosts_path: runtime.provider_hosts_path.to_string_lossy().to_string(),
                hosts_path: runtime.dns_hosts_path.to_string_lossy().to_string(),
                corefile_path: runtime.dns_corefile_path.to_string_lossy().to_string(),
                service_path: runtime.dns_service_path.to_string_lossy().to_string(),
                resolved_dropin_path: runtime.resolved_dropin_path.to_string_lossy().to_string(),
            },
        )),
        NetworkCommand::Proof {
            registry,
            resolve_dns,
            check_provider_connectivity,
        } => Ok(cynic_node::execute_network_command(
            &NetworkCommands::RuntimeProof {
                json: true,
                registry: registry_path(registry),
                resolve_dns,
                check_provider_connectivity,
                provider_hosts_path: runtime.provider_hosts_path.to_string_lossy().to_string(),
                hosts_path: runtime.dns_hosts_path.to_string_lossy().to_string(),
                corefile_path: runtime.dns_corefile_path.to_string_lossy().to_string(),
                service_path: runtime.dns_service_path.to_string_lossy().to_string(),
                resolved_dropin_path: runtime.resolved_dropin_path.to_string_lossy().to_string(),
            },
        )),
        NetworkCommand::Evidence { registry } => Ok(cynic_node::execute_network_command(
            &NetworkCommands::Evidence {
                json: true,
                registry: registry_path(registry),
            },
        )),
        other => Err(other),
    }
}
