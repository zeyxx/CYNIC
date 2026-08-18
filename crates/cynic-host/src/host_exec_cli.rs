use std::process::ExitCode;

mod args;
mod bootstrap;
mod bridge_commands;
mod build_runner_bootstrap;
mod hardware_baseline;
mod io;
mod network_actuate;
mod network_baseline;
mod profile_selection;
mod runtime_seed;
mod service_reconcile;
mod storage_identity;
mod storage_schema;
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
            let observer =
                crate::adapters::local_operator_transport::LocalOperatorTransportObserver;
            io::emit_json(
                &crate::transport::build_transport_matrix_report(&observer),
                false,
            )
        }
        "ssh-inspect" => {
            let host = args::flag_value(&args[1..], "--host")
                .ok_or_else(|| "ssh-inspect requires --host <hostname|ip>".to_string())?;
            let observer =
                crate::adapters::local_operator_transport::LocalOperatorTransportObserver;
            io::emit_json(
                &crate::transport::inspect_ssh_target(&observer, &host),
                false,
            )
        }
        "bootstrap-status" => bootstrap::execute_bootstrap_status(&args[1..]),
        "bootstrap-plan" => bootstrap::execute_bootstrap_plan(&args[1..]),
        "bootstrap-chain" => bootstrap::execute_bootstrap_chain(&args[1..]),
        "bootstrap-evidence" => bootstrap::execute_bootstrap_evidence(&args[1..]),
        "build-runner-bootstrap" => build_runner_bootstrap::execute(&args[1..]),
        "principal-prepare" => bootstrap::execute_principal_prepare(&args[1..]),
        "runtime-seed-plan" => runtime_seed::execute_plan(&args[1..]),
        "runtime-seed-reconcile" => runtime_seed::execute_reconcile(&args[1..]),
        "storage-identity-enroll" => storage_identity::execute(&args[1..]),
        "storage-schema-migrate" => storage_schema::execute(&args[1..]),
        "hardware-baseline-enroll" => hardware_baseline::execute(&args[1..]),
        "service-reconcile" => service_reconcile::execute(&args[1..]),
        "network-baseline" => network_baseline::execute_network_baseline(&args[1..]),
        "network-actuate" => network_actuate::execute(&args[1..]),
        _ => Err(format!("unknown profile: {profile}\n{}", args::usage())),
    }
}
