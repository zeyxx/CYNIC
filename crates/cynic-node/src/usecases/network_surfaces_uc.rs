use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use crate::adapters::libvirt_harness_target_attestor::LibvirtHarnessTargetAttestor;
use crate::adapters::local_harness_target_attestation_reader::LocalHarnessTargetAttestationReader;
use crate::adapters::local_network_actuation_request_reader::read_network_actuation_request;
use crate::adapters::qemu_guest_agent_network_runner::QemuGuestAgentNetworkRunner;
use crate::cli::NetworkCommands;
use crate::dns_activation_status;
use crate::dns_bundle;
use crate::dns_plan;
use crate::domain::network_execution::build_harness_target_attestation_report;
use crate::network_activation_chain;
use crate::network_actuator;
use crate::network_actuator_status;
use crate::network_evidence;
use crate::network_mesh_apply_plan;
use crate::network_mesh_bundle;
use crate::network_mesh_plan;
use crate::network_repair;
use crate::network_root_policy;
use crate::network_runtime_status;
use crate::network_shadow_proof;
use crate::observe;
use crate::ports::harness_target_attestation_port::{
    HarnessTargetAttestationPort, HarnessTargetAttestationRequest,
};
use crate::ports::harness_target_attestation_reader_port::HarnessTargetAttestationReaderPort;
use crate::provider_fallback_hosts;
use crate::provider_fallback_status;
use crate::usecases::network_actuation_target_uc::{
    QemuNetworkActuationConfig, execute_qemu_network_actuation,
};
use cynic_host::runtime_harness::runtime_harness_catalog;
use cynic_host::update_lifecycle::LocalArgvRunner;

pub fn execute_observe(registry: &str, fail_on_warning: bool, fail_on_critical: bool) -> ExitCode {
    let observation = observe::observe_payload(Some(registry));
    let should_fail = (fail_on_warning && observation.is_warning_or_worse())
        || (fail_on_critical && observation.is_critical());
    emit_json("observe", &observation, should_fail)
}

pub fn execute_registry_observation(registry: &str, output: Option<&str>) -> ExitCode {
    let observation = observe::observe_payload(Some(registry));
    let envelope = observation.registry_observation(registry);
    if let Some(output) = output {
        let bytes = match serde_json::to_vec_pretty(&envelope) {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::error!("registry observation serialization failed: {error}");
                return ExitCode::FAILURE;
            }
        };
        if let Err(error) = std::fs::write(output, bytes) {
            tracing::error!("registry observation write failed: {error}");
            return ExitCode::FAILURE;
        }
    }
    emit_json("registry observation", &envelope, false)
}

