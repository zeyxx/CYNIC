use std::process::ExitCode;

use crate::network_surface::execution::NetworkCommands;

use crate::network_surface::{
    NetworkCommand, NetworkProviderCommand, NetworkProviderFallbackCommand,
};
use crate::registry_path;
use crate::runtime_paths::RuntimePaths;

pub(crate) fn try_execute_provider_fallback(
    command: NetworkCommand,
    runtime: &RuntimePaths,
) -> Result<ExitCode, NetworkCommand> {
    let NetworkCommand::Provider {
        command: NetworkProviderCommand::Fallback { command },
    } = command
    else {
        return Err(command);
    };

    let exit = match command {
        NetworkProviderFallbackCommand::Status {
            registry,
            hosts_path,
            check_connectivity,
        } => crate::network_surface::execution::execute_network_command(&NetworkCommands::ProviderFallbackStatus {
            json: true,
            registry: registry_path(registry),
            hosts_path: hosts_path
                .unwrap_or_else(|| runtime.provider_hosts_path.to_string_lossy().to_string()),
            check_connectivity,
        }),
        NetworkProviderFallbackCommand::Hosts {
            registry,
            output_dir,
        } => crate::network_surface::execution::execute_network_command(&NetworkCommands::ProviderFallbackHosts {
            json: true,
            registry: registry_path(registry),
            output_dir,
        }),
    };

    Ok(exit)
}
