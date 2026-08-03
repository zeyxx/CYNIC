use std::process::ExitCode;

use cynic_node::NetworkCommands;

use crate::network_surface::{NetworkCommand, NetworkDnsCommand};
use crate::registry_path;
use crate::runtime_paths::RuntimePaths;

pub(crate) fn try_execute_dns(
    command: NetworkCommand,
    runtime: &RuntimePaths,
) -> Result<ExitCode, NetworkCommand> {
    let NetworkCommand::Dns { command } = command else {
        return Err(command);
    };

    let exit = match command {
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
        NetworkDnsCommand::Plan { .. } => {
            cynic_node::execute_network_command(&NetworkCommands::DnsPlan { json: true })
        }
        NetworkDnsCommand::Activate {
            registry,
            bundle_dir,
            yes,
        } => cynic_node::execute_network_command(&NetworkCommands::DnsActivate {
            json: true,
            registry: registry_path(registry),
            bundle_dir,
            yes,
        }),
    };

    Ok(exit)
}
