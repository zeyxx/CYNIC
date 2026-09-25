use serde::Serialize;

use crate::hardware_baseline::{HardwareBaselinePlan, plan_hardware_baseline};
use crate::hardware_baseline_reconcile::HardwareBaselineReconcileReport;
use crate::hardware_conformance::observe_hardware_conformance;
use crate::ports::HostHardwareObservationPort;
use crate::profiles::HostProfile;
use crate::registry::HostProfileResolution;
use crate::runtime_layout::current_runtime_layout;
use crate::usecases::{
    HostActivationPlanReport, HostApplyExecutionReport, HostObservationReports,
    build_host_activation_plan,
};

pub const HOST_BOOTSTRAP_COMPOSITE_PLAN_SCHEMA: &str = "cynic.host.bootstrap_composite_plan.v1";
pub const HOST_BOOTSTRAP_COMPOSITE_APPLY_SCHEMA: &str = "cynic.host.bootstrap_composite_apply.v1";

#[derive(Debug, Serialize)]
pub struct HostBootstrapCompositePlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub substrate: HostActivationPlanReport,
    pub hardware_baseline: HardwareBaselinePlan,
    pub authority_boundaries: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct HostBootstrapCompositeApply {
    pub schema: &'static str,
    pub status: &'static str,
    pub dry_run: bool,
    pub substrate: HostApplyExecutionReport,
    pub hardware_baseline: HardwareBaselineReconcileReport,
    pub authority_boundaries: Vec<&'static str>,
}

pub fn combine_host_bootstrap_apply(
    substrate: HostApplyExecutionReport,
    hardware_baseline: HardwareBaselineReconcileReport,
) -> HostBootstrapCompositeApply {
    let dry_run = substrate.dry_run;
    let hardware_ok = matches!(
        hardware_baseline.status,
        "planned" | "converged" | "completed"
    );
    let substrate_ok = dry_run || substrate.status == crate::domain::shared::SurfaceStatus::Active;
    let status = if substrate_ok && hardware_ok {
        if dry_run { "planned" } else { "completed" }
    } else {
        "blocked"
    };
    HostBootstrapCompositeApply {
        schema: HOST_BOOTSTRAP_COMPOSITE_APPLY_SCHEMA,
        status,
        dry_run,
        substrate,
        hardware_baseline,
        authority_boundaries: vec![
            "substrate activation is admitted as HumanOperator",
            "hardware enrollment crosses the bounded helper as HumanAdmin",
<<<<<<< HEAD
            "agent runtime adoption requires AutonomousCandidate proof and explicit operator consent",
=======
>>>>>>> github/main
        ],
    }
}

pub fn build_host_bootstrap_composite_plan(
    profile: &HostProfile,
    observed: &HostObservationReports,
    resolution: Option<&HostProfileResolution>,
    observer: &impl HostHardwareObservationPort,
) -> HostBootstrapCompositePlan {
    let substrate = build_host_activation_plan(profile, observed);
    let hardware = observe_hardware_conformance(observer);
    let subject = resolution
        .map(|value| value.hostname_observed.as_str())
        .unwrap_or("unresolved-host");
    let hardware_baseline = plan_hardware_baseline(
        subject,
        "cynic-bootstrap-host",
        &current_runtime_layout().hardware_baseline_path(),
        &hardware,
    );
    let substrate_mutates = !substrate.next_actions.is_empty();
    let status = if hardware_baseline.status == "blocked" {
        "blocked"
    } else if !substrate_mutates && hardware_baseline.status == "converged" {
        "converged"
    } else {
        "ready"
    };
    HostBootstrapCompositePlan {
        schema: HOST_BOOTSTRAP_COMPOSITE_PLAN_SCHEMA,
        status,
        mutates: substrate_mutates || hardware_baseline.mutates,
        substrate,
        hardware_baseline,
        authority_boundaries: vec![
            "substrate activation requires its own HumanOperator authorization",
            "hardware baseline enrollment requires a separate HumanAdmin authorization",
            "hardware drift is never repaired by initial enrollment",
<<<<<<< HEAD
            "agent runtime adoption requires AutonomousCandidate proof and explicit operator consent",
=======
>>>>>>> github/main
        ],
    }
}
