use cynic_process::ArgvRunner;
use cynic_protocol::{
    RuntimeCompatibilityPlan, RuntimeCompatibilityStatus,
    runtime_compatibility::runtime_compatibility_actions,
};
use serde::Serialize;

pub const MCP_RUNTIME_OBSERVATION_SCHEMA: &str = "cynic.mcp.runtime_observation.v1";

#[derive(Debug, Clone, Serialize)]
pub struct McpRuntimeObservation {
    pub schema: &'static str,
    pub status: String,
    pub mutates: bool,
    pub binary: String,
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub compatibility: Option<RuntimeCompatibilityPlan>,
    pub required_actions: Vec<String>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
}

/// Observe the installed MCP artifact without starting a server or opening a
/// session. Exit code 3 is an expected incompatible-contract observation.
pub fn observe_mcp_runtime(binary: &str, runner: &dyn ArgvRunner) -> McpRuntimeObservation {
    let argv = vec![binary.to_string(), "--contract-json".to_string()];
    let args = argv.iter().map(String::as_str).collect::<Vec<_>>();
    let output = runner.run(&args);
    let mut blockers = Vec::new();
    let mut unknowns = Vec::new();
    let compatibility = match serde_json::from_str::<RuntimeCompatibilityPlan>(&output.stdout) {
        Ok(plan) => Some(plan),
        Err(_) if output.spawn_error.is_some() => {
            blockers.push("mcp_runtime_artifact_unobservable".to_string());
            None
        }
        Err(_) => {
            blockers.push("mcp_runtime_contract_report_invalid".to_string());
            None
        }
    };
    let required_actions = compatibility
        .as_ref()
        .map(runtime_compatibility_actions)
        .unwrap_or_default();
    let status = match compatibility.as_ref().map(|plan| &plan.status) {
        Some(RuntimeCompatibilityStatus::Compatible) if output.success() => "observed",
        Some(RuntimeCompatibilityStatus::Incompatible) if output.exit_code == Some(3) => "blocked",
        Some(RuntimeCompatibilityStatus::Unknown) => {
            unknowns.extend(
                compatibility
                    .as_ref()
                    .into_iter()
                    .flat_map(|plan| plan.unknowns.iter().cloned()),
            );
            "unknown"
        }
        Some(RuntimeCompatibilityStatus::Compatible) => {
            blockers.push("mcp_runtime_contract_exit_status_unexpected".to_string());
            "blocked"
        }
        Some(RuntimeCompatibilityStatus::Incompatible) => {
            blockers.push("mcp_runtime_contract_exit_status_unexpected".to_string());
            "blocked"
        }
        None => "blocked",
    };
    blockers.sort();
    blockers.dedup();
    unknowns.sort();
    unknowns.dedup();
    McpRuntimeObservation {
        schema: MCP_RUNTIME_OBSERVATION_SCHEMA,
        status: status.to_string(),
        mutates: false,
        binary: binary.to_string(),
        argv,
        exit_code: output.exit_code,
        compatibility,
        required_actions,
        blockers,
        unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::observe_mcp_runtime;
    use cynic_process::{ArgvOutput, MapArgvRunner};
    use cynic_protocol::{RUNTIME_COMPATIBILITY_PLAN_SCHEMA, RuntimeCompatibilityStatus};

    fn report(status: RuntimeCompatibilityStatus) -> String {
        serde_json::json!({
            "schema": RUNTIME_COMPATIBILITY_PLAN_SCHEMA,
            "status": status,
            "contract_id": "cynic.agent_mcp",
            "required_revision": 8,
            "observed_revision": 8,
            "artifact_version": "0.1.0",
            "required_contract_digest": "sha256:desired",
            "observed_contract_digest": "sha256:installed",
            "missing_capabilities": ["cynic_source_change_submit"],
            "evidence_refs": ["artifact-digest:sha256:abc"],
            "unknowns": []
        })
        .to_string()
    }

    #[test]
    fn incompatible_artifact_is_a_blocked_read_only_observation() {
        let runner = MapArgvRunner {
            outputs: [(
                "cynic-mcp --contract-json".into(),
                ArgvOutput {
                    stdout: report(RuntimeCompatibilityStatus::Incompatible),
                    stderr: String::new(),
                    exit_code: Some(3),
                    spawn_error: None,
                },
            )]
            .into_iter()
            .collect(),
            record_executed: true,
            ..Default::default()
        };

        let observed = observe_mcp_runtime("cynic-mcp", &runner);
        assert_eq!(observed.status, "blocked");
        assert!(!observed.mutates);
        assert_eq!(observed.exit_code, Some(3));
        assert_eq!(
            observed.required_actions,
            vec!["reinstall_contract_aligned_binary".to_string()]
        );
        assert_eq!(
            runner.executed(),
            vec![vec!["cynic-mcp", "--contract-json"]]
        );
    }

    #[test]
    fn unobservable_missing_binary_is_recorded_cleanly() {
        let observed = observe_mcp_runtime("missing-mcp", &MapArgvRunner::default());
        assert_eq!(observed.status, "blocked");
        assert_eq!(observed.blockers, vec!["mcp_runtime_artifact_unobservable"]);
    }
}
