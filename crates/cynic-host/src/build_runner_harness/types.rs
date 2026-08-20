use cynic_protocol::{HarnessCellReport, HarnessMatrixReport, HarnessScenarioId, HarnessStatus};
use serde::{Deserialize, Serialize};

pub const BUILD_RUNNER_HARNESS_REPORT_SCHEMA: &str = "cynic.build_runner_harness.report.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRunnerHarnessReport {
    pub schema: String,
    pub registry_path: String,
    pub status: HarnessStatus,
    pub cells: Vec<HarnessCellReport>,
    pub matrix: HarnessMatrixReport,
    pub mutates: bool,
    pub can_apply: bool,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

impl BuildRunnerHarnessReport {
    pub fn cell(&self, id: HarnessScenarioId) -> Option<&HarnessCellReport> {
        self.cells.iter().find(|cell| cell.scenario_id == id)
    }
}
