use serde::{Deserialize, Serialize};

pub const POLICY_ADMISSION_REPORT_SCHEMA: &str = "cynic.policy_admission_report.v1";
pub const EXECUTION_AUTHORIZATION_SCHEMA: &str = "cynic.execution_authorization.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicySetRef {
    pub id: String,
    pub version: String,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyIntentRef {
    pub id: String,
    pub kind: String,
    pub actor: String,
    pub subject: String,
    pub digest: String,
    pub plan_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyScope {
    pub domain: String,
    pub target: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAuthority {
    ReadOnly,
    RuntimePrincipal,
    HumanOperator,
    HumanAdmin,
    FleetCanary,
    RuntimeAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionAuthorization {
    pub schema: String,
    pub actor: String,
    pub authority: PolicyAuthority,
    pub explicit_apply: bool,
    #[serde(default)]
    pub satisfied_preconditions: Vec<String>,
}

impl ExecutionAuthorization {
    pub fn new(
        actor: impl Into<String>,
        authority: PolicyAuthority,
        explicit_apply: bool,
        satisfied_preconditions: Vec<String>,
    ) -> Self {
        Self {
            schema: EXECUTION_AUTHORIZATION_SCHEMA.to_string(),
            actor: actor.into(),
            authority,
            explicit_apply,
            satisfied_preconditions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequiredProof {
    pub gate: String,
    pub subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationContract {
    pub kind: String,
    pub scope: PolicyScope,
    pub mutates: bool,
    pub requires_privilege: bool,
    pub requires_explicit_apply: bool,
    pub required_authority: PolicyAuthority,
    pub preconditions: Vec<String>,
    pub rollback_intent: String,
    pub expected_proofs: Vec<RequiredProof>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyViolation {
    pub policy_id: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum PolicyDecision {
    Allow {
        required_authority: PolicyAuthority,
        required_proofs: Vec<RequiredProof>,
    },
    Deny {
        violations: Vec<PolicyViolation>,
    },
    Unknown {
        missing_evidence: Vec<String>,
    },
}

impl PolicyDecision {
    pub fn admitted(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyAdmissionReport {
    pub schema: String,
    pub policy_set: PolicySetRef,
    pub intent: PolicyIntentRef,
    pub mutation: MutationContract,
    pub decision: PolicyDecision,
}

impl PolicyAdmissionReport {
    pub fn new(
        policy_set: PolicySetRef,
        intent: PolicyIntentRef,
        mutation: MutationContract,
        decision: PolicyDecision,
    ) -> Self {
        Self {
            schema: POLICY_ADMISSION_REPORT_SCHEMA.to_string(),
            policy_set,
            intent,
            mutation,
            decision,
        }
    }

    pub fn admitted(&self) -> bool {
        self.schema == POLICY_ADMISSION_REPORT_SCHEMA && self.decision.admitted()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mutation() -> MutationContract {
        MutationContract {
            kind: "service_control".to_string(),
            scope: PolicyScope {
                domain: "host".to_string(),
                target: "node-a".to_string(),
            },
            mutates: true,
            requires_privilege: true,
            requires_explicit_apply: true,
            required_authority: PolicyAuthority::HumanOperator,
            preconditions: vec!["service_declared".to_string()],
            rollback_intent: "restore previous service state".to_string(),
            expected_proofs: vec![RequiredProof {
                gate: "service_active".to_string(),
                subject: "node-a".to_string(),
            }],
        }
    }

    fn report(decision: PolicyDecision) -> PolicyAdmissionReport {
        PolicyAdmissionReport::new(
            PolicySetRef {
                id: "host.service".to_string(),
                version: "v1".to_string(),
                digest: "sha256:policy".to_string(),
            },
            PolicyIntentRef {
                id: "intent-a".to_string(),
                kind: "service_control".to_string(),
                actor: "operator-a".to_string(),
                subject: "node-a".to_string(),
                digest: "sha256:intent".to_string(),
                plan_digest: "sha256:plan".to_string(),
            },
            mutation(),
            decision,
        )
    }

    #[test]
    fn only_allow_is_admitted() {
        let required_proofs = mutation().expected_proofs;
        assert!(
            report(PolicyDecision::Allow {
                required_authority: PolicyAuthority::HumanOperator,
                required_proofs,
            })
            .admitted()
        );
        assert!(
            !report(PolicyDecision::Unknown {
                missing_evidence: vec!["registry".to_string()],
            })
            .admitted()
        );
        assert!(
            !report(PolicyDecision::Deny {
                violations: vec![PolicyViolation {
                    policy_id: "host.service".to_string(),
                    code: "role_not_allowed".to_string(),
                    message: "role is outside the admitted scope".to_string(),
                }],
            })
            .admitted()
        );
    }

    #[test]
    fn decision_serialization_is_explicit_and_machine_readable() {
        let value = serde_json::to_value(report(PolicyDecision::Unknown {
            missing_evidence: vec!["service_observation".to_string()],
        }))
        .expect("serialize admission report");

        assert_eq!(
            value.pointer("/decision/decision").and_then(|v| v.as_str()),
            Some("unknown")
        );
        assert_eq!(
            value.pointer("/mutation/requires_explicit_apply"),
            Some(&serde_json::Value::Bool(true))
        );
    }

    #[test]
    fn human_admin_authorization_is_explicit_and_machine_readable() {
        let value = serde_json::to_value(ExecutionAuthorization::new(
            "admin-a",
            PolicyAuthority::HumanAdmin,
            true,
            vec!["principal_policy_declared".to_string()],
        ))
        .expect("serialize execution authorization");

        assert_eq!(
            value.get("schema").and_then(|value| value.as_str()),
            Some(EXECUTION_AUTHORIZATION_SCHEMA)
        );
        assert_eq!(
            value.get("authority").and_then(|value| value.as_str()),
            Some("human_admin")
        );
    }
}
