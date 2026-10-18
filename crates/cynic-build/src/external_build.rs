//! Pure external build governance for non-CYNIC projects.
//!
//! WHY: external projects (aegisscan, aegis-core, ai-bus) must have their
//! build/test commands run on the builder (SSOT) with read-back proof. This
//! module defines the `ExternalBuildPort` trait and pure types that abstract
//! command execution without coupling to SSH, HTTP, or process spawn.
//! HOW: trait + pure data types + pure admission validators. I/O lives in
//! adapters (cynic-node, cynic-host).
//! NEGATIVE BOUNDARY: no std::process::Command, no SSH, no HTTP, no filesystem
//! I/O. No dependency on any external project. Commands come from
//! infra/external-projects.json (observed, not fabricated).

use serde::{Deserialize, Serialize};

use crate::ports::BuildCommandOutput;

// ── Schemas ───────────────────────────────────────────────────────────────────

pub const EXTERNAL_BUILD_SCHEMA: &str = "cynic.build.external.v1";
pub const EXTERNAL_BUILD_PROOF_SCHEMA: &str = "cynic.build.external_proof.v1";

// ── Core types ────────────────────────────────────────────────────────────────

/// Request to build/test an external project on the builder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalBuildRequest {
    /// Stable project slug (e.g. "aegisscan").
    pub project_id: String,
    /// Commands to run (from infra/external-projects.json, observed).
    pub commands: Vec<String>,
    /// Working directory on the builder (clone path).
    pub work_dir: String,
    /// Whether this is a build or a test run.
    pub kind: ExternalBuildKind,
}

/// Build vs test distinction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalBuildKind {
    Build,
    Test,
}

/// Result of one command execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalCommandResult {
    pub command: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub spawn_error: Option<String>,
    pub success: bool,
}

impl ExternalCommandResult {
    pub fn from_output(command: String, output: &BuildCommandOutput) -> Self {
        Self {
            command,
            exit_code: output.exit_code,
            stdout: output.stdout.clone(),
            stderr: output.stderr.clone(),
            spawn_error: output.spawn_error.clone(),
            success: output.success(),
        }
    }
}

/// Proof emitted after building/testing an external project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalBuildProof {
    pub schema: &'static str,
    pub project_id: String,
    pub kind: ExternalBuildKind,
    pub commands_run: usize,
    pub commands_passed: usize,
    pub commands_failed: usize,
    pub results: Vec<ExternalCommandResult>,
    pub overall_success: bool,
    pub builder_node_id: String,
}

// ── Admission (pure) ──────────────────────────────────────────────────────────

/// Admission report for an external build operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalBuildAdmission {
    pub admitted: bool,
    pub blockers: Vec<String>,
}

/// Validate that a project is eligible for external build/test operations.
///
/// Pure: no I/O. Checks that build_commands are non-empty and observed.
pub fn admit_external_build(
    project_id: &str,
    commands: &[String],
    build_status: &str,
    sensitivity: &str,
) -> ExternalBuildAdmission {
    let mut blockers = Vec::new();

    if project_id.is_empty() {
        blockers.push("project_id is empty".to_string());
    }

    if commands.is_empty() {
        blockers.push("no build/test commands declared".to_string());
    }

    if build_status != "observed_on_builder" && build_status != "verified" {
        blockers.push(format!(
            "build_commands_status '{build_status}' is not observed; cannot build unobserved commands"
        ));
    }

    if sensitivity == "sanitized" {
        // sanitized projects: build/test allowed, deploy forbidden
        // (already enforced by mutation_policy, but double-check here)
    }

    ExternalBuildAdmission {
        admitted: blockers.is_empty(),
        blockers,
    }
}

// ── Port trait ────────────────────────────────────────────────────────────────

/// Port for executing build/test commands on external projects.
///
/// Abstracts command execution on the builder. Concrete implementations
/// (SSH, HTTP builder API) live in adapter crates. No I/O in this trait.
pub trait ExternalBuildPort: Send + Sync {
    /// Run a single build/test command and return its output.
    fn run_command(&self, command: &str, work_dir: &str) -> BuildCommandOutput;

    /// Run a full build proof (multiple commands) and return the proof.
    fn run_build_proof(
        &self,
        request: &ExternalBuildRequest,
        builder_node_id: &str,
    ) -> ExternalBuildProof;
}

