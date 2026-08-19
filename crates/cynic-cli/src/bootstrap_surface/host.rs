use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::adapters::local_command_runner::LocalHostCommandRunner;
use cynic_host::adapters::local_hardware_baseline_bundle::LocalHardwareBaselineBundleStore;
use cynic_host::adapters::local_observation::LocalHostObservationAdapter;
use cynic_host::bootstrap_host_plan::{
    build_host_bootstrap_composite_plan, combine_host_bootstrap_apply,
};
use cynic_host::domain::shared::SurfaceStatus;
use cynic_host::host_bootstrap_evidence;
use cynic_host::ports::{HostPrincipalPort, HostServicePort};
use cynic_host::registry::{
    resolve_current_registry_service_policy, resolve_registry_service_policy,
};
use cynic_host::runtime_layout::current_runtime_layout;
use cynic_host::runtime_proof;
use cynic_host::service_supervisor_observation::{self, SupervisorProvider};
use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_host::usecases::{
    build_host_apply_report_with_resolution, build_host_bundle_with_resolution,
    execute_admitted_service_apply, execute_host_apply, observe_host_with_resolution,
    service_apply_satisfied_preconditions,
};
use cynic_protocol::{ExecutionAuthorization, PolicyAuthority};
// surface_audit_marker: build_host_activation_plan_with_resolution

use crate::bootstrap_surface::host_agent_access::{
    AgentAccessArgs, AgentAuthorityArgs, AgentAuthorityBundleArgs, execute_agent_access,
    execute_agent_authority, execute_agent_authority_bundle,
};
use crate::bootstrap_surface::host_e2e::build_host_bootstrap_e2e_report;
use crate::bootstrap_surface::selection::{ProfileArgs, observed_blocked, selected_host_context};
use crate::{emit_json, fail, registry_path};

