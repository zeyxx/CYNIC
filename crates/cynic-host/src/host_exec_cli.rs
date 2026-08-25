use std::process::ExitCode;

mod args;
mod bootstrap;
mod bridge_commands;
mod build_runner_bootstrap;
mod capability_issue;
mod hardware_baseline;
mod io;
mod network_actuate;
mod network_baseline;
mod profile_selection;
mod pve_helper_bundle;
mod runtime_seed;
mod service_reconcile;
mod storage_identity;
mod storage_schema;
mod transport_run;
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
        "pve-helper-bundle" => pve_helper_bundle::execute_pve_helper_bundle(&args[1..]),
        "transport-run" => transport_run::execute_transport_run(&args[1..]),
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
            let mut inspection = crate::transport::inspect_ssh_target(&observer, &host);
            let store_path = crate::ssh_trust::default_ssh_trust_store_path();
            let store = crate::ssh_trust::load_ssh_trust_store(&store_path)?;
            let verification = crate::ssh_trust::verify_ssh_target_trust(&inspection, &store);
            crate::ssh_trust::project_trust_verification(&mut inspection, &verification);
            io::emit_json(
                &crate::ssh_trust::SshTrustInspectionReport {
                    inspection,
                    verification,
                },
                false,
            )
        }
        "ssh-trust-admit" => {
            let command_args = &args[1..];
            let host = args::flag_value(command_args, "--host")
                .ok_or_else(|| "ssh-trust-admit requires --host <hostname|ip>".to_string())?;
            let fingerprint = args::flag_value(command_args, "--fingerprint")
                .ok_or_else(|| "ssh-trust-admit requires --fingerprint <sha256>".to_string())?;
            let attestation_source = args::flag_value(command_args, "--attestation-source")
                .ok_or_else(|| {
                    "ssh-trust-admit requires --attestation-source <source>".to_string()
                })?;
            let observer =
                crate::adapters::local_operator_transport::LocalOperatorTransportObserver;
            let inspection = crate::transport::inspect_ssh_target(&observer, &host);
            let mut report = crate::ssh_trust::plan_ssh_trust_admission(
                &inspection,
                "ED25519",
                &fingerprint,
                &attestation_source,
                args::has_flag(command_args, "--yes"),
            );
            if report.status == "planned" {
                let anchor = report.anchor.clone().expect("planned admission has anchor");
                let store_path = crate::ssh_trust::default_ssh_trust_store_path();
                let added = crate::ssh_trust::admit_ssh_trust_anchor(&store_path, anchor)?;
                let store = crate::ssh_trust::load_ssh_trust_store(&store_path)?;
                report.status = if added {
                    "admitted"
                } else {
                    "already_admitted"
                }
                .to_string();
                report.mutates = added;
                report.verification =
                    crate::ssh_trust::verify_ssh_target_trust(&inspection, &store);
            }
            io::emit_json(&report, report.status == "blocked")
        }
        "bootstrap-status" => bootstrap::execute_bootstrap_status(&args[1..]),
        "bootstrap-plan" => bootstrap::execute_bootstrap_plan(&args[1..]),
        "bootstrap-chain" => bootstrap::execute_bootstrap_chain(&args[1..]),
        "bootstrap-evidence" => bootstrap::execute_bootstrap_evidence(&args[1..]),
        "build-runner-bootstrap" => build_runner_bootstrap::execute(&args[1..]),
        "capability-issue" => capability_issue::execute(&args[1..]),
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