pub fn execute_network_command(command: &NetworkCommands) -> ExitCode {
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
        NetworkCommands::DnsActivate {
            json: _,
            registry,
            bundle_dir,
            yes,
        } => match crate::apply_dns_activation_value(registry, bundle_dir, *yes) {
            Ok(report) => {
                let should_fail = report["status"].as_str() != Some("proved")
                    && report["status"].as_str() != Some("converged");
                emit_json("dns activation apply", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("dns activation apply failed: {error}");
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
        NetworkCommands::ActuationObservation { json: _, registry } => {
            let observation = observe::observe_payload(Some(registry));
            let projection = observation.network_actuation_observation(
                cynic_host::runtime_layout::current_runtime_layout()
                    .capabilities()
                    .wireguard_tool
                    .mesh_interface,
            );
            emit_json("network actuation observation", &projection, false)
        }
        NetworkCommands::ActuatorStatus { json: _, registry } => {
            let observation = observe::observe_payload(Some(registry));
            let status = network_actuator_status::activation_status(&observation);
            emit_json("network actuator status", &status, false)
        }
        NetworkCommands::ActuatorBundle {
            registry,
            output_dir,
        } => {
            let observation = observe::observe_payload(Some(registry));
            match network_actuator::write_install_bundle(&observation, output_dir) {
                Ok(bundle) => emit_json("network actuator bundle", &bundle, false),
                Err(error) => {
                    tracing::error!("network actuator bundle failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        NetworkCommands::ActuatorPlan {
            registry,
            bundle_dir,
        } => {
            let observation = observe::observe_payload(Some(registry));
            let plan = network_actuator::activation_plan(&observation, bundle_dir);
            emit_json("network actuator plan", &plan, false)
        }
        NetworkCommands::Apply {
            registry,
            profile,
            apply,
        } => {
            let observation = observe::observe_payload(Some(registry));
            let report = network_actuator::apply_profile(&observation, profile, *apply);
            let should_fail = report.status != "completed";
            emit_json("network actuator apply", &report, should_fail)
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
        NetworkCommands::MeshPlan {
            json: _,
            registry,
            provider,
        } => match network_mesh_plan::build_network_mesh_plan(registry, provider.as_deref()) {
            Ok(report) => emit_json("network mesh plan", &report, false),
            Err(error) => {
                tracing::error!("network mesh plan failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::MeshBundle {
            json: _,
            registry,
            provider,
            output_dir,
        } => match network_mesh_bundle::build_network_mesh_bundle(
            registry,
            provider.as_deref(),
            output_dir,
        ) {
            Ok(report) => emit_json("network mesh bundle", &report, false),
            Err(error) => {
                tracing::error!("network mesh bundle failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::MeshApplyPlan {
            json: _,
            bundle_dir,
        } => match network_mesh_apply_plan::build_network_mesh_apply_plan(bundle_dir) {
            Ok(report) => emit_json("network mesh apply plan", &report, false),
            Err(error) => {
                tracing::error!("network mesh apply plan failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::MeshTargetAttestation {
            json: _,
            harness_kind,
            target_id,
            transport,
            transport_target,
            expected_network_boundary,
            virsh_bin,
            ip_bin,
        } => {
            let context = match runtime_harness_catalog(Some(harness_kind)) {
                Ok(mut catalog) => match catalog.contexts.pop() {
                    Some(context) => context,
                    None => {
                        tracing::error!("runtime harness catalog returned no selected context");
                        return ExitCode::FAILURE;
                    }
                },
                Err(error) => {
                    tracing::error!("runtime harness selection failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let attestor = LibvirtHarnessTargetAttestor::new(
                LocalArgvRunner,
                virsh_bin.clone(),
                ip_bin.clone(),
            );
            match attestor.attest_target(
                &context,
                &HarnessTargetAttestationRequest {
                    target_id: target_id.clone(),
                    provider: "qemu-libvirt".to_string(),
                    transport: transport.clone(),
                    transport_target: transport_target.clone(),
                    expected_network_boundary: expected_network_boundary.clone(),
                },
            ) {
                Ok(attestation) => {
                    let report = build_harness_target_attestation_report(&context, attestation);
                    let should_fail = report.status != "attested";
                    emit_json("network mesh target attestation", &report, should_fail)
                }
                Err(error) => {
                    tracing::error!("network mesh target attestation failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        NetworkCommands::MeshApplyTarget {
            json: _,
            bundle_dir,
            profile,
            apply,
            harness_kind,
            target_attestation,
            virsh_bin,
            qga_timeout_seconds,
            qga_max_status_polls,
        } => {
            let context = match selected_harness_context(harness_kind) {
                Ok(context) => context,
                Err(error) => {
                    tracing::error!("runtime harness selection failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let attestation = match LocalHarnessTargetAttestationReader
                .read_attestation(Path::new(target_attestation))
            {
                Ok(attestation) => attestation,
                Err(error) => {
                    tracing::error!("network mesh target attestation load failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let plan = match network_mesh_apply_plan::build_network_mesh_apply_plan(bundle_dir) {
                Ok(plan) => plan,
                Err(error) => {
                    tracing::error!("network mesh apply plan failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let allowed_argv = network_mesh_apply_plan::target_transaction_argv(&plan, profile);
            let runner = match QemuGuestAgentNetworkRunner::new(
                LocalArgvRunner,
                context.clone(),
                attestation,
                virsh_bin.clone(),
                allowed_argv,
                *qga_timeout_seconds,
                *qga_max_status_polls,
                Duration::from_millis(50),
            ) {
                Ok(runner) => runner,
                Err(error) => {
                    tracing::error!("network mesh target transport refused: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let report = network_mesh_apply_plan::apply_network_mesh_target_transaction_with_runner(
                bundle_dir, profile, *apply, &context, &runner, plan,
            );
            let should_fail = report.status != "completed";
            emit_json("network mesh apply target", &report, should_fail)
        }
        NetworkCommands::ActuateTarget {
            json: _,
            request,
            apply,
            harness_kind,
            target_attestation,
            observer_bin,
            virsh_bin,
            qga_timeout_seconds,
            qga_max_status_polls,
        } => {
            if !apply {
                tracing::error!("network actuation target requires --apply");
                return ExitCode::FAILURE;
            }
            let context = match selected_harness_context(harness_kind) {
                Ok(context) => context,
                Err(error) => {
                    tracing::error!("runtime harness selection failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let attestation = match LocalHarnessTargetAttestationReader
                .read_attestation(Path::new(target_attestation))
            {
                Ok(attestation) => attestation,
                Err(error) => {
                    tracing::error!("network actuation target attestation load failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let request = match read_network_actuation_request(Path::new(request)) {
                Ok(request) => request,
                Err(error) => {
                    tracing::error!("network actuation request load failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let config = QemuNetworkActuationConfig {
                virsh_bin: virsh_bin.clone(),
                observation_argv: vec![
                    observer_bin.clone(),
                    "network".to_string(),
                    "actuation-observation".to_string(),
                    "--json".to_string(),
                ],
                timeout_seconds: *qga_timeout_seconds,
                max_status_polls: *qga_max_status_polls,
                poll_interval: Duration::from_millis(50),
            };
            match execute_qemu_network_actuation(
                &request,
                context,
                attestation,
                LocalArgvRunner,
                config,
            ) {
                Ok(report) => {
                    let should_fail = report.status != "proved";
                    emit_json("network actuation target", &report, should_fail)
                }
                Err(error) => {
                    tracing::error!("network actuation target failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        NetworkCommands::MeshApplyProfile {
            json: _,
            bundle_dir,
            profile,
            apply,
            closed_environment,
        } => match network_mesh_apply_plan::apply_network_mesh_profile(
            bundle_dir,
            profile,
            *apply,
            *closed_environment,
        ) {
            Ok(report) => {
                let should_fail = report.status != "completed";
                emit_json("network mesh apply profile", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("network mesh apply profile failed: {error}");
                ExitCode::FAILURE
            }
        },
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
        NetworkCommands::RuntimeProof {
            json: _,
            registry,
            resolve_dns,
            check_provider_connectivity,
            provider_hosts_path,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
        } => match crate::network_runtime_proof::build_network_runtime_proof_bundle(
            registry,
            *resolve_dns,
            *check_provider_connectivity,
            provider_hosts_path,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
        ) {
            Ok(report) => emit_json(
                "network runtime proof",
                &report,
                report.status != "satisfied",
            ),
            Err(error) => {
                tracing::error!("network runtime proof failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::PeerRuntimeProof { peer, json: _ } => {
            let fetcher =
                crate::adapters::runtime_proof_http_fetcher::HttpRuntimeProofFetcher::new();
            let admission = crate::network_runtime_proof::verify_peer_runtime_publisher_admission(
                &fetcher, &peer,
            );
            let report = serde_json::json!({
                "peer": peer,
                "runtime_publisher_admission": admission,
                "federation_trust": admission,
            });
            emit_json("peer runtime proof", &report, !admission)
        }
        NetworkCommands::ActivationChain {
            json: _,
            registry,
            resolve_dns,
            check_provider_connectivity,
            provider_hosts_path,
            hosts_path,
            corefile_path,
            service_path,
            resolved_dropin_path,
        } => match network_activation_chain::build_activation_chain_report(
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
                let should_fail = report.status == "blocked";
                emit_json("network activation chain", &report, should_fail)
            }
            Err(error) => {
                tracing::error!("network activation chain failed: {error}");
                ExitCode::FAILURE
            }
        },
        NetworkCommands::RecoveryTick {
            registry,
            state_path,
            cooldown_seconds,
            max_attempts,
            apply,
            json: _,
        } => {
            let runner = crate::adapters::local_network_command_runner::LocalNetworkCommandRunner;
            let report =
                crate::usecases::node_network_recovery_uc::reconcile_node_network_recovery_tick(
                    *apply,
                    registry,
                    std::path::Path::new(state_path),
                    std::time::Duration::from_secs(*cooldown_seconds),
                    *max_attempts,
                    &runner,
                    |reg| crate::observe::observe_payload(reg),
                );
            let should_fail = report.status == "blocked";
            emit_json("node network recovery tick", &report, should_fail)
        }
    }
}

fn selected_harness_context(
    harness_kind: &str,
) -> Result<cynic_host::runtime_harness::RuntimeHarnessContext, String> {
    runtime_harness_catalog(Some(harness_kind))?
        .contexts
        .pop()
        .ok_or_else(|| "runtime harness catalog returned no selected context".to_string())
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
