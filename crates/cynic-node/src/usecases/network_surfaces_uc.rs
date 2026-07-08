use std::process::ExitCode;

use crate::cli::NetworkCommands;
use crate::dns_activation_status;
use crate::dns_bundle;
use crate::dns_plan;
use crate::network_actuator_status;
use crate::network_evidence;
use crate::network_repair;
use crate::network_root_policy;
use crate::network_runtime_status;
use crate::network_shadow_proof;
use crate::observe;
use crate::provider_fallback_hosts;
use crate::provider_fallback_status;

pub(crate) fn execute_observe(
    registry: &str,
    fail_on_warning: bool,
    fail_on_critical: bool,
) -> ExitCode {
    let observation = observe::observe_payload(Some(registry));
    let should_fail = (fail_on_warning && observation.is_warning_or_worse())
        || (fail_on_critical && observation.is_critical());
    emit_json("observe", &observation, should_fail)
}

pub(crate) fn execute_network_command(command: &NetworkCommands) -> ExitCode {
    match command {
        NetworkCommands::DnsBundle {
            json: _,
            registry,
            output_dir,
        } => match dns_bundle::build_dns_bundle(registry, output_dir) {
            Ok(report) => emit_json("dns bundle", &report, false),
            Err(error) => {
                tracing::error!("dns bundle failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::DnsPlan { json: _ } => {
            let report = dns_plan::build_dns_plan();
            emit_json("dns plan", &report, false)
        }
        NetworkCommands::DnsActivationStatus {
            json: _,
            registry,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
            resolve_dns,
        } => match dns_activation_status::build_dns_activation_status(
            registry,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
            *resolve_dns,
        ) {
            Ok(report) => {
                let should_fail = report.status != "active";
                emit_json("dns activation status", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("dns activation status failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::Evidence { json: _, registry } => {
            let observation = observe::observe_payload(Some(registry));
            let evidence = network_evidence::build_evidence(&observation);
            emit_json("network evidence", &evidence, false)
        }
        NetworkCommands::Policy { json: _, registry } => {
            let observation = observe::observe_payload(Some(registry));
            let policy = network_root_policy::build_root_policy(&observation);
            emit_json("network root policy", &policy, false)
        }
        NetworkCommands::ActuatorStatus { json: _, registry } => {
            let observation = observe::observe_payload(Some(registry));
            let status = network_actuator_status::activation_status(&observation);
            emit_json("network actuator status", &status, false)
        }
        NetworkCommands::Repair { dry_run, registry } => {
            if !dry_run {
                tracing::error!(
                    "network repair currently requires --dry-run; apply is not implemented"
                );
                return ExitCode::FAILURE;
            }
            let observation = observe::observe_payload(Some(registry));
            let plan = network_repair::build_dry_run_plan(&observation);
            emit_json("network repair plan", &plan, false)
        }
        NetworkCommands::ProviderFallbackStatus {
            json: _,
            registry,
            hosts_path,
            check_connectivity,
        } => match provider_fallback_status::build_provider_fallback_status(
            registry,
            hosts_path,
            *check_connectivity,
        ) {
            Ok(report) => {
                let should_fail = report.status != "active";
                emit_json("provider fallback status", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("provider fallback status failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::ProviderFallbackHosts {
            json: _,
            registry,
            output_dir,
        } => match provider_fallback_hosts::build_provider_fallback_hosts(registry, output_dir) {
            Ok(report) => {
                let should_fail = report.status == "empty";
                emit_json("provider fallback hosts", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("provider fallback hosts failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::ShadowProof {
            json: _,
            registry,
            resolve_dns,
            observe_runtime,
        } => {
            match network_shadow_proof::build_shadow_proof(registry, *resolve_dns, *observe_runtime)
            {
                Ok(report) => {
                    let should_fail = report.status == "drift_detected";
                    emit_json("network shadow proof", &report, should_fail)
                }
                Err(error) => {
                    tracing::error!("network shadow proof failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        NetworkCommands::RuntimeStatus {
            json: _,
            registry,
            resolve_dns,
            check_provider_connectivity,
            provider_hosts_path,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
        } => match network_runtime_status::build_network_runtime_status(
            registry,
            *resolve_dns,
            *check_provider_connectivity,
            provider_hosts_path,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
        ) {
            Ok(report) => {
                let should_fail = should_fail_runtime_status(&report.status);
                emit_json("network runtime status", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("network runtime status failed: {error}");
                ExitCode::FAILURE
            }
        },
    }
}

fn should_fail_runtime_status(status: &str) -> bool {
    status == "blocked"
}

fn emit_json<T: serde::Serialize>(label: &str, value: &T, should_fail: bool) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(payload) => {
            println!("{payload}");
            if should_fail {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            tracing::error!("{label} serialization failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::should_fail_runtime_status;

    #[test]
    fn runtime_status_only_fails_when_blocked() {
        assert!(should_fail_runtime_status("blocked"));
        assert!(!should_fail_runtime_status("provider_fallback_active"));
        assert!(!should_fail_runtime_status("mesh_active"));
    }
}
