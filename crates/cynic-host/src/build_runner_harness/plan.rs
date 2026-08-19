use cynic_protocol::{
    reconcile_harness_matrix, HarnessObservation,
};

use super::observe::observe_harness_scenarios;
use super::registry::load_harness_scenarios;
use super::types::{BuildRunnerHarnessReport, BUILD_RUNNER_HARNESS_REPORT_SCHEMA};

pub fn build_runner_harness_plan(
    registry_path: &str,
    custom_observations: Option<&[HarnessObservation]>,
) -> Result<BuildRunnerHarnessReport, String> {
    let scenarios = load_harness_scenarios(registry_path)?;
    let observations = observe_harness_scenarios(&scenarios, custom_observations);
    let matrix = reconcile_harness_matrix(&scenarios, &observations);

    Ok(BuildRunnerHarnessReport {
        schema: BUILD_RUNNER_HARNESS_REPORT_SCHEMA.to_string(),
        registry_path: registry_path.to_string(),
        status: matrix.status,
        cells: matrix.cells.clone(),
        matrix: matrix.clone(),
        mutates: false,
        can_apply: false,
        blockers: matrix.blockers,
        residual_unknowns: matrix.residual_unknowns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::HarnessStatus;

    #[test]
    fn build_runner_harness_plan_generates_six_cells_and_is_non_mutating() {
        let registry = format!("{}/../../infra/registry.json", env!("CARGO_MANIFEST_DIR"));
        let report = build_runner_harness_plan(&registry, None).expect("harness plan");

        assert_eq!(report.schema, BUILD_RUNNER_HARNESS_REPORT_SCHEMA);
        assert!(!report.mutates);
        assert!(!report.can_apply);
        assert_eq!(report.cells.len(), 6);

        let staging = report
            .cell(HarnessScenarioId::BuilderStagingObserve)
            .expect("staging cell");
        assert_eq!(staging.status, HarnessStatus::Unknown);
        assert!(staging
            .unknowns
            .contains(&"provider_observation_unavailable".to_string()));

        let canary = report
            .cell(HarnessScenarioId::BuilderCanaryApply)
            .expect("canary cell");
        assert_eq!(canary.status, HarnessStatus::Blocked);
        assert!(canary
            .blockers
            .contains(&"staging_proof_missing".to_string()));
    }
}
