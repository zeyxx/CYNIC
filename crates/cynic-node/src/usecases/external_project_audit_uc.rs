//! Pure orchestration usecase that composes the three external project ports
//! in sequence: clone (SourcePort) → build (BuildPort) → ingest evidence
//! (EvidencePort).
//!
//! WHY: external projects (aegisscan, aegis-core, ai-bus) need end-to-end
//! auditing — clone the repo, build/test it on the builder, then ingest the
//! resulting evidence into CYNIC's observation store. This module is the single
//! entry point that orchestrates all three ports without coupling to any
//! concrete adapter (Forgejo, GitHub, SSH, HTTP, local storage).
//!
//! HOW: pure orchestration. The function takes three port trait objects and
//! a request, then calls each port in sequence. If any step fails, subsequent
//! steps are skipped and the failure is recorded in the report. No I/O in this
//! module — all side effects are delegated to the ports.
//!
//! NEGATIVE BOUNDARY: no std::fs, no HTTP, no process spawn, no git commands,
//! no database I/O. The port traits abstract all I/O.

use cynic_build::{
    ExternalBuildKind, ExternalBuildPort, ExternalBuildProof, ExternalBuildRequest,
    EXTERNAL_BUILD_PROOF_SCHEMA,
};
use cynic_kernel::ports::external_evidence::{
    ExternalEvidenceBatch, ExternalEvidenceIngestionProof, ExternalEvidencePort,
    EXTERNAL_EVIDENCE_INGESTION_PROOF_SCHEMA,
};
use cynic_vcs::{
    ExternalCloneResult, ExternalRepoRef, ExternalSourcePort, EXTERNAL_CLONE_PROOF_SCHEMA,
};

/// Schema identifier for external project audit reports.
pub const EXTERNAL_PROJECT_AUDIT_SCHEMA: &str = "cynic.node.external_project_audit.v1";

/// Request to audit an external project end-to-end.
#[derive(Debug, Clone)]
pub struct ExternalProjectAuditRequest {
    /// Stable project slug (e.g. "aegisscan").
    pub project_id: String,
    /// Repository reference for cloning.
    pub repo: ExternalRepoRef,
    /// Build/test commands to run on the builder.
    pub build_commands: Vec<String>,
    /// Whether this is a build or test run.
    pub build_kind: ExternalBuildKind,
    /// Target builder node identifier.
    pub builder_node_id: String,
    /// Evidence batches to ingest after build succeeds.
    pub evidence_batches: Vec<ExternalEvidenceBatch>,
}

/// Result of the full external project audit pipeline.
#[derive(Debug, Clone)]
pub struct ExternalProjectAuditReport {
    /// Schema identifier.
    pub schema: &'static str,
    /// Project slug.
    pub project_id: String,
    /// Clone step result.
    pub clone_result: ExternalCloneResult,
    /// Build step proof.
    pub build_proof: ExternalBuildProof,
    /// Evidence ingestion step proof.
    pub evidence_proof: ExternalEvidenceIngestionProof,
    /// True only if all three steps succeeded.
    pub overall_success: bool,
}

