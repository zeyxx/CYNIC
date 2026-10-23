//! Pure deploy planning: canary order, per-step gates, containment ladder.
//!
//! Zero I/O. The policy file (`infra/deploy-policy.json`) is parsed by the
//! caller (CLI adapter); this module only decides from parsed data:
//! - [`decide_deploy_plan`] turns an eligible release + policy + machine list
//!   into an ordered step plan (couche A: rendered, executed manually).
//! - [`decide_containment`] maps a failed probe to the next ladder rung.
//! Couche B automates execution; couche C automates the ladder.

use serde::{Deserialize, Serialize};

use crate::deploy_locations::SkippedMachine;
use crate::types::ReleaseEligibilityVerdict;

/// One health gate declared by the deploy policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployHealthGate {
    pub name: String,
    pub description: String,
    pub command: String,
    pub pass_condition: String,
}

/// One containment rung declared by the deploy policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainmentRung {
    pub rung: u32,
    pub name: String,
    pub action: String,
    pub clears: String,
}

/// Machine-readable deploy policy (`cynic.deploy_policy.v1`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployPolicyContract {
    pub schema: String,
    pub status: String,
    pub canary_order: Vec<String>,
    pub stop_before_swap: bool,
    pub containment_ladder: Vec<ContainmentRung>,
}

impl DeployPolicyContract {
    pub fn parse(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Health gates are informational for the plan; the executor resolves
    /// them from the raw policy document (kept untyped here on purpose:
    /// gate semantics belong to the execution layer, not the decision).
    pub fn ladder_len(&self) -> usize {
        self.containment_ladder.len()
    }
}

/// One planned deploy step: a machine and the gates that promote it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployStep {
    pub machine: String,
    pub order: usize,
    pub gate_names: Vec<String>,
}

/// An ordered canary plan for one release on a subset of the fleet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployPlan {
    pub schema: &'static str,
    pub commit_sha: String,
    pub steps: Vec<DeployStep>,
    pub ladder_rungs: usize,
    /// Machines the registry did not opt in, with reasons. Rendered,
    /// never silent. Defaults empty so older serialized plans still read.
    #[serde(default)]
    pub skipped: Vec<SkippedMachine>,
}

/// Decide the canary plan: machines ordered by the policy, restricted to
/// the machines the caller passes (unknown names are refused, never
/// appended). A non-eligible release yields no plan.
pub fn decide_deploy_plan(
    verdict: &ReleaseEligibilityVerdict,
    policy: &DeployPolicyContract,
    machines: &[String],
    gate_names: &[String],
    commit_sha: &str,
) -> Result<DeployPlan, String> {
    decide_deploy_plan_with_skips(verdict, policy, machines, &[], gate_names, commit_sha)
}

/// Decide a canary plan and retain explicit fleet opt-out reasons.
///
/// An empty plan is valid only when every requested machine was skipped by
/// the opt-in decision. An unmatched opted-in machine remains an error.
pub fn decide_deploy_plan_with_skips(
    verdict: &ReleaseEligibilityVerdict,
    policy: &DeployPolicyContract,
    machines: &[String],
    skipped: &[SkippedMachine],
    gate_names: &[String],
    commit_sha: &str,
) -> Result<DeployPlan, String> {
    if !verdict.is_eligible() {
        return Err("release is not eligible: no deploy plan".to_string());
    }
    if policy.canary_order.is_empty() {
        return Err("deploy policy declares no canary order".to_string());
    }
    let mut steps = Vec::new();
    for (order, machine) in policy.canary_order.iter().enumerate() {
        if !machines.contains(machine) {
            continue;
        }
        steps.push(DeployStep {
            machine: machine.clone(),
            order,
            gate_names: gate_names.to_vec(),
        });
    }
    if steps.is_empty() && (!machines.is_empty() || skipped.is_empty()) {
        return Err("no known machine matches the canary order".to_string());
    }
    Ok(DeployPlan {
        schema: "cynic.deploy_plan.v1",
        commit_sha: commit_sha.to_string(),
        steps,
        ladder_rungs: policy.ladder_len(),
        skipped: skipped.to_vec(),
    })
}

/// Map a failed probe to the next containment rung (1-based). Rung 0 means
/// "no failure, keep promoting". Past the last rung means "stop, diagnose".
/// Pure: the ladder itself comes from the policy.
pub fn decide_containment(failed_rung: u32, ladder_len: u32) -> ContainmentDecision {
    if failed_rung == 0 {
        return ContainmentDecision::Promote;
    }
    if failed_rung > ladder_len {
        return ContainmentDecision::StopAndDiagnose;
    }
    ContainmentDecision::Climb { rung: failed_rung }
}

