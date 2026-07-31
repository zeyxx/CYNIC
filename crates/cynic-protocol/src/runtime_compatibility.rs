use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const RUNTIME_COMPATIBILITY_PLAN_SCHEMA: &str = "cynic.runtime_compatibility.plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeContractRequirement {
    pub contract_id: String,
    pub contract_revision: u64,
    pub required_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeContractObservation {
    pub component: String,
    pub artifact_version: Option<String>,
    pub contract_id: Option<String>,
    pub contract_revision: Option<u64>,
    pub capabilities: Option<Vec<String>>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeCompatibilityStatus {
    Compatible,
    Incompatible,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeCompatibilityPlan {
    pub schema: String,
    pub status: RuntimeCompatibilityStatus,
    pub contract_id: String,
    pub required_revision: u64,
    pub observed_revision: Option<u64>,
    pub artifact_version: Option<String>,
    pub missing_capabilities: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

pub fn reconcile_runtime_compatibility(
    requirement: &RuntimeContractRequirement,
    observation: &RuntimeContractObservation,
) -> RuntimeCompatibilityPlan {
    let mut unknowns = observation.unknowns.clone();
    let observed_capabilities = observation
        .capabilities
        .as_ref()
        .map(|items| items.iter().map(String::as_str).collect::<BTreeSet<_>>());
    let missing_capabilities: Vec<String> = observed_capabilities
        .as_ref()
        .map(|observed| {
            requirement
                .required_capabilities
                .iter()
                .filter(|capability| !observed.contains(capability.as_str()))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    if observation.contract_id.is_none() {
        unknowns.push("runtime_contract_id_unobserved".to_string());
    }
    if observation.contract_revision.is_none() {
        unknowns.push("runtime_contract_revision_unobserved".to_string());
    }
    if observation.capabilities.is_none() {
        unknowns.push("runtime_capabilities_unobserved".to_string());
    }
    unknowns.sort();
    unknowns.dedup();

    let identity_mismatch = observation
        .contract_id
        .as_ref()
        .is_some_and(|observed| observed != &requirement.contract_id);
    let revision_too_old = observation
        .contract_revision
        .is_some_and(|observed| observed < requirement.contract_revision);
    let status = if identity_mismatch || revision_too_old || !missing_capabilities.is_empty() {
        RuntimeCompatibilityStatus::Incompatible
    } else if !unknowns.is_empty() {
        RuntimeCompatibilityStatus::Unknown
    } else {
        RuntimeCompatibilityStatus::Compatible
    };

    RuntimeCompatibilityPlan {
        schema: RUNTIME_COMPATIBILITY_PLAN_SCHEMA.to_string(),
        status,
        contract_id: requirement.contract_id.clone(),
        required_revision: requirement.contract_revision,
        observed_revision: observation.contract_revision,
        artifact_version: observation.artifact_version.clone(),
        missing_capabilities,
        evidence_refs: observation.evidence_refs.clone(),
        unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requirement() -> RuntimeContractRequirement {
        RuntimeContractRequirement {
            contract_id: "cynic.agent_mcp".to_string(),
            contract_revision: 2,
            required_capabilities: vec![
                "cynic_mcp_surface".to_string(),
                "cynic_source_control".to_string(),
            ],
        }
    }

    #[test]
    fn rejects_a_new_artifact_that_lacks_a_required_capability() {
        let plan = reconcile_runtime_compatibility(
            &requirement(),
            &RuntimeContractObservation {
                component: "cynic-mcp".to_string(),
                artifact_version: Some("v99.0.0".to_string()),
                contract_id: Some("cynic.agent_mcp".to_string()),
                contract_revision: Some(2),
                capabilities: Some(vec!["cynic_mcp_surface".to_string()]),
                evidence_refs: vec!["binary:sha256:abc".to_string()],
                unknowns: vec![],
            },
        );

        assert_eq!(plan.status, RuntimeCompatibilityStatus::Incompatible);
        assert_eq!(plan.missing_capabilities, vec!["cynic_source_control"]);
    }

    #[test]
    fn preserves_unobserved_capabilities_as_unknown() {
        let plan = reconcile_runtime_compatibility(
            &requirement(),
            &RuntimeContractObservation {
                component: "cynic-mcp".to_string(),
                artifact_version: Some("v2".to_string()),
                contract_id: Some("cynic.agent_mcp".to_string()),
                contract_revision: Some(2),
                capabilities: None,
                evidence_refs: vec![],
                unknowns: vec![],
            },
        );

        assert_eq!(plan.status, RuntimeCompatibilityStatus::Unknown);
        assert!(
            plan.unknowns
                .contains(&"runtime_capabilities_unobserved".to_string())
        );
    }

    #[test]
    fn accepts_revision_floor_with_all_capabilities() {
        let plan = reconcile_runtime_compatibility(
            &requirement(),
            &RuntimeContractObservation {
                component: "cynic-mcp".to_string(),
                artifact_version: Some("v2".to_string()),
                contract_id: Some("cynic.agent_mcp".to_string()),
                contract_revision: Some(3),
                capabilities: Some(vec![
                    "cynic_source_control".to_string(),
                    "cynic_mcp_surface".to_string(),
                ]),
                evidence_refs: vec![],
                unknowns: vec![],
            },
        );

        assert_eq!(plan.status, RuntimeCompatibilityStatus::Compatible);
    }
}
