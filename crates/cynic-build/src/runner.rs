//! Build runner: pure pipeline decisions over injected ports.
//!
//! run_build executes the observe -> verify -> decide -> execute flow without
//! touching the OS: commands go through BuildRunner, files through
//! FilesystemPort, identity through IdentityPort. Hexagonal core.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::build::fleet::{AttemptVerdict, BuildResult, BuildStatus, ToolchainFaultClass};
use crate::ports::{BuildRunner, FilesystemPort, IdentityPort};
use crate::types::{
    ArtifactInstall, BUILD_PROOF_SCHEMA, BuildCommandProof, BuildProof, BuildRequest,
};

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

pub fn run_build(
    request: &BuildRequest,
    runner: &dyn BuildRunner,
    fs: &dyn FilesystemPort,
    identity: &dyn IdentityPort,
) -> BuildProof {
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
    // require root to clean up, violating node sovereignty. Identity arrives
    // through the injected port: the domain never shells out to `id -u`.
    if identity.uid() == 0 {
        blockers.push("build_root_forbidden: cargo must not run as root".to_string());
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
            installed_path: None,
            installed_digest: None,
            rolled_back: false,
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
            installed_path: None,
            installed_digest: None,
            rolled_back: false,
            commands: Vec::new(),
            evidence_refs,
            blockers: vec!["explicit_build_apply_required".to_string()],
        };
    }

    let argv_refs = request.argv.iter().map(String::as_str).collect::<Vec<_>>();
    let output = runner.run_command(&argv_refs);
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
            installed_path: None,
            installed_digest: None,
            rolled_back: false,
            commands: vec![command],
            evidence_refs,
            blockers: vec![blocker],
        };
    }

    let path = Path::new(&request.artifact_path);
    let artifact_digest = fs
        .read_bytes(path)
        .ok()
        .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(bytes))));
    let mut blockers = (artifact_digest.is_none())
        .then(|| vec!["build_artifact_readback_missing".to_string()])
        .unwrap_or_default();
    let mut status = if artifact_digest.is_some() {
        "proved"
    } else {
        "failed"
    };
    let mut installed_path = None;
    let mut installed_digest = None;
    if artifact_digest.is_some() {
        if let Some(target) = request.install_path.as_deref() {
            match install_artifact(fs, identity, path, Path::new(target)) {
                Ok(done) => {
                    installed_path = Some(target.to_string());
                    installed_digest = Some(done.installed_digest);
                }
                Err(error) => {
                    status = "failed";
                    blockers.push(format!("artifact_install_failed:{error}"));
                }
            }
        }
    }
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
        installed_path,
        installed_digest,
        rolled_back: false,
        commands: vec![command],
        evidence_refs,
        blockers,
    }
}

