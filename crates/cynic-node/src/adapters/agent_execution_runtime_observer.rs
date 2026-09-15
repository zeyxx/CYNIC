use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};

use cynic_workload::{
    AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA,
    AgentExecutionRuntimeObservation,
};
use sha2::{Digest, Sha256};

pub const DEFAULT_AGENT_EXECUTION_RUNTIME_PROOF: &str =
    "/var/lib/cynic/agent-scheduler/runtime-proof.json";

/// Read the node-owned execution proof supplied by the deployed scheduler.
/// An absent or malformed proof never promotes the runtime to readiness.
pub fn observe_from_env() -> AgentExecutionRuntimeObservation {
    let path = configured_path();
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let mut observation = AgentExecutionRuntimeObservation::unobserved();
            observation.unknowns.push(format!(
                "node_owned_agent_execution_proof_read_failed:{error}"
            ));
            return observation;
        }
    };
    let mut observation = match serde_json::from_slice::<AgentExecutionRuntimeObservation>(&bytes) {
        Ok(observation) => observation,
        Err(error) => {
            let mut observation = AgentExecutionRuntimeObservation::unobserved();
            observation.unknowns.push(format!(
                "node_owned_agent_execution_proof_decode_failed:{error}"
            ));
            return observation;
        }
    };
    if observation.schema != AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA {
        observation
            .unknowns
            .push("node_owned_agent_execution_proof_schema_invalid".to_string());
        return observation;
    }
    let digest = format!("sha256:{}", hex::encode(Sha256::digest(&bytes)));
    observation
        .evidence_refs
        .push(format!("agent-execution-runtime-proof:{digest}"));
    observation.evidence_refs.sort();
    observation.evidence_refs.dedup();
    observation
}

/// Merge a capability proof produced by a node-owned runtime transition into
/// the configured observation file and read it back exactly. This is the only
/// writer for the scheduler proof surface; MCP remains read-only.
pub fn persist_capabilities(
    capabilities: &[&str],
    evidence_refs: &[String],
) -> Result<AgentExecutionRuntimeObservation, String> {
    let path = configured_path();
    if fs::symlink_metadata(&path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err("node_owned_agent_execution_proof_path_symlink".to_string());
    }
    let mut observation = if path.is_file() {
        observe_from_env()
    } else {
        AgentExecutionRuntimeObservation {
            schema: AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA.to_string(),
            observed_capabilities: Vec::new(),
            evidence_refs: Vec::new(),
            unknowns: Vec::new(),
        }
    };
    if observation.schema != AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA {
        return Err("node_owned_agent_execution_proof_schema_invalid".to_string());
    }
    if let Some(unknown) = observation
        .unknowns
        .iter()
        .find(|unknown| unknown.starts_with("node_owned_agent_execution_proof_"))
    {
        return Err(unknown.clone());
    }
    observation
        .observed_capabilities
        .extend(capabilities.iter().map(|value| (*value).to_string()));
    observation
        .evidence_refs
        .extend(evidence_refs.iter().cloned());
    observation.observed_capabilities.sort();
    observation.observed_capabilities.dedup();
    observation.evidence_refs.sort();
    observation.evidence_refs.dedup();
    let bytes = serde_json::to_vec_pretty(&observation)
        .map_err(|error| format!("node_owned_agent_execution_proof_encode_failed:{error}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("node_owned_agent_execution_proof_parent_failed:{error}"))?;
    }
    let temp = path.with_extension(format!("proof-{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| format!("node_owned_agent_execution_proof_write_failed:{error}"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("node_owned_agent_execution_proof_sync_failed:{error}"))?;
    fs::rename(&temp, &path)
        .map_err(|error| format!("node_owned_agent_execution_proof_publish_failed:{error}"))?;
    if let Some(parent) = path.parent() {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| {
                format!("node_owned_agent_execution_proof_directory_sync_failed:{error}")
            })?;
    }
    let read_back = observe_from_env();
    if !capabilities.iter().all(|capability| {
        read_back
            .observed_capabilities
            .iter()
            .any(|observed| observed == capability)
    }) {
        return Err("node_owned_agent_execution_proof_read_back_missing".to_string());
    }
    Ok(read_back)
}

fn configured_path() -> PathBuf {
    std::env::var_os("CYNIC_AGENT_EXECUTION_RUNTIME_PROOF")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_EXECUTION_RUNTIME_PROOF))
}