#[derive(Subcommand)]
pub(crate) enum BootstrapHostCommand {
    Observe(ProfileArgs),
    Bundle(ProfileArgs),
    Plan(ProfileArgs),
    Evidence(ProfileArgs),
    Proof(ProfileArgs),
    Supervisor {
        #[command(flatten)]
        profile: ProfileArgs,
        #[arg(long, default_value = "auto")]
        provider: String,
    },
    Principal {
        #[command(flatten)]
        profile: ProfileArgs,
        #[arg(long)]
        apply: bool,
    },
    AgentAccess(AgentAccessArgs),
    AgentAuthorityBundle(AgentAuthorityBundleArgs),
    AgentAuthority(AgentAuthorityArgs),
    #[command(name = "e2e")]
    E2e {
        #[command(flatten)]
        profile: ProfileArgs,
        #[arg(long, default_value = "single-node")]
        runtime_profile: String,
        #[arg(long)]
        dry_run: bool,
    },
    ApplyReport(ProfileArgs),
    Apply {
        #[command(flatten)]
        profile: ProfileArgs,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Observe host OS-substrate health (soft lockups, THP defrag, reclaim/OOM
    /// pressure) — the layer CYNIC was blind to during the 2026-08-18 crash.
    #[command(name = "os-health")]
    OsHealth(ProfileArgs),
}

pub(crate) fn execute_bootstrap_host(command: BootstrapHostCommand) -> ExitCode {
    match command {
        BootstrapHostCommand::Observe(args) => {
            let (selected, resolution) = match selected_host_context(&args) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            let observed =
                observe_host_with_resolution(&selected, &LocalHostObservationAdapter, resolution);
            emit_json(&observed, observed_blocked(&observed))
        }
        BootstrapHostCommand::Bundle(args) => {
            let (selected, resolution) = match selected_host_context(&args) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            emit_json(
                &build_host_bundle_with_resolution(&selected, resolution),
                false,
            )
        }
        BootstrapHostCommand::Plan(args) => {
            let (selected, resolution) = match selected_host_context(&args) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            let observed = observe_host_with_resolution(
                &selected,
                &LocalHostObservationAdapter,
                resolution.clone(),
            );
            let plan = cynic_host::bootstrap_host_plan::build_host_bootstrap_composite_plan(
                &selected,
                &observed,
                resolution.as_ref(),
                &LocalHostObservationAdapter,
            );
            emit_json(&plan, false)
        }
        BootstrapHostCommand::Evidence(args) => {
            let (selected, resolution) = match selected_host_context(&args) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            match host_bootstrap_evidence::observe_host_bootstrap_evidence(selected, resolution) {
                Ok(report) => emit_json(&report, report.status != SurfaceStatus::Active),
                Err(error) => fail(error.to_string()),
            }
        }
        BootstrapHostCommand::Proof(args) => {
            let (selected, resolution) = match selected_host_context(&args) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            let observed =
                observe_host_with_resolution(&selected, &LocalHostObservationAdapter, resolution);
            let proof = runtime_proof::host_identity_runtime_proof_bundle(&observed);
            emit_json(&proof, proof.status != "satisfied")
        }
        BootstrapHostCommand::Supervisor { profile, provider } => {
            let provider = match SupervisorProvider::parse(&provider) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            let (selected, resolution) = match selected_host_context(&profile) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            let subject = resolution
                .as_ref()
                .and_then(|resolution| resolution.matched_node.clone())
                .or_else(|| {
                    resolution
                        .as_ref()
                        .filter(|resolution| resolution.hostname_observed != "unknown")
                        .map(|resolution| resolution.hostname_observed.clone())
                })
                .unwrap_or_else(|| selected.name().to_string());
            let registry = registry_path(profile.registry.clone());
            let service_policy = if profile.profile.is_none() {
                match profile
                    .node
                    .as_ref()
                    .map(|node| resolve_registry_service_policy(node, Some(registry.as_str())))
                    .unwrap_or_else(|| {
                        resolve_current_registry_service_policy(Some(registry.as_str()))
                    }) {
                    Ok(Some(policy)) => policy,
                    Ok(None) | Err(_) => selected.desired_services(),
                }
            } else {
                selected.desired_services()
            };
            let report = service_supervisor_observation::observe_service_supervisor_with_provider(
                subject,
                &service_policy,
                provider,
            );
            emit_json(&report, false)
        }
        BootstrapHostCommand::Principal { profile, apply } => {
            let (selected, resolution) = match selected_host_context(&profile) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            let policy = selected.desired_principal();
            if apply {
                let authorization = ExecutionAuthorization::new(
                    "local-cli-admin",
                    PolicyAuthority::HumanAdmin,
                    true,
                    cynic_host::principal_prepare::principal_prepare_preconditions(),
                );
                let report = cynic_host::principal_prepare::principal_prepare_apply(
                    &policy,
                    resolution.as_ref(),
                    authorization,
                    &LocalHostCommandRunner,
                    &LocalHostObservationAdapter,
                );
                emit_json(&report, report.status != "completed")
            } else {
                let report = cynic_host::principal_prepare::principal_prepare_plan_with_resolution(
                    &policy,
                    resolution.as_ref(),
                );
                emit_json(&report, false)
            }
        }
        BootstrapHostCommand::AgentAccess(args) => execute_agent_access(args),
        BootstrapHostCommand::AgentAuthorityBundle(args) => execute_agent_authority_bundle(args),
        BootstrapHostCommand::AgentAuthority(args) => execute_agent_authority(args),
        BootstrapHostCommand::E2e {
            profile,
            runtime_profile,
            dry_run,
        } => {
            if !dry_run {
                return fail("host e2e currently requires --dry-run");
            }
            match build_host_bootstrap_e2e_report(&profile, &runtime_profile) {
                Ok(report) => {
                    let should_fail = report.status != "ok";
                    emit_json(&report, should_fail)
                }
                Err(error) => fail(error),
            }
        }
        BootstrapHostCommand::ApplyReport(args) => {
            let (selected, resolution) = match selected_host_context(&args) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            let observed = observe_host_with_resolution(
                &selected,
                &LocalHostObservationAdapter,
                resolution.clone(),
            );
            let report =
                build_host_apply_report_with_resolution(&selected, &observed, true, resolution);
            emit_json(&report, false)
        }
        BootstrapHostCommand::Apply {
            profile,
            dry_run,
            apply,
        } => {
            if dry_run == apply {
                return fail("apply requires exactly one of --dry-run or --apply");
            }
            let (selected, resolution) = match selected_host_context(&profile) {
                Ok(value) => value,
                Err(error) => return fail(error),
            };
            let observed = observe_host_with_resolution(
                &selected,
                &LocalHostObservationAdapter,
                resolution.clone(),
            );
            let composite_plan = build_host_bootstrap_composite_plan(
                &selected,
                &observed,
                resolution.as_ref(),
                &LocalHostObservationAdapter,
            );
            let substrate = if dry_run {
                execute_host_apply(&selected, &observed, true, &LocalHostCommandRunner)
            } else {
                let authorization = ExecutionAuthorization::new(
                    "local-cli-operator",
                    PolicyAuthority::HumanOperator,
                    true,
                    service_apply_satisfied_preconditions(&observed),
                );
                execute_admitted_service_apply(
                    &selected,
                    &observed,
                    authorization,
                    &LocalHostCommandRunner,
                    &LocalHostObservationAdapter,
                )
            };
            let hardware = cynic_host::hardware_baseline_reconcile::reconcile_hardware_baseline(
                &composite_plan.hardware_baseline,
                apply,
                &current_runtime_layout(),
                &LocalHardwareBaselineBundleStore,
                &LocalArgvRunner,
            );
            let report = combine_host_bootstrap_apply(substrate, hardware);
            let should_fail = !dry_run && report.status != "completed";
            emit_json(&report, should_fail)
        }
        BootstrapHostCommand::OsHealth(args) => {
            let observer = cynic_host::os_health::observe::LocalOsHealthObserver;
            let assessment = cynic_host::os_health::assess_os_health_default(&observer);
            if args.json {
                emit_json(&assessment, assessment.status != "healthy")
            } else {
                println!("{}", cynic_host::os_health::os_health_human(&assessment));
                if assessment.status != "healthy" {
                    return ExitCode::FAILURE;
                }
                ExitCode::SUCCESS
            }
        }
    }
}
