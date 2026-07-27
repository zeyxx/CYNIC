use std::path::Path;

use cynic_protocol::{BuildInputAdmission, BuildRunnerAdmission, BuildRunnerEnvelope};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::update_lifecycle::ArgvRunner;

pub const BUILD_PROOF_SCHEMA: &str = "cynic.host.build_proof.v1";

#[derive(Debug, Clone)]
pub struct BuildRequest {
    pub input: BuildInputAdmission,
    pub runner_envelope: BuildRunnerEnvelope,
    pub runner_admission: BuildRunnerAdmission,
    pub source_commit: String,
    pub argv: Vec<String>,
    pub artifact_path: String,
    pub apply: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildCommandProof {
    pub argv: Vec<String>,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildProof {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub source_commit: String,
    pub runner_node_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub consumed_actions: u64,
    pub artifact_path: String,
    pub artifact_digest: Option<String>,
    pub commands: Vec<BuildCommandProof>,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
}

pub fn run_build(request: &BuildRequest, runner: &impl ArgvRunner) -> BuildProof {
    let evidence_refs = request
        .input
        .evidence_refs
        .iter()
        .chain(request.runner_admission.evidence_refs.iter())
        .cloned()
        .collect::<Vec<_>>();
    let mut blockers = request.input.blockers.clone();
    if request.input.status != "admitted" {
        blockers.push("build_input_not_admitted".to_string());
    }
    if request.input.source_commit.as_deref() != Some(request.source_commit.as_str()) {
        blockers.push("build_source_commit_mismatch".to_string());
    }
    blockers.extend(request.runner_admission.blockers.clone());
    if request.runner_admission.status != "admitted" {
        blockers.push("build_runner_not_admitted".to_string());
    }
    if request.runner_admission.node_id != request.runner_envelope.node_id {
        blockers.push("build_runner_node_mismatch".to_string());
    }
    if request.runner_envelope.source_commit != request.source_commit
        || request.runner_admission.source_commit != request.source_commit
    {
        blockers.push("build_runner_source_commit_mismatch".to_string());
    }
    if request.runner_envelope.argv != request.argv {
        blockers.push("build_runner_argv_mismatch".to_string());
    }
    if request.runner_envelope.artifact_path != request.artifact_path {
        blockers.push("build_runner_artifact_path_mismatch".to_string());
    }
    if request.argv.is_empty() || request.argv.iter().any(|arg| arg.trim().is_empty()) {
        blockers.push("build_argv_incomplete".to_string());
    }
    if request
        .argv
        .first()
        .is_some_and(|command| matches!(command.as_str(), "sh" | "bash" | "dash" | "zsh" | "fish"))
        || request
            .argv
            .iter()
            .any(|arg| arg == "-c" || arg == "--command")
    {
        blockers.push("shell_composition_forbidden".to_string());
    }
    if request.artifact_path.trim().is_empty() {
        blockers.push("build_artifact_path_missing".to_string());
    }
    if !blockers.is_empty() {
        return BuildProof {
            schema: BUILD_PROOF_SCHEMA,
            status: "blocked",
            mutates: false,
            source_commit: request.source_commit.clone(),
            runner_node_id: request.runner_envelope.node_id.clone(),
            lease_id: request.runner_envelope.lease.lease_id.clone(),
            fencing_token: request.runner_envelope.lease.fencing_token,
            consumed_actions: 0,
            artifact_path: request.artifact_path.clone(),
            artifact_digest: None,
            commands: Vec::new(),
            evidence_refs,
            blockers,
        };
    }
    if !request.apply {
        return BuildProof {
            schema: BUILD_PROOF_SCHEMA,
            status: "planned",
            mutates: true,
            source_commit: request.source_commit.clone(),
            runner_node_id: request.runner_envelope.node_id.clone(),
            lease_id: request.runner_envelope.lease.lease_id.clone(),
            fencing_token: request.runner_envelope.lease.fencing_token,
            consumed_actions: 0,
            artifact_path: request.artifact_path.clone(),
            artifact_digest: None,
            commands: Vec::new(),
            evidence_refs,
            blockers: vec!["explicit_build_apply_required".to_string()],
        };
    }

    let argv_refs = request.argv.iter().map(String::as_str).collect::<Vec<_>>();
    let output = runner.run(&argv_refs);
    let command = BuildCommandProof {
        argv: request.argv.clone(),
        success: output.success(),
        exit_code: output.exit_code,
        stdout: output.stdout,
        stderr: output.spawn_error.unwrap_or(output.stderr),
    };
    if !command.success {
        return BuildProof {
            schema: BUILD_PROOF_SCHEMA,
            status: "failed",
            mutates: true,
            source_commit: request.source_commit.clone(),
            runner_node_id: request.runner_envelope.node_id.clone(),
            lease_id: request.runner_envelope.lease.lease_id.clone(),
            fencing_token: request.runner_envelope.lease.fencing_token,
            consumed_actions: 1,
            artifact_path: request.artifact_path.clone(),
            artifact_digest: None,
            commands: vec![command],
            evidence_refs,
            blockers: vec!["build_command_failed".to_string()],
        };
    }

    let path = Path::new(&request.artifact_path);
    let artifact_digest = std::fs::read(path)
        .ok()
        .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(bytes))));
    let status = if artifact_digest.is_some() {
        "proved"
    } else {
        "failed"
    };
    let blockers = (artifact_digest.is_none())
        .then(|| vec!["build_artifact_readback_missing".to_string()])
        .unwrap_or_default();
    BuildProof {
        schema: BUILD_PROOF_SCHEMA,
        status,
        mutates: true,
        source_commit: request.source_commit.clone(),
        runner_node_id: request.runner_envelope.node_id.clone(),
        lease_id: request.runner_envelope.lease.lease_id.clone(),
        fencing_token: request.runner_envelope.lease.fencing_token,
        consumed_actions: 1,
        artifact_path: request.artifact_path.clone(),
        artifact_digest,
        commands: vec![command],
        evidence_refs,
        blockers,
    }
}
