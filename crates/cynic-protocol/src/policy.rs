use serde::{Deserialize, Serialize};

pub const POLICY_ADMISSION_REPORT_SCHEMA: &str = "cynic.policy_admission_report.v1";
pub const EVOLUTIVE_POLICY_SCHEMA: &str = "cynic.evolutive_policy.v1";
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

    #[test]
    fn evolutive_policy_hydrates_from_foundation_law() {
        let build_policy = EvolutivePolicy::foundation(EvolutivePolicyDomain::Build);
        assert_eq!(build_policy.schema, EVOLUTIVE_POLICY_SCHEMA);
        assert_eq!(build_policy.domain, EvolutivePolicyDomain::Build);
        assert!(build_policy.default_gate.contains("make check"));
        assert!(build_policy.build_gate.is_some());

        let storage_policy = EvolutivePolicy::foundation(EvolutivePolicyDomain::Storage);
        assert_eq!(storage_policy.domain, EvolutivePolicyDomain::Storage);
        assert!(storage_policy.default_gate.contains("fail-closed"));
    }
}