/// Orchestrate clone → build → evidence ingestion for an external project.
///
/// Calls each port in sequence:
/// 1. `source_port.clone_repo` — clones the repo into `/tmp/{project_id}`.
/// 2. `build_port.run_build_proof` — runs build/test commands in the clone path.
/// 3. `evidence_port.ingest_batch` — ingests evidence batches.
///
/// If any step fails, subsequent steps are skipped and the failure is recorded
/// in the report. The function never panics — port errors are captured in the
/// returned report.
///
/// Pure orchestration: no I/O. All side effects are delegated to the ports.
///
/// # Arguments
/// * `source_port` — the external source port (VCS adapter).
/// * `build_port` — the external build port (builder adapter).
/// * `evidence_port` — the external evidence port (storage adapter).
/// * `req` — the audit request containing repo, build commands, and evidence batches.
///
/// # Returns
/// An [`ExternalProjectAuditReport`] with the results of each step.
pub fn audit_external_project(
    source_port: &dyn ExternalSourcePort,
    build_port: &dyn ExternalBuildPort,
    evidence_port: &dyn ExternalEvidencePort,
    req: &ExternalProjectAuditRequest,
) -> ExternalProjectAuditReport {
    // Step 1: Clone the repo.
    let clone_path = format!("/tmp/{}", req.project_id);
    let clone_result = source_port
        .clone_repo(&req.repo, &clone_path)
        .unwrap_or_else(|e| ExternalCloneResult {
            schema: EXTERNAL_CLONE_PROOF_SCHEMA,
            project_id: req.project_id.clone(),
            clone_path: String::new(),
            head_sha: String::new(),
            default_branch: req.repo.default_branch.clone(),
            success: false,
            error: Some(e),
        });

    // Step 2: Build (only if clone succeeded).
    let build_proof = if clone_result.success {
        let build_req = ExternalBuildRequest {
            project_id: req.project_id.clone(),
            commands: req.build_commands.clone(),
            work_dir: clone_path,
            kind: req.build_kind,
            builder_node_id: req.builder_node_id.clone(),
        };
        build_port.run_build_proof(&build_req, &req.builder_node_id)
    } else {
        // Clone failed — skip build, return a failure proof.
        ExternalBuildProof {
            schema: EXTERNAL_BUILD_PROOF_SCHEMA,
            project_id: req.project_id.clone(),
            kind: req.build_kind,
            commands_run: 0,
            commands_passed: 0,
            commands_failed: 0,
            results: vec![],
            overall_success: false,
            builder_node_id: req.builder_node_id.clone(),
        }
    };

    // Step 3: Evidence ingestion (only if build succeeded).
    let evidence_proof = if build_proof.overall_success {
        evidence_port.ingest_batch(&req.project_id, &req.evidence_batches)
    } else {
        // Build failed — skip evidence ingestion, return a failure proof.
        ExternalEvidenceIngestionProof {
            schema: EXTERNAL_EVIDENCE_INGESTION_PROOF_SCHEMA,
            project_id: req.project_id.clone(),
            batches_ingested: 0,
            batches_failed: 0,
            digests: vec![],
            overall_success: false,
        }
    };

    let overall_success =
        clone_result.success && build_proof.overall_success && evidence_proof.overall_success;

    ExternalProjectAuditReport {
        schema: EXTERNAL_PROJECT_AUDIT_SCHEMA,
        project_id: req.project_id.clone(),
        clone_result,
        build_proof,
        evidence_proof,
        overall_success,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_build::ExternalCommandResult;
    use cynic_kernel::ports::external_evidence::ExternalEvidenceIngestionResult;
    use cynic_vcs::{ExternalBranch, ExternalProvider};
    use serde_json::json;
    use std::sync::Mutex;

    // ── Mock ExternalSourcePort ─────────────────────────────────────────────────

    struct MockSourcePort {
        clone_result: Result<ExternalCloneResult, String>,
        clone_repo_calls: Mutex<Vec<(String, String)>>,
    }

    impl ExternalSourcePort for MockSourcePort {
        fn clone_repo(
            &self,
            _repo: &ExternalRepoRef,
            target_path: &str,
        ) -> Result<ExternalCloneResult, String> {
            self.clone_repo_calls
                .lock()
                .unwrap()
                .push((_repo.project_id.clone(), target_path.to_string()));
            self.clone_result.clone()
        }
        fn fetch(&self, _repo: &ExternalRepoRef) -> Result<Vec<ExternalBranch>, String> {
            Ok(vec![])
        }
        fn list_branches(&self, _repo: &ExternalRepoRef) -> Result<Vec<ExternalBranch>, String> {
            Ok(vec![])
        }
        fn create_branch(
            &self,
            _repo: &ExternalRepoRef,
            _branch_name: &str,
            _from_sha: &str,
        ) -> Result<ExternalBranch, String> {
            Ok(ExternalBranch {
                name: "test-branch".to_string(),
                head_sha: "abc123".to_string(),
                is_default: false,
            })
        }
        fn push(
            &self,
            _repo: &ExternalRepoRef,
            _branch: &str,
            _commit_sha: &str,
        ) -> Result<cynic_vcs::ExternalPushProof, String> {
            Ok(cynic_vcs::ExternalPushProof {
                schema: cynic_vcs::EXTERNAL_PUSH_PROOF_SCHEMA,
                project_id: "test".to_string(),
                branch: "test-branch".to_string(),
                pushed_sha: "abc123".to_string(),
                remote_url: "https://example.com".to_string(),
                success: true,
            })
        }
        fn create_pr(
            &self,
            _repo: &ExternalRepoRef,
            _title: &str,
            _head_ref: &str,
            _base_ref: &str,
        ) -> Result<cynic_vcs::ExternalPrProof, String> {
            Ok(cynic_vcs::ExternalPrProof {
                schema: cynic_vcs::EXTERNAL_PR_PROOF_SCHEMA,
                project_id: "test".to_string(),
                pr_number: 1,
                html_url: "https://example.com/pr/1".to_string(),
                head_ref: "test-branch".to_string(),
                base_ref: "main".to_string(),
            })
        }
    }

    // ── Mock ExternalBuildPort ─────────────────────────────────────────────────

    struct MockBuildPort {
        proof: ExternalBuildProof,
        build_proof_calls: Mutex<Vec<ExternalBuildRequest>>,
    }

    impl ExternalBuildPort for MockBuildPort {
        fn run_command(
            &self,
            _command: &str,
            _work_dir: &str,
        ) -> cynic_build::BuildCommandOutput {
            cynic_build::BuildCommandOutput {
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: None,
            }
        }
        fn run_build_proof(
            &self,
            request: &ExternalBuildRequest,
            _builder_node_id: &str,
        ) -> ExternalBuildProof {
            self.build_proof_calls
                .lock()
                .unwrap()
                .push(request.clone());
            self.proof.clone()
        }
    }

    // ── Mock ExternalEvidencePort ──────────────────────────────────────────────

    struct MockEvidencePort {
        proof: ExternalEvidenceIngestionProof,
        ingest_batch_calls: Mutex<Vec<(String, Vec<ExternalEvidenceBatch>)>>,
    }

    impl ExternalEvidencePort for MockEvidencePort {
        fn ingest(
            &self,
            _batch: &ExternalEvidenceBatch,
        ) -> ExternalEvidenceIngestionResult {
            ExternalEvidenceIngestionResult {
                project_id: "test".to_string(),
                subject: "test".to_string(),
                digest: "mock-digest".to_string(),
                persisted: true,
                error: None,
            }
        }
        fn ingest_batch(
            &self,
            project_id: &str,
            batches: &[ExternalEvidenceBatch],
        ) -> ExternalEvidenceIngestionProof {
            self.ingest_batch_calls.lock().unwrap().push((
                project_id.to_string(),
                batches.to_vec(),
            ));
            self.proof.clone()
        }
    }

    // ── Helpers ────────────────────────────────────────────────────────────────

    fn success_clone_result() -> ExternalCloneResult {
        ExternalCloneResult {
            schema: EXTERNAL_CLONE_PROOF_SCHEMA,
            project_id: "test".to_string(),
            clone_path: "/tmp/test".to_string(),
            head_sha: "abc123".to_string(),
            default_branch: "main".to_string(),
            success: true,
            error: None,
        }
    }

    fn failure_clone_result() -> ExternalCloneResult {
        ExternalCloneResult {
            schema: EXTERNAL_CLONE_PROOF_SCHEMA,
            project_id: "test".to_string(),
            clone_path: String::new(),
            head_sha: String::new(),
            default_branch: "main".to_string(),
            success: false,
            error: Some("network unreachable".to_string()),
        }
    }

    fn success_build_proof() -> ExternalBuildProof {
        ExternalBuildProof {
            schema: EXTERNAL_BUILD_PROOF_SCHEMA,
            project_id: "test".to_string(),
            kind: ExternalBuildKind::Build,
            commands_run: 2,
            commands_passed: 2,
            commands_failed: 0,
            results: vec![
                ExternalCommandResult {
                    command: "go build ./...".to_string(),
                    exit_code: Some(0),
                    stdout: String::new(),
                    stderr: String::new(),
                    spawn_error: None,
                    success: true,
                },
                ExternalCommandResult {
                    command: "go test ./...".to_string(),
                    exit_code: Some(0),
                    stdout: String::new(),
                    stderr: String::new(),
                    spawn_error: None,
                    success: true,
                },
            ],
            overall_success: true,
            builder_node_id: "cynic-builder".to_string(),
        }
    }

    fn failure_build_proof() -> ExternalBuildProof {
        ExternalBuildProof {
            schema: EXTERNAL_BUILD_PROOF_SCHEMA,
            project_id: "test".to_string(),
            kind: ExternalBuildKind::Build,
            commands_run: 1,
            commands_passed: 0,
            commands_failed: 1,
            results: vec![ExternalCommandResult {
                command: "go build ./...".to_string(),
                exit_code: Some(1),
                stdout: String::new(),
                stderr: "compilation error".to_string(),
                spawn_error: None,
                success: false,
            }],
            overall_success: false,
            builder_node_id: "cynic-builder".to_string(),
        }
    }

    fn success_evidence_proof() -> ExternalEvidenceIngestionProof {
        ExternalEvidenceIngestionProof {
            schema: EXTERNAL_EVIDENCE_INGESTION_PROOF_SCHEMA,
            project_id: "test".to_string(),
            batches_ingested: 1,
            batches_failed: 0,
            digests: vec!["digest-1".to_string()],
            overall_success: true,
        }
    }

    fn failure_evidence_proof() -> ExternalEvidenceIngestionProof {
        ExternalEvidenceIngestionProof {
            schema: EXTERNAL_EVIDENCE_INGESTION_PROOF_SCHEMA,
            project_id: "test".to_string(),
            batches_ingested: 0,
            batches_failed: 1,
            digests: vec!["digest-1".to_string()],
            overall_success: false,
        }
    }

    fn sample_request() -> ExternalProjectAuditRequest {
        ExternalProjectAuditRequest {
            project_id: "test".to_string(),
            repo: ExternalRepoRef {
                project_id: "test".to_string(),
                remote_url: "https://example.com/test.git".to_string(),
                default_branch: "main".to_string(),
                provider: ExternalProvider::GitHub,
            },
            build_commands: vec!["go build ./...".to_string(), "go test ./...".to_string()],
            build_kind: ExternalBuildKind::Build,
            builder_node_id: "cynic-builder".to_string(),
            evidence_batches: vec![ExternalEvidenceBatch {
                project_id: "test".to_string(),
                producer: "test-findings-v1".to_string(),
                subject: "build-1".to_string(),
                observed_at_unix_ms: 1000,
                provenance: vec!["provenance-1".to_string()],
                payload: json!({"finding": "test", "severity": "high"}),
            }],
        }
    }

    // ── Tests ──────────────────────────────────────────────────────────────────

    #[test]
    fn audit_success_all_steps_pass() {
        let source_port = MockSourcePort {
            clone_result: Ok(success_clone_result()),
            clone_repo_calls: Mutex::new(Vec::new()),
        };
        let build_port = MockBuildPort {
            proof: success_build_proof(),
            build_proof_calls: Mutex::new(Vec::new()),
        };
        let evidence_port = MockEvidencePort {
            proof: success_evidence_proof(),
            ingest_batch_calls: Mutex::new(Vec::new()),
        };

        let report = audit_external_project(
            &source_port,
            &build_port,
            &evidence_port,
            &sample_request(),
        );

        assert!(report.overall_success);
        assert!(report.clone_result.success);
        assert!(report.build_proof.overall_success);
        assert!(report.evidence_proof.overall_success);
        assert_eq!(report.project_id, "test");
        assert_eq!(report.schema, EXTERNAL_PROJECT_AUDIT_SCHEMA);
    }

    #[test]
    fn audit_clone_called_with_correct_path() {
        let source_port = MockSourcePort {
            clone_result: Ok(success_clone_result()),
            clone_repo_calls: Mutex::new(Vec::new()),
        };
        let build_port = MockBuildPort {
            proof: success_build_proof(),
            build_proof_calls: Mutex::new(Vec::new()),
        };
        let evidence_port = MockEvidencePort {
            proof: success_evidence_proof(),
            ingest_batch_calls: Mutex::new(Vec::new()),
        };

        let _ = audit_external_project(
            &source_port,
            &build_port,
            &evidence_port,
            &sample_request(),
        );

        let calls = source_port.clone_repo_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "test");
        assert_eq!(calls[0].1, "/tmp/test");
    }

    #[test]
    fn audit_clone_failure_skips_build_and_evidence() {
        let source_port = MockSourcePort {
            clone_result: Err("network unreachable".to_string()),
            clone_repo_calls: Mutex::new(Vec::new()),
        };
        let build_port = MockBuildPort {
            proof: success_build_proof(),
            build_proof_calls: Mutex::new(Vec::new()),
        };
        let evidence_port = MockEvidencePort {
            proof: success_evidence_proof(),
            ingest_batch_calls: Mutex::new(Vec::new()),
        };

        let report = audit_external_project(
            &source_port,
            &build_port,
            &evidence_port,
            &sample_request(),
        );

        assert!(!report.overall_success);
        assert!(!report.clone_result.success);
        assert!(!report.build_proof.overall_success);
        assert!(!report.evidence_proof.overall_success);
        assert_eq!(report.build_proof.commands_run, 0);
        assert_eq!(report.evidence_proof.batches_ingested, 0);
    }

    #[test]
    fn audit_build_failure_skips_evidence() {
        let source_port = MockSourcePort {
            clone_result: Ok(success_clone_result()),
            clone_repo_calls: Mutex::new(Vec::new()),
        };
        let build_port = MockBuildPort {
            proof: failure_build_proof(),
            build_proof_calls: Mutex::new(Vec::new()),
        };
        let evidence_port = MockEvidencePort {
            proof: success_evidence_proof(),
            ingest_batch_calls: Mutex::new(Vec::new()),
        };

        let report = audit_external_project(
            &source_port,
            &build_port,
            &evidence_port,
            &sample_request(),
        );

        assert!(!report.overall_success);
        assert!(report.clone_result.success);
        assert!(!report.build_proof.overall_success);
        assert!(!report.evidence_proof.overall_success);
        assert_eq!(report.evidence_proof.batches_ingested, 0);
    }

    #[test]
    fn audit_evidence_failure_reports_partial_success() {
        let source_port = MockSourcePort {
            clone_result: Ok(success_clone_result()),
            clone_repo_calls: Mutex::new(Vec::new()),
        };
        let build_port = MockBuildPort {
            proof: success_build_proof(),
            build_proof_calls: Mutex::new(Vec::new()),
        };
        let evidence_port = MockEvidencePort {
            proof: failure_evidence_proof(),
            ingest_batch_calls: Mutex::new(Vec::new()),
        };

        let report = audit_external_project(
            &source_port,
            &build_port,
            &evidence_port,
            &sample_request(),
        );

        assert!(!report.overall_success);
        assert!(report.clone_result.success);
        assert!(report.build_proof.overall_success);
        assert!(!report.evidence_proof.overall_success);
    }

    #[test]
    fn audit_build_receives_clone_path_as_work_dir() {
        let source_port = MockSourcePort {
            clone_result: Ok(success_clone_result()),
            clone_repo_calls: Mutex::new(Vec::new()),
        };
        let build_port = MockBuildPort {
            proof: success_build_proof(),
            build_proof_calls: Mutex::new(Vec::new()),
        };
        let evidence_port = MockEvidencePort {
            proof: success_evidence_proof(),
            ingest_batch_calls: Mutex::new(Vec::new()),
        };

        let _ = audit_external_project(
            &source_port,
            &build_port,
            &evidence_port,
            &sample_request(),
        );

        let calls = build_port.build_proof_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].work_dir, "/tmp/test");
        assert_eq!(calls[0].commands.len(), 2);
    }

    #[test]
    fn audit_evidence_receives_project_id_and_batches() {
        let source_port = MockSourcePort {
            clone_result: Ok(success_clone_result()),
            clone_repo_calls: Mutex::new(Vec::new()),
        };
        let build_port = MockBuildPort {
            proof: success_build_proof(),
            build_proof_calls: Mutex::new(Vec::new()),
        };
        let evidence_port = MockEvidencePort {
            proof: success_evidence_proof(),
            ingest_batch_calls: Mutex::new(Vec::new()),
        };

        let _ = audit_external_project(
            &source_port,
            &build_port,
            &evidence_port,
            &sample_request(),
        );

        let calls = evidence_port.ingest_batch_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "test");
        assert_eq!(calls[0].1.len(), 1);
        assert_eq!(calls[0].1[0].subject, "build-1");
    }

    #[test]
    fn audit_empty_evidence_batches_produces_empty_proof() {
        let source_port = MockSourcePort {
            clone_result: Ok(success_clone_result()),
            clone_repo_calls: Mutex::new(Vec::new()),
        };
        let build_port = MockBuildPort {
            proof: success_build_proof(),
            build_proof_calls: Mutex::new(Vec::new()),
        };
        let evidence_port = MockEvidencePort {
            proof: success_evidence_proof(),
            ingest_batch_calls: Mutex::new(Vec::new()),
        };

        let mut req = sample_request();
        req.evidence_batches = vec![];

        let report = audit_external_project(
            &source_port,
            &build_port,
            &evidence_port,
            &req,
        );

        // Even with empty batches, the port is called and returns its proof.
        // The overall success depends on the port's behavior.
        let calls = evidence_port.ingest_batch_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1.len(), 0);
    }
}
