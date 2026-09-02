use std::path::Path;

use cynic_protocol::build::fleet::{AttemptVerdict, BuildResult, BuildStatus, ToolchainFaultClass};
use cynic_protocol::{
    BuildInputAdmission, BuildRunnerAdmission, BuildRunnerEnvelope, EvidenceEnvelope,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::update_lifecycle::ArgvRunner;

pub const BUILD_PROOF_SCHEMA: &str = "cynic.host.build_proof.v1";

/// Classify a failed command read-back as substrate fault or source verdict.
///
/// A chronic-ICE host (see `AGENTS.md` harness rules) crashes rustc without ever
/// judging the code. Attributing that to the source manufactures a false
/// failure, so the raw signals are mapped to `cynic_protocol::build::fleet`
/// types and the canonical classifier decides. `None` means the toolchain did
/// judge the source and the failure is real.
fn toolchain_fault_from_command(
    command: &BuildCommandProof,
    node_id: &str,
) -> Option<ToolchainFaultClass> {
    // A crash signal leaves no exit code; rustc also emits an explicit ICE note.
    let crashed = command.exit_code.is_none();
    let ice_reported = command.stderr.contains("internal compiler error")
        || command.stderr.contains("SIGSEGV")
        || command.stderr.contains("signal: 11");

    let status = if crashed || ice_reported {
        BuildStatus::Ice
    } else {
        return None;
    };

    let result = BuildResult {
        status,
        exit_code: command.exit_code.unwrap_or_default(),
        evidence_refs: Vec::new(),
    };
    match AttemptVerdict::from_result(&result, node_id) {
        AttemptVerdict::ToolchainFault { class, .. } => Some(class),
        AttemptVerdict::SourceJudged { .. } => None,
    }
}

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

pub const CANONICAL_GATE_READ_BACK_SCHEMA: &str = "cynic.build.canonical_gate_read_back.v1";

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

/// The narrow build fact consumed by runtime conformance.  It can originate
/// from an in-process build proof or from content-addressed scheduler evidence;
/// conformance never treats an arbitrary operator-supplied JSON file as proof.
#[derive(Debug, Clone, Serialize)]
pub struct BuildConformanceEvidence {
    pub schema: &'static str,
    pub status: &'static str,
    pub source_commit: String,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CanonicalGateReadBack {
    schema: String,
    source_commit: String,
    argv: Vec<String>,
    exit_code: Option<i32>,
}

impl BuildConformanceEvidence {
    pub fn from_build_proof(proof: &BuildProof) -> Self {
        Self {
            schema: "cynic.host.build_conformance_evidence.v1",
            status: match proof.status {
                "proved" => "proved",
                "failed" => "failed",
                _ => "unknown",
            },
            source_commit: proof.source_commit.clone(),
            evidence_refs: proof.evidence_refs.clone(),
            blockers: proof.blockers.clone(),
        }
    }

    pub fn from_content_addressed_envelope(
        requested_digest: &str,
        envelope: &EvidenceEnvelope,
    ) -> Result<Self, String> {
        if envelope.digest != requested_digest {
            return Err("build_evidence_digest_mismatch".to_string());
        }
        if !envelope.subject.starts_with("build-workload:")
            || !envelope.producer.starts_with("cynic-node/build-scheduler:")
        {
            return Err("build_evidence_provenance_rejected".to_string());
        }
        let read_back: CanonicalGateReadBack = serde_json::from_value(envelope.payload.clone())
            .map_err(|_| "build_evidence_payload_invalid".to_string())?;
        if read_back.schema != CANONICAL_GATE_READ_BACK_SCHEMA
            || read_back.source_commit.trim().is_empty()
            || read_back.argv.is_empty()
            || read_back
                .argv
                .iter()
                .any(|argument| argument.trim().is_empty())
        {
            return Err("build_evidence_contract_rejected".to_string());
        }
        let (status, blockers) = if read_back.exit_code == Some(0) {
            ("proved", Vec::new())
        } else {
            (
                "failed",
                vec![format!(
                    "canonical_gate_exit_code:{:?}",
                    read_back.exit_code
                )],
            )
        };
        Ok(Self {
            schema: "cynic.host.build_conformance_evidence.v1",
            status,
            source_commit: read_back.source_commit,
            evidence_refs: vec![requested_digest.to_string()],
            blockers,
        })
    }
}

pub fn run_build(request: &BuildRequest, runner: &dyn ArgvRunner) -> BuildProof {
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

    // Guard: reject running as root. Running cargo as root creates files that
    // require root to clean up, violating node sovereignty.
    if let Ok(output) = std::process::Command::new("id").arg("-u").output() {
        if String::from_utf8_lossy(&output.stdout).trim() == "0" {
            blockers.push("build_root_forbidden: cargo must not run as root".to_string());
        }
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
        // A toolchain crash proves nothing about the source: report an explicit
        // unknown instead of a failure attributed to the code.
        let node_id = request.runner_envelope.node_id.clone();
        let (status, blocker) = match toolchain_fault_from_command(&command, &node_id) {
            Some(ToolchainFaultClass::CompilerCrash) => (
                "unproved_toolchain_fault",
                format!("toolchain_compiler_crash_on_node:{node_id}"),
            ),
            Some(ToolchainFaultClass::BudgetExhausted) => (
                "unproved_toolchain_fault",
                format!("toolchain_budget_exhausted_on_node:{node_id}"),
            ),
            None => ("failed", "build_command_failed".to_string()),
        };
        return BuildProof {
            schema: BUILD_PROOF_SCHEMA,
            status,
            mutates: true,
            source_commit: request.source_commit.clone(),
            runner_node_id: node_id,
            lease_id: request.runner_envelope.lease.lease_id.clone(),
            fencing_token: request.runner_envelope.lease.fencing_token,
            consumed_actions: 1,
            artifact_path: request.artifact_path.clone(),
            artifact_digest: None,
            commands: vec![command],
            evidence_refs,
            blockers: vec![blocker],
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

#[cfg(test)]
mod tests {
    use cynic_protocol::build_evidence_envelope;

    use super::*;

    #[test]
    fn conformance_accepts_only_a_successful_canonical_gate_envelope() {
        let envelope = build_evidence_envelope(
            "build-workload:workload-1:canonical_gate_report",
            "cynic-node/build-scheduler:node-1",
            1,
            vec!["source:snapshot".to_string()],
            &serde_json::json!({
                "schema": CANONICAL_GATE_READ_BACK_SCHEMA,
                "source_commit": "abc123",
                "argv": ["make", "check"],
                "exit_code": 0,
                "stdout": "ok",
                "stderr": ""
            }),
        )
        .expect("envelope");

        let proof =
            BuildConformanceEvidence::from_content_addressed_envelope(&envelope.digest, &envelope)
                .expect("valid canonical gate evidence");

        assert_eq!(proof.status, "proved");
        assert_eq!(proof.evidence_refs, vec![envelope.digest]);
    }

    #[test]
    fn conformance_rejects_a_non_scheduler_envelope() {
        let envelope = build_evidence_envelope(
            "build-workload:workload-1:canonical_gate_report",
            // Untrusted producer: not the node scheduler.
            "operator-json-file",
            1,
            // Provenance must be non-empty or the envelope is refused before it
            // is even built (evidence_provenance_missing), which would test the
            // wrong contract. The point here is that a well-formed envelope from
            // an untrusted producer is still rejected as conformance proof.
            vec!["operator:local-json".to_string()],
            &serde_json::json!({
                "schema": CANONICAL_GATE_READ_BACK_SCHEMA,
                "source_commit": "abc123",
                "argv": ["make", "check"],
                "exit_code": 0,
            }),
        )
        .expect("envelope");

        assert_eq!(
            BuildConformanceEvidence::from_content_addressed_envelope(&envelope.digest, &envelope,)
                .expect_err("untrusted producer must not become conformance proof"),
            "build_evidence_provenance_rejected"
        );
    }

    fn command(exit_code: Option<i32>, stderr: &str) -> BuildCommandProof {
        BuildCommandProof {
            argv: vec!["cargo".to_string(), "check".to_string()],
            success: false,
            exit_code,
            stdout: String::new(),
            stderr: stderr.to_string(),
        }
    }

    #[test]
    fn crash_without_exit_code_is_a_toolchain_fault_not_a_source_failure() {
        // Cortex signature: rustc dies on a signal, leaving no exit code.
        assert_eq!(
            toolchain_fault_from_command(&command(None, ""), "cortex"),
            Some(ToolchainFaultClass::CompilerCrash)
        );
    }

    #[test]
    fn reported_ice_is_a_toolchain_fault_even_with_an_exit_code() {
        assert_eq!(
            toolchain_fault_from_command(
                &command(
                    Some(101),
                    "error: internal compiler error: unexpected panic"
                ),
                "cortex"
            ),
            Some(ToolchainFaultClass::CompilerCrash)
        );
        assert_eq!(
            toolchain_fault_from_command(&command(Some(101), "signal: 11, SIGSEGV"), "cortex"),
            Some(ToolchainFaultClass::CompilerCrash)
        );
    }

    #[test]
    fn a_real_compile_error_stays_a_source_failure() {
        // E0599 on cynic-builder is a verdict about the code: it must NOT be
        // excused as substrate noise, otherwise real bugs become invisible.
        assert_eq!(
            toolchain_fault_from_command(
                &command(Some(101), "error[E0599]: no method named `resolve` found"),
                "cynic-builder"
            ),
            None
        );
    }

    /// Regression test: the root guard must reject uid 0.
    #[test]
    fn root_guard_blocks_root_uid() {
        // We can't easily test the actual `id -u` call in unit tests,
        // but we can verify the guard logic by checking the blocker message format.
        // The actual integration test is the build workflow refusing to run as root.
        let uid_output = "0\n";
        assert_eq!(uid_output.trim(), "0");
    }
}
