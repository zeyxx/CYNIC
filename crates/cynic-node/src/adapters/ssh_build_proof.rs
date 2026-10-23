//! SSH build proof adapter — implements `BuildProofPort` over the unprivileged
//! builder transport.
//!
//! This is the *only* place that knows SSH details for proof. The port stays
//! transport-agnostic; swap this adapter for a local-process or mTLS one without
//! touching the surface.

use async_trait::async_trait;
use std::process::Command;
use std::time::Instant;

use crate::ports::build_proof_port::BuildProofPort;
use cynic_build::build_proof::{BuildProofReport, BuildProofRequest};

use cynic_protocol::remote_cache_roots::{parse_state_root, remote_cache_roots};

pub struct SshBuildProof {
    pub host: String,
    pub user: String,
    pub node_id: String,
}

impl SshBuildProof {
    pub fn new(
        host: impl Into<String>,
        user: impl Into<String>,
        node_id: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            user: user.into(),
            node_id: node_id.into(),
        }
    }

    fn ssh_target(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }

    /// Resolve the runner's own state root through its native command.
    ///
    /// The proof adapter runs on one host and builds on another. Resolving
    /// the checkout path on the client produced a path the runner may not
    /// own, and left the proof build without a warm target dir at all.
    fn resolve_runner_roots(
        &self,
    ) -> Result<cynic_protocol::remote_cache_roots::RemoteCacheRoots, String> {
        let (ok, stdout, stderr) = self.run_remote(&[
            "cynic".to_string(),
            "runtime".to_string(),
            "paths".to_string(),
            "--json".to_string(),
        ])?;
        if !ok {
            return Err(format!(
                "runner paths unresolvable: {}",
                stderr.trim().lines().last().unwrap_or("no output")
            ));
        }
        let state_root = parse_state_root(&stdout)
            .ok_or_else(|| "runner paths payload carries no state root".to_string())?;
        Ok(remote_cache_roots(&state_root))
    }

    /// Scope the runner target dir to the proof checkout.
    ///
    /// The checkout `dir` is `{checkout_base}/cynic-{sha12}`; only its final
    /// component names the scope. Falls back to the unscoped profile root
    /// rather than failing the proof on an unparseable name.
    fn scoped_target_dir(
        &self,
        roots: &cynic_protocol::remote_cache_roots::RemoteCacheRoots,
        checkout_dir: &str,
    ) -> String {
        let name = checkout_dir
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("");
        cynic_protocol::remote_cache_roots::scoped_target_dir(&roots.target_dir, name)
            .unwrap_or_else(|| roots.target_dir.clone())
    }

    fn run_remote(&self, argv: &[String]) -> Result<(bool, String, String), String> {
        let mut cmd = Command::new("ssh");
        cmd.arg(self.ssh_target());
        for arg in argv {
            cmd.arg(arg);
        }
        let output = cmd
            .output()
            .map_err(|e| format!("spawn ssh {}: {e}", self.ssh_target()))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Ok((output.status.success(), stdout, stderr))
    }
}

