use cynic_host::adapters::local_command_runner::LocalHostCommandRunner;
use cynic_host::adapters::local_observation::LocalHostObservationAdapter;
use cynic_host::domain::shared::SurfaceStatus;
use cynic_host::runtime_proof;
use cynic_host::usecases::{
    HostActivationPlanReport, HostApplyExecutionReport, HostApplyReport, HostObservationReports,
    build_host_activation_plan_with_resolution, build_host_apply_report_with_resolution,
    execute_host_apply, observe_host_with_resolution,
};
use cynic_protocol::{RuntimeExecutionPlan, RuntimeProofBundle, runtime_initial_observed_gates};
use serde::Serialize;

use crate::bootstrap_surface::selection::{ProfileArgs, selected_host_context};
use crate::{registry_path, runtime_suite};

pub(crate) const HOST_BOOTSTRAP_E2E_SCHEMA: &str = "cynic.host_bootstrap.e2e.v1";

#[derive(Debug, Serialize)]
pub(crate) struct HostBootstrapE2eReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) mode: &'static str,
    pub(crate) registry_path: String,
    pub(crate) runtime_profile: String,
    pub(crate) rule: &'static str,
    pub(crate) observed_gates: Vec<String>,
    pub(crate) blocked_gates: Vec<String>,
    pub(crate) next_actions: Vec<String>,
    pub(crate) observation: HostObservationReports,
    pub(crate) activation_plan: HostActivationPlanReport,
    pub(crate) apply_report: HostApplyReport,
    pub(crate) apply_execution: HostApplyExecutionReport,
    pub(crate) proof_bundle: RuntimeProofBundle,
    pub(crate) runtime_plan: RuntimeExecutionPlan,
}

pub(crate) fn build_host_bootstrap_e2e_report(
    profile: &ProfileArgs,
    runtime_profile: &str,
) -> Result<HostBootstrapE2eReport, String> {
    let registry_path = registry_path(profile.registry.clone());
    let (selected, resolution) = selected_host_context(profile)?;
    let observation =
        observe_host_with_resolution(&selected, &LocalHostObservationAdapter, resolution.clone());
    let activation_plan =
        build_host_activation_plan_with_resolution(&selected, &observation, resolution.clone());
    let apply_report =
        build_host_apply_report_with_resolution(&selected, &observation, true, resolution);
    let apply_execution =
        execute_host_apply(&selected, &observation, true, &LocalHostCommandRunner);
    let proof_bundle = runtime_proof::host_identity_runtime_proof_bundle(&observation);

    let mut runtime_gates = runtime_initial_observed_gates();
    runtime_gates.extend(proof_bundle.observed_gates());
    let runtime_plan = runtime_suite::build_runtime_execution_plan_with_observed_gates(
        &registry_path,
        runtime_profile,
        runtime_gates,
    )?;
    let blocked_gates = blocked_runtime_gates(&runtime_plan);
    let next_actions = next_actions(&activation_plan, &blocked_gates);
    let status = if observation.status == SurfaceStatus::Active
        && apply_execution.status == SurfaceStatus::Active
        && proof_bundle.status == "satisfied"
        && runtime_plan.status == "ready"
    {
        "ok"
    } else {
        "blocked"
    };

    Ok(HostBootstrapE2eReport {
        schema: HOST_BOOTSTRAP_E2E_SCHEMA,
        status,
        mode: "dry_run",
        registry_path,
        runtime_profile: runtime_profile.to_string(),
        rule: "non-mutating host substrate chain: observe -> plan -> apply-report -> apply --dry-run -> proof -> runtime plan",
        observed_gates: runtime_plan.observed_gates.clone(),
        blocked_gates,
        next_actions,
        observation,
        activation_plan,
        apply_report,
        apply_execution,
        proof_bundle,
        runtime_plan,
    })
}

fn blocked_runtime_gates(plan: &RuntimeExecutionPlan) -> Vec<String> {
    let mut gates = plan
        .layers
        .iter()
        .flat_map(|layer| layer.missing_entry_gate.iter().copied())
        .map(str::to_string)
        .collect::<Vec<_>>();
    gates.sort();
    gates.dedup();
    gates
}

fn next_actions(plan: &HostActivationPlanReport, blocked_gates: &[String]) -> Vec<String> {
    let mut actions = plan
        .next_actions
        .iter()
        .map(|action| (*action).to_string())
        .collect::<Vec<_>>();
    actions.extend(
        blocked_gates
            .iter()
            .map(|gate| format!("satisfy_runtime_gate:{gate}")),
    );
    actions.sort();
    actions.dedup();
    actions
}
