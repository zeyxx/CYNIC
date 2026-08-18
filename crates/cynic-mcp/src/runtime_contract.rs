use serde_json::Value;
use std::path::Path;

/// Observe the executable that is producing a runtime-contract report.
///
/// This deliberately reports only a content digest and path reference. It
/// does not infer provenance from a version string, and it leaves an
/// unreadable executable explicit as an unknown.
fn running_artifact_evidence() -> (Vec<String>, Vec<String>) {
    let Ok(path) = std::env::current_exe() else {
        return (
            Vec::new(),
            vec!["runtime_artifact_path_unobserved".to_string()],
        );
    };
    let path_ref = format!("artifact-path:{}", path.display());
    match std::fs::read(&path) {
        Ok(bytes) => (
            vec![
                path_ref,
                format!(
                    "artifact-digest:sha256:{}",
                    cynic_host::sha256_digest(&bytes)
                ),
            ],
            Vec::new(),
        ),
        Err(_) => (
            vec![path_ref],
            vec!["runtime_artifact_digest_unobserved".to_string()],
        ),
    }
}

/// Reconcile the desired exposure contract with the contract compiled into
/// this artifact. Both CLI and MCP projections consume this single decision.
pub fn reconcile_compiled_contract(
    desired: &Value,
    compiled: &Value,
) -> Result<cynic_protocol::RuntimeCompatibilityPlan, String> {
    let desired_handshake = desired
        .get("session_handshake")
        .ok_or_else(|| "desired MCP handshake is absent".to_string())?;
    let compiled_handshake = compiled
        .get("session_handshake")
        .ok_or_else(|| "compiled MCP handshake is absent".to_string())?;
    let desired_contract_digest = cynic_protocol::digest_json(desired)
        .map_err(|error| format!("cannot digest desired MCP contract: {error}"))?;
    let compiled_contract_digest = cynic_protocol::digest_json(compiled)
        .map_err(|error| format!("cannot digest compiled MCP contract: {error}"))?;
    let requirement = cynic_protocol::RuntimeContractRequirement {
        contract_id: desired_handshake
            .get("contract_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        contract_revision: desired_handshake
            .get("contract_revision")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        required_capabilities: desired_handshake
            .get("required_capabilities")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect(),
        contract_digest: Some(desired_contract_digest),
    };
    let mut capabilities = crate::protocol::exposed_tool_names(compiled);
    capabilities.extend(crate::protocol::source_capabilities(compiled));
    let (artifact_evidence, artifact_unknowns) = running_artifact_evidence();
    let mut evidence_refs = vec![
        format!("artifact:cynic-mcp:{}", env!("CYNIC_VERSION")),
        format!("source-commit:{}", env!("CYNIC_SOURCE_COMMIT")),
        format!("source-dirty:{}", env!("CYNIC_SOURCE_DIRTY")),
    ];
    evidence_refs.extend(artifact_evidence);
    Ok(cynic_protocol::reconcile_runtime_compatibility(
        &requirement,
        &cynic_protocol::RuntimeContractObservation {
            component: "cynic-mcp".to_string(),
            artifact_version: Some(env!("CYNIC_VERSION").to_string()),
            contract_id: compiled_handshake
                .get("contract_id")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            contract_revision: compiled_handshake
                .get("contract_revision")
                .and_then(Value::as_u64),
            capabilities: Some(capabilities),
            contract_digest: Some(compiled_contract_digest.clone()),
            evidence_refs,
            unknowns: artifact_unknowns,
        },
    ))
}

/// Observe the runtime contract from the declared project root.  This is the
/// one reconciliation used by the command-line report and mutating MCP
/// admission; a projection must not independently decide whether its own
/// artifact is current.
pub fn observe_runtime_contract(
    project_root: &Path,
) -> Result<cynic_protocol::RuntimeCompatibilityPlan, String> {
    let compiled: Value = serde_json::from_str(include_str!("../../../infra/agent-exposure.json"))
        .map_err(|error| format!("compiled MCP contract is invalid: {error}"))?;
    let desired_bytes = std::fs::read(project_root.join("infra/agent-exposure.json"))
        .map_err(|error| format!("desired MCP contract is unobservable: {error}"))?;
    let desired: Value = serde_json::from_slice(&desired_bytes)
        .map_err(|error| format!("desired MCP contract is invalid: {error}"))?;
    reconcile_compiled_contract(&desired, &compiled)
}

/// Compatibility is an admission condition for mutation, never for
/// observation.  Keeping this predicate beside reconciliation prevents each
/// MCP router from inventing its own interpretation of `unknown` or
/// `incompatible`.
pub fn admits_mutation(plan: &cynic_protocol::RuntimeCompatibilityPlan) -> bool {
    plan.status == cynic_protocol::RuntimeCompatibilityStatus::Compatible
}

#[cfg(test)]
mod tests {
    use super::{admits_mutation, reconcile_compiled_contract};
    use serde_json::json;

    #[test]
    fn shared_reconciler_rejects_missing_compiled_capability() {
        let desired = json!({
            "session_handshake": {
                "contract_id": "cynic.agent_mcp",
                "contract_revision": 2,
                "required_capabilities": ["cynic_mcp_surface", "cynic_source_control"]
            }
        });
        let compiled = json!({
            "session_handshake": {
                "contract_id": "cynic.agent_mcp",
                "contract_revision": 2,
                "required_capabilities": ["cynic_mcp_surface"]
            },
            "exposed_tools": ["cynic_mcp_surface"]
        });
        let plan = reconcile_compiled_contract(&desired, &compiled).expect("plan");
        assert_eq!(
            plan.status,
            cynic_protocol::RuntimeCompatibilityStatus::Incompatible
        );
        assert!(
            plan.missing_capabilities
                .contains(&"cynic_source_control".to_string())
        );
        assert!(
            plan.evidence_refs
                .iter()
                .any(|reference| reference.starts_with("artifact-digest:sha256:"))
        );
    }

    #[test]
    fn only_a_compatible_contract_admits_mutation() {
        let desired = json!({
            "session_handshake": {
                "contract_id": "cynic.agent_mcp",
                "contract_revision": 2,
                "required_capabilities": ["cynic_mcp_surface"]
            },
            "exposed_tools": ["cynic_mcp_surface"]
        });
        let compiled = desired.clone();
        let compatible = reconcile_compiled_contract(&desired, &compiled).expect("plan");
        assert!(admits_mutation(&compatible));

        let incompatible = reconcile_compiled_contract(
            &desired,
            &json!({
                "session_handshake": {
                    "contract_id": "cynic.agent_mcp",
                    "contract_revision": 1,
                    "required_capabilities": []
                },
                "exposed_tools": []
            }),
        )
        .expect("plan");
        assert!(!admits_mutation(&incompatible));
    }
}