#[async_trait]
impl BuildProofPort for SshBuildProof {
    async fn prove(&self, request: &BuildProofRequest) -> Result<BuildProofReport, String> {
        let start = Instant::now();
        let sha = &request.source_commit;
        let roots = self.resolve_runner_roots()?;
        let dir = format!(
            "{}/cynic-{}",
            roots.checkout_base,
            &sha[..12.min(sha.len())]
        );
        let package = request.package.as_deref();

        let (check_args, test_args) = match &request.phase {
            cynic_build::build_proof::BuildProofPhase::Check => {
                let mut a: Vec<String> = vec![
                    "env".to_string(),
                    "RUST_MIN_STACK=8589934592".to_string(),
                    "CARGO_BUILD_JOBS=1".to_string(),
                    "CARGO_INCREMENTAL=0".to_string(),
                    format!("CARGO_TARGET_DIR={}", self.scoped_target_dir(&roots, &dir)),
                    "cargo".to_string(),
                    "check".to_string(),
                    "--locked".to_string(),
                    "--offline".to_string(),
                    "--manifest-path".to_string(),
                    format!("{dir}/Cargo.toml"),
                ];
                if let Some(pkg) = package {
                    a.push("--package".to_string());
                    a.push(pkg.to_string());
                }
                a.push("--lib".to_string());
                (a, None)
            }
            cynic_build::build_proof::BuildProofPhase::Test { filter } => {
                // No `--lib`: the proof runs the whole suite (lib, integration
                // targets, doc tests). A `--lib`-only proof silently skips
                // `tests/` targets and blesses code the suite never executed.
                let mut a: Vec<String> = vec![
                    "env".to_string(),
                    "RUST_MIN_STACK=8589934592".to_string(),
                    "CARGO_BUILD_JOBS=1".to_string(),
                    "CARGO_INCREMENTAL=0".to_string(),
                    format!("CARGO_TARGET_DIR={}", self.scoped_target_dir(&roots, &dir)),
                    "cargo".to_string(),
                    "test".to_string(),
                    "--locked".to_string(),
                    "--offline".to_string(),
                    "--manifest-path".to_string(),
                    format!("{dir}/Cargo.toml"),
                ];
                if let Some(pkg) = package {
                    a.push("--package".to_string());
                    a.push(pkg.to_string());
                }
                if let Some(f) = filter {
                    a.push(f.to_string());
                }
                (a.clone(), Some(a))
            }
        };

        let (ok, stdout, stderr) = self.run_remote(&check_args)?;

        if !ok {
            return Ok(BuildProofReport {
                schema: cynic_build::build_proof::BUILD_PROOF_REPORT_SCHEMA.to_string(),
                source_commit: sha.clone(),
                package: request.package.clone(),
                phase: "check".to_string(),
                status: cynic_build::build_proof::BuildProofStatus::Failed {
                    detail: format!("stdout: {stdout}\nstderr: {stderr}"),
                },
                argv: check_args.iter().map(|s| (*s).to_string()).collect(),
                node_id: self.node_id.clone(),
                elapsed_secs: start.elapsed().as_secs(),
                tests_passed: None,
                tests_failed: None,
            });
        }

        let mut tests_passed: Option<u64> = None;
        let mut tests_failed: Option<u64> = None;
        if let Some(test_args) = test_args {
            let (ok, stdout, stderr) = self.run_remote(&test_args)?;
            if !ok {
                return Ok(BuildProofReport {
                    schema: cynic_build::build_proof::BUILD_PROOF_REPORT_SCHEMA.to_string(),
                    source_commit: sha.clone(),
                    package: request.package.clone(),
                    phase: "test".to_string(),
                    status: cynic_build::build_proof::BuildProofStatus::Failed {
                        detail: format!("stdout: {stdout}\nstderr: {stderr}"),
                    },
                    argv: test_args.iter().map(|s| (*s).to_string()).collect(),
                    node_id: self.node_id.clone(),
                    elapsed_secs: start.elapsed().as_secs(),
                    tests_passed: None,
                    tests_failed: None,
                });
            }
            let (passed, failed) = cynic_build::build_proof::parse_cargo_test_counts(&stdout);
            // Fail closed on a vacuous filter match: cargo exits 0 with
            // "0 passed" when the filter names no test. A proof of nothing
            // must not read as converged.
            let filter_is_vacuous = matches!(
                &request.phase,
                cynic_build::build_proof::BuildProofPhase::Test { filter: Some(_) }
            ) && passed == Some(0)
                && failed == Some(0);
            if filter_is_vacuous {
                return Ok(BuildProofReport {
                    schema: cynic_build::build_proof::BUILD_PROOF_REPORT_SCHEMA.to_string(),
                    source_commit: sha.clone(),
                    package: request.package.clone(),
                    phase: "test".to_string(),
                    status: cynic_build::build_proof::BuildProofStatus::Failed {
                        detail: "proof filter matched no tests; vacuous pass refused".to_string(),
                    },
                    argv: test_args.iter().map(|s| (*s).to_string()).collect(),
                    node_id: self.node_id.clone(),
                    elapsed_secs: start.elapsed().as_secs(),
                    tests_passed: None,
                    tests_failed: None,
                });
            }
            tests_passed = passed;
            tests_failed = failed;
        }

        Ok(BuildProofReport {
            schema: cynic_build::build_proof::BUILD_PROOF_REPORT_SCHEMA.to_string(),
            source_commit: sha.clone(),
            package: request.package.clone(),
            phase: match &request.phase {
                cynic_build::build_proof::BuildProofPhase::Check => "check".to_string(),
                cynic_build::build_proof::BuildProofPhase::Test { .. } => "test".to_string(),
            },
            status: cynic_build::build_proof::BuildProofStatus::Converged,
            argv: check_args.iter().map(|s| (*s).to_string()).collect(),
            node_id: self.node_id.clone(),
            elapsed_secs: start.elapsed().as_secs(),
            tests_passed,
            tests_failed,
        })
    }

    fn node_id(&self) -> &str {
        &self.node_id
    }
}