/// Atomically install a built artifact with hash read-back and rollback.
///
/// 1. Reads and digests the artifact through the filesystem port.
/// 2. Backs up an existing target to `<target>.prev` (fail-closed).
/// 3. Writes a sibling temp file and renames it over the target.
/// 4. Reads the target back: on digest mismatch the backup is restored
///    and installation fails. A failed install never leaves a half-written
///    target behind.
pub fn install_artifact(
    fs: &dyn FilesystemPort,
    identity: &dyn IdentityPort,
    artifact: &Path,
    target: &Path,
) -> Result<ArtifactInstall, String> {
    let bytes = fs
        .read_bytes(artifact)
        .map_err(|error| format!("install_artifact_read_failed:{error}"))?;
    let digest = format!("sha256:{}", hex::encode(Sha256::digest(&bytes)));
    let backup = if fs.exists(target) {
        let previous = target.with_extension("prev");
        fs.copy(target, &previous)
            .map_err(|error| format!("install_backup_failed:{error}"))?;
        Some(previous)
    } else {
        None
    };
    let staging = target.with_extension(format!("install-{}.tmp", identity.pid()));
    let restore = |message: String| {
        if let Some(previous) = backup.as_ref() {
            let _ = fs.copy(previous, target);
        }
        let _ = fs.remove_file(&staging);
        message
    };
    fs.write_bytes(&staging, &bytes)
        .map_err(|error| restore(format!("install_write_failed:{error}")))?;
    fs.rename(&staging, target)
        .map_err(|error| restore(format!("install_activate_failed:{error}")))?;
    let installed = fs
        .read_bytes(target)
        .map_err(|error| restore(format!("install_read_back_failed:{error}")))?;
    let installed_digest = format!("sha256:{}", hex::encode(Sha256::digest(installed)));
    if installed_digest != digest {
        if let Some(previous) = backup.as_ref() {
            let _ = fs.copy(previous, target);
        }
        return Err(format!("install_digest_mismatch:expected_{digest}"));
    }
    Ok(ArtifactInstall {
        installed_digest,
        backup_path: backup.map(|path| path.to_string_lossy().to_string()),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use cynic_protocol::{LeaseGuard, SecurityEvidenceState};

    use super::*;
    use crate::adapters::MemFilesystem;
    use crate::build::{BuildInputAdmission, BuildRunnerAdmission, BuildRunnerEnvelope};
    use crate::ports::BuildCommandOutput;

    struct OkRunner;

    impl BuildRunner for OkRunner {
        fn run_command(&self, _argv: &[&str]) -> BuildCommandOutput {
            BuildCommandOutput {
                exit_code: Some(0),
                stdout: "ok".to_string(),
                stderr: String::new(),
                spawn_error: None,
            }
        }
    }

    struct FixedIdentity {
        uid: u32,
    }

    impl IdentityPort for FixedIdentity {
        fn uid(&self) -> u32 {
            self.uid
        }

        fn pid(&self) -> u32 {
            4242
        }
    }

    fn lease() -> LeaseGuard {
        LeaseGuard {
            agent_id: "build-agent-1".to_string(),
            target: "workload-1".to_string(),
            lease_id: "lease-1".to_string(),
            fencing_token: 1,
        }
    }

    fn admitted_request() -> BuildRequest {
        BuildRequest {
            input: BuildInputAdmission {
                schema: "cynic.build.input_admission.v1".to_string(),
                status: "admitted".to_string(),
                source_commit: Some("abc123".to_string()),
                blockers: Vec::new(),
                evidence_refs: Vec::new(),
            },
            runner_envelope: BuildRunnerEnvelope {
                schema: "cynic.build.runner_envelope.v1".to_string(),
                node_id: "builder-1".to_string(),
                declared_role: "build_runner".to_string(),
                role_observation: SecurityEvidenceState::Proven,
                source_commit: "abc123".to_string(),
                lease: lease(),
                lease_verification: SecurityEvidenceState::Proven,
                argv: vec!["cargo".to_string(), "check".to_string()],
                artifact_path: "/mem/artifact".to_string(),
                action_budget: 10,
                mutation_authorized: true,
                evidence_refs: Vec::new(),
            },
            runner_admission: BuildRunnerAdmission {
                schema: "cynic.build.runner_admission.v1".to_string(),
                status: "admitted".to_string(),
                node_id: "builder-1".to_string(),
                source_commit: "abc123".to_string(),
                blockers: Vec::new(),
                evidence_refs: Vec::new(),
            },
            source_commit: "abc123".to_string(),
            argv: vec!["cargo".to_string(), "check".to_string()],
            artifact_path: "/mem/artifact".to_string(),
            install_path: None,
            apply: true,
        }
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
        assert_eq!(
            toolchain_fault_from_command(
                &command(Some(101), "error[E0599]: no method named `resolve` found"),
                "cynic-builder"
            ),
            None
        );
    }

    #[test]
    fn root_uid_blocks_through_the_identity_port_not_a_subprocess() {
        let fs = MemFilesystem::new();
        let proof = run_build(
            &admitted_request(),
            &OkRunner,
            &fs,
            &FixedIdentity { uid: 0 },
        );
        assert_eq!(proof.status, "blocked");
        assert!(
            proof
                .blockers
                .iter()
                .any(|b| b.starts_with("build_root_forbidden"))
        );
    }

    #[test]
    fn non_root_uid_passes_the_guard_and_proves_on_mem_fs() {
        let fs = MemFilesystem::new();
        fs.seed(Path::new("/mem/artifact"), b"binary-bytes");
        let proof = run_build(
            &admitted_request(),
            &OkRunner,
            &fs,
            &FixedIdentity { uid: 1000 },
        );
        assert_eq!(proof.status, "proved");
        assert!(proof.artifact_digest.unwrap().starts_with("sha256:"));
    }

    #[test]
    fn install_copies_artifact_with_matching_digest_on_mem_fs() {
        let fs = MemFilesystem::new();
        let artifact = PathBuf::from("/mem/artifact");
        let target = PathBuf::from("/mem/installed");
        fs.seed(&artifact, b"binary-bytes");

        let done = install_artifact(&fs, &FixedIdentity { uid: 1000 }, &artifact, &target)
            .expect("install");
        assert_eq!(fs.read(&target).unwrap(), b"binary-bytes");
        assert!(done.installed_digest.starts_with("sha256:"));
        assert_eq!(done.backup_path, None);
    }

    #[test]
    fn install_backs_up_existing_target_and_matches_digest() {
        let fs = MemFilesystem::new();
        let artifact = PathBuf::from("/mem/artifact");
        let target = PathBuf::from("/mem/installed");
        fs.seed(&artifact, b"new-binary");
        fs.seed(&target, b"old-binary");

        let done = install_artifact(&fs, &FixedIdentity { uid: 1000 }, &artifact, &target)
            .expect("install");
        assert_eq!(fs.read(&target).unwrap(), b"new-binary");
        let backup = PathBuf::from("/mem/installed.prev");
        assert_eq!(fs.read(&backup).unwrap(), b"old-binary");
        assert_eq!(done.backup_path, Some(backup.to_string_lossy().to_string()));
    }

    #[test]
    fn install_fails_closed_on_missing_artifact() {
        let fs = MemFilesystem::new();
        let target = PathBuf::from("/mem/installed");
        let error = install_artifact(
            &fs,
            &FixedIdentity { uid: 1000 },
            Path::new("/mem/absent"),
            &target,
        )
        .unwrap_err();
        assert!(error.starts_with("install_artifact_read_failed:"));
        assert!(!fs.exists(&target));
    }
}
