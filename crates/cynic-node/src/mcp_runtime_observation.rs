//! Node-owned read-only observation of the MCP runtime contract.
//!
//! The CLI only supplies injected identifiers and renders JSON.  Contract
//! reconciliation remains in `cynic-host::mcp_runtime`, so the node, host and
//! CLI do not grow independent compatibility policies.

use std::process::ExitCode;

use cynic_host::{
    mcp_runtime::{MCP_RUNTIME_OBSERVATION_SCHEMA, McpRuntimeObservation, observe_mcp_runtime},
    cynic_lifecycle::LocalArgvRunner,
};
use cynic_protocol::{
    RUNTIME_PROOF_BUNDLE_SCHEMA, RUNTIME_PROOF_SCHEMA, RuntimeProof, RuntimeProofBundle,
    RuntimeProofEvidence,
};
use serde::Serialize;

const NODE_MCP_RUNTIME_OBSERVATION_SCHEMA: &str = "cynic.node.mcp_runtime_observation.v1";

#[derive(Debug, Serialize)]
struct NodeMcpRuntimeObservation {
    schema: &'static str,
    producer: &'static str,
    node_id: String,
    mutates: bool,
    observation_schema: &'static str,
    observation: McpRuntimeObservation,
    proof_bundle: RuntimeProofBundle,
}

pub(crate) fn execute(node_id: &str, binary: &str, proof_only: bool) -> ExitCode {
    if node_id.trim().is_empty() || binary.trim().is_empty() {
        eprintln!("node_id and binary must be non-empty");
        return ExitCode::FAILURE;
    }
    let observation = observe_mcp_runtime(binary, &LocalArgvRunner);
    let status_ok = observation.status == "observed";
    let proof_bundle = runtime_proof_bundle(node_id, &observation);
    let report = NodeMcpRuntimeObservation {
        schema: NODE_MCP_RUNTIME_OBSERVATION_SCHEMA,
        producer: "cynic-node",
        node_id: node_id.to_string(),
        mutates: false,
        observation_schema: MCP_RUNTIME_OBSERVATION_SCHEMA,
        observation,
        proof_bundle,
    };
    let encoded = if proof_only {
        serde_json::to_string(&report.proof_bundle)
    } else {
        serde_json::to_string(&report)
    };
    match encoded {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("failed to encode node MCP observation: {error}");
            return ExitCode::FAILURE;
        }
    }
    if status_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(3)
    }
}

fn runtime_proof_bundle(node_id: &str, observation: &McpRuntimeObservation) -> RuntimeProofBundle {
    let status = match observation.status.as_str() {
        "observed" => "satisfied",
        "unknown" => "unknown",
        _ => "blocked",
    };
    let detail = observation
        .compatibility
        .as_ref()
        .map(|compatibility| {
            format!(
                "compatibility={:?}; required_revision={}; observed_revision={:?}; missing_capabilities={}; required_actions={}",
                compatibility.status,
                compatibility.required_revision,
                compatibility.observed_revision,
                compatibility.missing_capabilities.join(","),
                observation.required_actions.join(","),
            )
        })
        .unwrap_or_else(|| {
            format!(
                "blockers={}; unknowns={}",
                observation.blockers.join(","),
                observation.unknowns.join(","),
            )
        });
    RuntimeProofBundle {
        schema: RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
        status: status.to_string(),
        proofs: vec![RuntimeProof {
            schema: RUNTIME_PROOF_SCHEMA.to_string(),
            gate: "mcp_runtime_contract".to_string(),
            status: status.to_string(),
            producer: "cynic-node".to_string(),
            subject: node_id.to_string(),
            observed_at_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis())
                .unwrap_or_default(),
            evidence: vec![RuntimeProofEvidence {
                id: "mcp_runtime_contract".to_string(),
                status: status.to_string(),
                detail,
            }],
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_schema_is_node_owned_and_read_only() {
        let report = NodeMcpRuntimeObservation {
            schema: NODE_MCP_RUNTIME_OBSERVATION_SCHEMA,
            producer: "cynic-node",
            node_id: "cynic-builder".to_string(),
            mutates: false,
            observation_schema: MCP_RUNTIME_OBSERVATION_SCHEMA,
            observation: McpRuntimeObservation {
                schema: MCP_RUNTIME_OBSERVATION_SCHEMA,
                status: "blocked".to_string(),
                mutates: false,
                binary: "cynic-mcp".to_string(),
                argv: vec!["cynic-mcp".to_string(), "--contract-json".to_string()],
                exit_code: Some(3),
                compatibility: None,
                required_actions: Vec::new(),
                blockers: vec!["mcp_runtime_artifact_unobservable".to_string()],
                unknowns: Vec::new(),
            },
            proof_bundle: RuntimeProofBundle {
                schema: RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
                status: "blocked".to_string(),
                proofs: Vec::new(),
            },
        };
        assert_eq!(report.schema, NODE_MCP_RUNTIME_OBSERVATION_SCHEMA);
        assert!(!report.mutates);
        assert!(!report.observation.mutates);
    }

    #[test]
    fn incompatible_contract_becomes_blocked_node_proof() {
        let observation = McpRuntimeObservation {
            schema: MCP_RUNTIME_OBSERVATION_SCHEMA,
            status: "blocked".to_string(),
            mutates: false,
            binary: "cynic-mcp".to_string(),
            argv: Vec::new(),
            exit_code: Some(3),
            compatibility: None,
            required_actions: Vec::new(),
            blockers: vec!["mcp_runtime_contract_incompatible".to_string()],
            unknowns: Vec::new(),
        };
        let proof = runtime_proof_bundle("cynic-builder", &observation);
        assert_eq!(proof.status, "blocked");
        assert_eq!(proof.proofs[0].gate, "mcp_runtime_contract");
        assert_eq!(proof.proofs[0].producer, "cynic-node");
    }
}
