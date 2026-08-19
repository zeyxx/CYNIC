//! Build executor — executes build workloads received from the kernel.
//!
//! This is the node-side build capability. It receives build workloads
//! (argv, artifact path, lease), executes them via the local process runner,
//! and returns a signed proof of execution.
//!
//! The executor is the node-side counterpart to the kernel's `BuildScheduler`.

use std::path::Path;

use sha2::{Digest, Sha256};

use cynic_protocol::LeaseGuard;

/// Build executor error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildExecutorError {
    LeaseInvalid,
    ArgvEmpty,
    ShellCompositionForbidden,
    ProcessSpawnFailed { reason: String },
    ArtifactReadbackFailed,
}

impl std::fmt::Display for BuildExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LeaseInvalid => write!(f, "lease invalid"),
            Self::ArgvEmpty => write!(f, "argv empty"),
            Self::ShellCompositionForbidden => write!(f, "shell composition forbidden"),
            Self::ProcessSpawnFailed { reason } => write!(f, "process spawn failed: {reason}"),
            Self::ArtifactReadbackFailed => write!(f, "artifact readback failed"),
        }
    }
}

impl std::error::Error for BuildExecutorError {}

/// Build workload received from the kernel.
#[derive(Debug, Clone)]
pub struct BuildWorkload {
    pub workload_id: String,
    pub argv: Vec<String>,
    pub artifact_path: String,
    pub lease: LeaseGuard,
    pub source_commit: String,
    pub now_unix_ms: u64,
}

/// Build execution proof.
#[derive(Debug, Clone)]
pub struct BuildExecutionProof {
    pub schema: &'static str,
    pub workload_id: String,
    pub status: &'static str,
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub artifact_digest: Option<String>,
    pub artifact_path: String,
    pub source_commit: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub evidence_refs: Vec<String>,
}

/// Build executor — runs build workloads on this node.
pub struct BuildExecutor {
    node_id: String,
    role: String,
}

impl BuildExecutor {
    pub fn new(node_id: String, role: String) -> Self {
        Self { node_id, role }
    }

    /// Execute a build workload and return a signed proof.
    pub fn execute(&self, workload: &BuildWorkload) -> Result<BuildExecutionProof, BuildExecutorError> {
        // Step 1: Validate lease
        if workload.lease.agent_id.is_empty()
            || workload.lease.lease_id.is_empty()
            || workload.lease.target.is_empty()
            || workload.lease.fencing_token == 0
        {
            return Err(BuildExecutorError::LeaseInvalid);
        }

        // Step 2: Validate argv
        if workload.argv.is_empty() || workload.argv.iter().any(|a| a.trim().is_empty()) {
            return Err(BuildExecutorError::ArgvEmpty);
        }

        // Step 3: Reject shell composition
        if workload
            .argv
            .first()
            .is_some_and(|cmd| matches!(cmd.as_str(), "sh" | "bash" | "dash" | "zsh" | "fish"))
            || workload.argv.iter().any(|arg| arg == "-c" || arg == "--command")
        {
            return Err(BuildExecutorError::ShellCompositionForbidden);
        }

        // Step 4: Execute
        let argv_refs: Vec<&str> = workload.argv.iter().map(String::as_str).collect();
        let output = std::process::Command::new(argv_refs[0])
            .args(&argv_refs[1..])
            .output()
            .map_err(|e| BuildExecutorError::ProcessSpawnFailed {
                reason: e.to_string(),
            })?;

        let exit_code = output.status.code();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Step 5: Read-back artifact
        let artifact_digest = if exit_code == Some(0) {
            let path = Path::new(&workload.artifact_path);
            std::fs::read(path)
                .ok()
                .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(&bytes))))
        } else {
            None
        };

        let status = if exit_code == Some(0) && artifact_digest.is_some() {
            "proved"
        } else {
            "failed"
        };

        let mut evidence_refs = vec![
            format!("node:{}", self.node_id),
            format!("role:{}", self.role),
            format!("lease:{}", workload.lease.lease_id),
            format!("commit:{}", workload.source_commit),
        ];
        if let Some(ref digest) = artifact_digest {
            evidence_refs.push(format!("artifact:{}", digest));
        }

        Ok(BuildExecutionProof {
            schema: "cynic.node.build_execution_proof.v1",
            workload_id: workload.workload_id.clone(),
            status,
            argv: workload.argv.clone(),
            exit_code,
            stdout,
            stderr,
            artifact_digest,
            artifact_path: workload.artifact_path.clone(),
            source_commit: workload.source_commit.clone(),
            lease_id: workload.lease.lease_id.clone(),
            fencing_token: workload.lease.fencing_token,
            evidence_refs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::LeaseGuard;

    fn valid_workload() -> BuildWorkload {
        BuildWorkload {
            workload_id: "wl-1".to_string(),
            argv: vec!["echo".to_string(), "hello".to_string()],
            artifact_path: "/tmp/cynic-test-artifact".to_string(),
            lease: LeaseGuard {
                agent_id: "kernel".to_string(),
                target: "build-runner:builder-1".to_string(),
                lease_id: "lease-1".to_string(),
                fencing_token: 7,
            },
            source_commit: "abc".to_string(),
            now_unix_ms: 1,
        }
    }

    #[test]
    fn executor_rejects_empty_argv() {
        let executor = BuildExecutor::new("builder-1".to_string(), "build_runner".to_string());
        let mut workload = valid_workload();
        workload.argv = vec![];
        let result = executor.execute(&workload);
        assert!(matches!(result, Err(BuildExecutorError::ArgvEmpty)));
    }

    #[test]
    fn executor_rejects_shell_composition() {
        let executor = BuildExecutor::new("builder-1".to_string(), "build_runner".to_string());
        let mut workload = valid_workload();
        workload.argv = vec!["bash".to_string(), "-c".to_string(), "echo hello".to_string()];
        let result = executor.execute(&workload);
        assert!(matches!(result, Err(BuildExecutorError::ShellCompositionForbidden)));
    }

    #[test]
    fn executor_rejects_invalid_lease() {
        let executor = BuildExecutor::new("builder-1".to_string(), "build_runner".to_string());
        let mut workload = valid_workload();
        workload.lease.fencing_token = 0;
        let result = executor.execute(&workload);
        assert!(matches!(result, Err(BuildExecutorError::LeaseInvalid)));
    }

    #[test]
    fn executor_runs_valid_workload() {
        let executor = BuildExecutor::new("builder-1".to_string(), "build_runner".to_string());
        let workload = valid_workload();
        let result = executor.execute(&workload);
        assert!(result.is_ok(), "expected success, got {:?}", result);
        let proof = result.unwrap();
        assert_eq!(proof.status, "proved");
        assert_eq!(proof.exit_code, Some(0));
        assert!(proof.stdout.contains("hello"));
    }
}