// ── Pure proof builder ────────────────────────────────────────────────────────

/// Build an external build proof from command results. Pure, no I/O.
pub fn build_external_proof(
    project_id: &str,
    kind: ExternalBuildKind,
    results: Vec<ExternalCommandResult>,
    builder_node_id: &str,
) -> ExternalBuildProof {
    let commands_run = results.len();
    let commands_passed = results.iter().filter(|r| r.success).count();
    let commands_failed = commands_run - commands_passed;
    let overall_success = commands_failed == 0 && commands_run > 0;

    ExternalBuildProof {
        schema: EXTERNAL_BUILD_PROOF_SCHEMA,
        project_id: project_id.to_string(),
        kind,
        commands_run,
        commands_passed,
        commands_failed,
        results,
        overall_success,
        builder_node_id: builder_node_id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admit_with_observed_commands_passes() {
        let r = admit_external_build(
            "aegisscan",
            &["go build ./...".to_string()],
            "observed_on_builder",
            "public",
        );
        assert!(r.admitted);
    }

    #[test]
    fn admit_with_empty_commands_blocked() {
        let r = admit_external_build("aegisscan", &[], "observed_on_builder", "public");
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("no build/test commands")));
    }

    #[test]
    fn admit_with_unobserved_status_blocked() {
        let r = admit_external_build(
            "aegisscan",
            &["go build ./...".to_string()],
            "inferred_from_manifest_presence",
            "public",
        );
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("not observed")));
    }

    #[test]
    fn admit_empty_project_id_blocked() {
        let r = admit_external_build("", &["go build ./...".to_string()], "observed_on_builder", "public");
        assert!(!r.admitted);
    }

    #[test]
    fn build_proof_all_pass() {
        let results = vec![
            ExternalCommandResult {
                command: "go build ./...".to_string(),
                exit_code: Some(0),
                stdout: "".to_string(),
                stderr: "".to_string(),
                spawn_error: None,
                success: true,
            },
        ];
        let proof = build_external_proof("aegisscan", ExternalBuildKind::Build, results, "cynic-builder");
        assert!(proof.overall_success);
        assert_eq!(proof.commands_run, 1);
        assert_eq!(proof.commands_passed, 1);
        assert_eq!(proof.commands_failed, 0);
    }

    #[test]
    fn build_proof_one_fail() {
        let results = vec![
            ExternalCommandResult {
                command: "go build ./...".to_string(),
                exit_code: Some(0),
                stdout: "".to_string(),
                stderr: "".to_string(),
                spawn_error: None,
                success: true,
            },
            ExternalCommandResult {
                command: "go test ./...".to_string(),
                exit_code: Some(1),
                stdout: "".to_string(),
                stderr: "FAIL".to_string(),
                spawn_error: None,
                success: false,
            },
        ];
        let proof = build_external_proof("aegisscan", ExternalBuildKind::Test, results, "cynic-builder");
        assert!(!proof.overall_success);
        assert_eq!(proof.commands_run, 2);
        assert_eq!(proof.commands_passed, 1);
        assert_eq!(proof.commands_failed, 1);
    }

    #[test]
    fn build_proof_empty_commands_not_success() {
        let proof = build_external_proof("aegisscan", ExternalBuildKind::Build, vec![], "cynic-builder");
        assert!(!proof.overall_success);
        assert_eq!(proof.commands_run, 0);
    }

    #[test]
    fn port_trait_object_safe() {
        struct MockPort;
        impl ExternalBuildPort for MockPort {
            fn run_command(&self, _command: &str, _work_dir: &str) -> BuildCommandOutput {
                BuildCommandOutput {
                    exit_code: Some(0),
                    stdout: "".to_string(),
                    stderr: "".to_string(),
                    spawn_error: None,
                }
            }
            fn run_build_proof(
                &self,
                request: &ExternalBuildRequest,
                builder_node_id: &str,
            ) -> ExternalBuildProof {
                build_external_proof(
                    &request.project_id,
                    request.kind,
                    vec![],
                    builder_node_id,
                )
            }
        }
        let port: Box<dyn ExternalBuildPort> = Box::new(MockPort);
        let output = port.run_command("go build ./...", "/tmp/test");
        assert!(output.success());
    }
}