/// Next action after a probe result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainmentDecision {
    Promote,
    Climb { rung: u32 },
    StopAndDiagnose,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ReleaseVersion;

    fn eligible_verdict() -> ReleaseEligibilityVerdict {
        ReleaseEligibilityVerdict::Eligible {
            schema: "cynic.release_verdict.v1",
            version: ReleaseVersion::new(0, 2, 1, None),
            commit_sha: "abc123".to_string(),
            tag_name: "v0.2.1".to_string(),
            decided_at_unix_s: 100,
        }
    }

    fn blocked_verdict() -> ReleaseEligibilityVerdict {
        ReleaseEligibilityVerdict::Blocked {
            schema: "cynic.release_verdict.v1",
            reasons: vec!["cargo_check failed".to_string()],
            decided_at_unix_s: 100,
        }
    }

    fn policy() -> DeployPolicyContract {
        DeployPolicyContract {
            schema: "cynic.deploy_policy.v1".to_string(),
            status: "normative".to_string(),
            canary_order: vec![
                "builder".to_string(),
                "cortex".to_string(),
                "cynic-agent-host".to_string(),
            ],
            stop_before_swap: true,
            containment_ladder: vec![
                ContainmentRung {
                    rung: 1,
                    name: "config_out".to_string(),
                    action: "remove stanza".to_string(),
                    clears: "config regressions".to_string(),
                },
                ContainmentRung {
                    rung: 2,
                    name: "binary_rollback".to_string(),
                    action: "restore bak".to_string(),
                    clears: "code regressions".to_string(),
                },
            ],
        }
    }

    #[test]
    fn plan_follows_canary_order_and_skips_unknown_machines() {
        let plan = decide_deploy_plan(
            &eligible_verdict(),
            &policy(),
            &[
                "cortex".to_string(),
                "builder".to_string(),
                "ghost".to_string(),
            ],
            &["version_match".to_string()],
            "abc123",
        )
        .expect("plan");
        let machines: Vec<&str> = plan.steps.iter().map(|s| s.machine.as_str()).collect();
        assert_eq!(machines, vec!["builder", "cortex"]);
        assert_eq!(plan.steps[0].order, 0);
        assert_eq!(plan.steps[1].order, 1);
    }

    #[test]
    fn plan_preserves_opt_out_reasons_when_every_machine_is_skipped() {
        let skipped = vec![SkippedMachine {
            machine: "cortex".to_string(),
            reason: "deploy_target is false".to_string(),
        }];
        let plan = decide_deploy_plan_with_skips(
            &eligible_verdict(),
            &policy(),
            &[],
            &skipped,
            &["identity_match".to_string()],
            "abc123",
        )
        .expect("all machines opting out is a valid empty plan");

        assert!(plan.steps.is_empty());
        assert_eq!(plan.skipped, skipped);
        assert_eq!(plan.ladder_rungs, 2);
    }

    #[test]
    fn plan_refuses_unmatched_opted_in_machines_even_when_other_machines_are_skipped() {
        let skipped = vec![SkippedMachine {
            machine: "cortex".to_string(),
            reason: "deploy_target is false".to_string(),
        }];
        let err = decide_deploy_plan_with_skips(
            &eligible_verdict(),
            &policy(),
            &["ghost".to_string()],
            &skipped,
            &["identity_match".to_string()],
            "abc123",
        )
        .expect_err("a skipped machine must not hide an unmatched opted-in machine");

        assert!(err.contains("no known machine"));
    }

    #[test]
    fn plan_refused_for_blocked_release() {
        let err = decide_deploy_plan(
            &blocked_verdict(),
            &policy(),
            &["builder".to_string()],
            &[],
            "abc123",
        )
        .expect_err("must refuse");
        assert!(err.contains("not eligible"));
    }

    #[test]
    fn plan_refused_when_no_machine_matches() {
        let err = decide_deploy_plan(
            &eligible_verdict(),
            &policy(),
            &["ghost".to_string()],
            &[],
            "abc123",
        )
        .expect_err("must refuse");
        assert!(err.contains("no known machine"));
    }

    #[test]
    fn containment_climbs_then_stops() {
        assert_eq!(decide_containment(0, 2), ContainmentDecision::Promote);
        assert_eq!(
            decide_containment(1, 2),
            ContainmentDecision::Climb { rung: 1 }
        );
        assert_eq!(
            decide_containment(2, 2),
            ContainmentDecision::Climb { rung: 2 }
        );
        assert_eq!(
            decide_containment(3, 2),
            ContainmentDecision::StopAndDiagnose
        );
    }

    #[test]
    fn policy_parses_from_json() {
        let json = r#"{"schema":"cynic.deploy_policy.v1","status":"normative","canary_order":["builder"],"stop_before_swap":true,"containment_ladder":[]}"#;
        let policy = DeployPolicyContract::parse(json).expect("parse");
        assert_eq!(policy.canary_order, vec!["builder".to_string()]);
        assert!(policy.stop_before_swap);
    }
}
