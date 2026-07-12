use std::process::ExitCode;

mod args;
mod bootstrap;
mod bridge_commands;
mod io;
mod network_baseline;
mod profile_selection;
mod validation;

pub fn execute(args: Vec<String>) -> Result<ExitCode, String> {
    let Some(profile) = args.first().map(String::as_str) else {
        return Err(args::usage());
    };

    match profile {
        "bridge-bundle" => bridge_commands::execute_bridge_bundle(&args[1..]),
        "bridge-plan" => bridge_commands::execute_bridge_plan(&args[1..]),
        "bridge-install" => bridge_commands::execute_bridge_install(&args[1..]),
        "bridge-status" => bridge_commands::execute_bridge_status(),
        "host-exec-validate-request" => validation::execute_host_exec_validate_request(&args[1..]),
        "transport-matrix" => {
            io::emit_json(&crate::transport::build_transport_matrix_report(), false)
        }
        "bootstrap-status" => bootstrap::execute_bootstrap_status(&args[1..]),
        "bootstrap-plan" => bootstrap::execute_bootstrap_plan(&args[1..]),
        "bootstrap-chain" => bootstrap::execute_bootstrap_chain(&args[1..]),
        "bootstrap-evidence" => bootstrap::execute_bootstrap_evidence(&args[1..]),
        "principal-prepare" => bootstrap::execute_principal_prepare(&args[1..]),
        "network-baseline" => network_baseline::execute_network_baseline(&args[1..]),
        _ => Err(format!("unknown profile: {profile}\n{}", args::usage())),
    }
}
