use std::{
    fs,
    path::{Component, Path},
};

use cynic_host::update_lifecycle::ArgvRunner;
use cynic_workload::{
    AGENT_WORKSPACE_OBSERVATION_SCHEMA,
    AgentWorkspaceClaimObservation,
    AgentWorkspaceObservation,
    AgentWorkspacePathObservation,
    AgentWorkspaceRequest,
};
use cynic_protocol::{
    AgentEvidenceLineage,
    AgentEvidenceRef,
    digest_json,
};
use sha2::{Digest, Sha256};

use crate::ports::agent_workspace_port::{
    AgentWorkspaceLayoutPort, AgentWorkspaceObservationPort, AgentWorkspacePortError,
};

#[derive(Debug)]
pub struct LocalAgentWorkspaceObserver<'a, L, R> {
    layout: &'a L,
    runner: &'a R,
    git_bin: &'a str,
    id_bin: &'a str,
    producer_id: &'a str,
    producer_attestation_digest: &'a str,
}

impl<'a, L, R> LocalAgentWorkspaceObserver<'a, L, R> {
    pub fn new(
        layout: &'a L,
        runner: &'a R,
        git_bin: &'a str,
        id_bin: &'a str,
        producer_id: &'a str,
        producer_attestation_digest: &'a str,
    ) -> Self {
        Self {
            layout,
            runner,
            git_bin,
            id_bin,
            producer_id,
            producer_attestation_digest,
        }
    }
}

impl<L, R> AgentWorkspaceObservationPort for LocalAgentWorkspaceObserver<'_, L, R>
where
    L: AgentWorkspaceLayoutPort,
    R: ArgvRunner,
{
    fn observe(
        &self,
        request: &AgentWorkspaceRequest,
        repository_claims: Vec<AgentWorkspaceClaimObservation>,
    ) -> Result<AgentWorkspaceObservation, AgentWorkspacePortError> {
        let layout = self.layout.resolve(
            &request.repository_id,
            &request.workspace_id,
            &request.node_id,
        )?;
        if !layout.workspace_root.exists() {
            return self.finish_observation(
                AgentWorkspaceObservation {
                    schema: AGENT_WORKSPACE_OBSERVATION_SCHEMA.to_string(),
                    repository_id: request.repository_id.clone(),
                    workspace_id: request.workspace_id.clone(),
                    node_id: request.node_id.clone(),
                    exists: false,
                    isolated: false,
                    head_commit: None,
                    path_preimages: Vec::new(),
                    repository_claims,
                    evidence_refs: Vec::new(),
                },
                &layout.evidence_digest,
            );
        }

        let workspace_root = canonical_directory(&layout.workspace_root, "workspace_root")?;
        canonical_directory(&layout.repository_root, "repository_root")?;
        let ownership_evidence = observe_ownership(
            self.runner,
            self.id_bin,
            &workspace_root,
            &layout.required_principal,
            self.producer_id,
            self.producer_attestation_digest,
            &layout.evidence_digest,
        )?;
        let workspace = path_text(&workspace_root, "workspace_root")?;
        let observed_root = run_git(
            self.runner,
            self.git_bin,
            &workspace,
            &["rev-parse", "--show-toplevel"],
        )?;
        let observed_root = canonical_directory(Path::new(observed_root.trim()), "git_toplevel")?;
        let head_commit = run_git(
            self.runner,
            self.git_bin,
            &workspace,
            &["rev-parse", "HEAD"],
        )?;

        let mut path_preimages = Vec::with_capacity(request.changes.len());
        for change in &request.changes {
            path_preimages.push(observe_preimage(&workspace_root, &change.path)?);
        }

        self.finish_observation(
            AgentWorkspaceObservation {
                schema: AGENT_WORKSPACE_OBSERVATION_SCHEMA.to_string(),
                repository_id: request.repository_id.clone(),
                workspace_id: request.workspace_id.clone(),
                node_id: request.node_id.clone(),
                exists: true,
                isolated: layout.dedicated && observed_root == workspace_root,
                head_commit: Some(head_commit.trim().to_string()),
                path_preimages,
                repository_claims,
                evidence_refs: vec![ownership_evidence],
            },
            &layout.evidence_digest,
        )
    }
}

impl<L, R> LocalAgentWorkspaceObserver<'_, L, R> {
    fn finish_observation(
        &self,
        mut observation: AgentWorkspaceObservation,
        layout_evidence_digest: &str,
    ) -> Result<AgentWorkspaceObservation, AgentWorkspacePortError> {
        let digest = digest_json(&observation).map_err(|error| {
            AgentWorkspacePortError::new(
                "workspace_observation_digest_failed",
                format!("workspace observation could not be canonicalized: {error}"),
            )
        })?;
        observation.evidence_refs.push(AgentEvidenceRef {
            kind: "workspace_observation".to_string(),
            subject: format!("{}:{}", observation.repository_id, observation.workspace_id),
            digest,
            lineage: Some(AgentEvidenceLineage {
                producer_id: self.producer_id.to_string(),
                producer_attestation_digest: self.producer_attestation_digest.to_string(),
                origin_digests: vec![layout_evidence_digest.to_string()],
                derivation_digests: Vec::new(),
                influence_digests: Vec::new(),
            }),
        });
        Ok(observation)
    }
}

fn observe_preimage(
    workspace_root: &Path,
    relative_path: &str,
) -> Result<AgentWorkspacePathObservation, AgentWorkspacePortError> {
    let components = Path::new(relative_path)
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value),
            _ => Err(AgentWorkspacePortError::new(
                "workspace_preimage_path_rejected",
                format!("source change path '{relative_path}' is not repository-relative"),
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if components.is_empty() {
        return Err(AgentWorkspacePortError::new(
            "workspace_preimage_path_rejected",
            "source change path must not be empty",
        ));
    }
    let mut cursor = workspace_root.to_path_buf();
    for component in &components[..components.len() - 1] {
        cursor.push(component);
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(AgentWorkspacePortError::new(
                    "workspace_preimage_symlink_rejected",
                    format!("source change path '{relative_path}' crosses a symlink"),
                ));
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(AgentWorkspacePortError::new(
                    "workspace_preimage_parent_not_directory",
                    format!("source change path '{relative_path}' has a non-directory parent"),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(AgentWorkspacePortError::new(
                    "workspace_preimage_metadata_failed",
                    format!("cannot inspect parent of '{relative_path}': {error}"),
                ));
            }
        }
    }
    let path = workspace_root.join(relative_path);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AgentWorkspacePathObservation {
                path: relative_path.to_string(),
                digest: None,
            });
        }
        Err(error) => {
            return Err(AgentWorkspacePortError::new(
                "workspace_preimage_metadata_failed",
                format!("cannot inspect '{relative_path}': {error}"),
            ));
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(AgentWorkspacePortError::new(
            "workspace_preimage_symlink_rejected",
            format!("source change path '{relative_path}' resolves through a symlink"),
        ));
    }
    if !metadata.is_file() {
        return Err(AgentWorkspacePortError::new(
            "workspace_preimage_not_file",
            format!("source change path '{relative_path}' is not a regular file"),
        ));
    }
    let content = fs::read(&path).map_err(|error| {
        AgentWorkspacePortError::new(
            "workspace_preimage_read_failed",
            format!("cannot read '{relative_path}': {error}"),
        )
    })?;
    Ok(AgentWorkspacePathObservation {
        path: relative_path.to_string(),
        digest: Some(format!("sha256:{}", hex::encode(Sha256::digest(content)))),
    })
}

fn run_git<R: ArgvRunner>(
    runner: &R,
    git_bin: &str,
    workspace: &str,
    args: &[&str],
) -> Result<String, AgentWorkspacePortError> {
    let mut argv = vec![git_bin, "-C", workspace];
    argv.extend_from_slice(args);
    let output = runner.run(&argv);
    if output.success() {
        Ok(output.stdout)
    } else {
        Err(AgentWorkspacePortError::new(
            "workspace_git_observation_failed",
            format!(
                "read-only git observation failed: {}",
                output
                    .spawn_error
                    .as_deref()
                    .unwrap_or(output.stderr.trim())
            ),
        ))
    }
}

#[cfg(unix)]
fn observe_ownership<R: ArgvRunner>(
    runner: &R,
    id_bin: &str,
    workspace_root: &Path,
    required_principal: &str,
    producer_id: &str,
    producer_attestation_digest: &str,
    layout_evidence_digest: &str,
) -> Result<AgentEvidenceRef, AgentWorkspacePortError> {
    use std::os::unix::fs::MetadataExt;

    let principal = runner.run(&[id_bin, "-u", required_principal]);
    if !principal.success() {
        return Err(AgentWorkspacePortError::new(
            "workspace_principal_unobservable",
            format!("principal '{required_principal}' could not be observed"),
        ));
    }
    let expected_uid = principal.stdout.trim().parse::<u32>().map_err(|_| {
        AgentWorkspacePortError::new(
            "workspace_principal_unobservable",
            format!("principal '{required_principal}' has no observable numeric uid"),
        )
    })?;
    let execution = runner.run(&[id_bin, "-u"]);
    if !execution.success() {
        return Err(AgentWorkspacePortError::new(
            "workspace_execution_principal_unobservable",
            "the executing principal uid could not be observed",
        ));
    }
    let execution_uid = execution.stdout.trim().parse::<u32>().map_err(|_| {
        AgentWorkspacePortError::new(
            "workspace_execution_principal_unobservable",
            "the executing principal has no observable numeric uid",
        )
    })?;
    let owner_uid = fs::symlink_metadata(workspace_root)
        .map_err(|error| {
            AgentWorkspacePortError::new(
                "workspace_ownership_unobservable",
                format!("workspace ownership cannot be observed: {error}"),
            )
        })?
        .uid();
    if owner_uid != expected_uid || execution_uid != expected_uid {
        return Err(AgentWorkspacePortError::new(
            "workspace_ownership_mismatch",
            format!(
                "workspace owner uid {owner_uid} and execution uid {execution_uid} must both match principal '{required_principal}' uid {expected_uid}"
            ),
        ));
    }
    let digest = digest_json(&(
        workspace_root,
        required_principal,
        expected_uid,
        owner_uid,
        execution_uid,
    ))
    .map_err(|error| {
        AgentWorkspacePortError::new(
            "workspace_ownership_digest_failed",
            format!("workspace ownership could not be canonicalized: {error}"),
        )
    })?;
    Ok(AgentEvidenceRef {
        kind: "workspace_ownership".to_string(),
        subject: required_principal.to_string(),
        digest,
        lineage: Some(AgentEvidenceLineage {
            producer_id: producer_id.to_string(),
            producer_attestation_digest: producer_attestation_digest.to_string(),
            origin_digests: vec![layout_evidence_digest.to_string()],
            derivation_digests: Vec::new(),
            influence_digests: Vec::new(),
        }),
    })
}

#[cfg(not(unix))]
fn observe_ownership<R: ArgvRunner>(
    _runner: &R,
    _id_bin: &str,
    _workspace_root: &Path,
    _required_principal: &str,
    _producer_id: &str,
    _producer_attestation_digest: &str,
    _layout_evidence_digest: &str,
) -> Result<AgentEvidenceRef, AgentWorkspacePortError> {
    Err(AgentWorkspacePortError::new(
        "workspace_ownership_adapter_unavailable",
        "the local workspace ownership adapter currently requires Unix uid observation",
    ))
}

fn canonical_directory(
    path: &Path,
    field: &str,
) -> Result<std::path::PathBuf, AgentWorkspacePortError> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        AgentWorkspacePortError::new(
            "workspace_path_unobservable",
            format!("{field} cannot be canonicalized: {error}"),
        )
    })?;
    if !canonical.is_dir() {
        return Err(AgentWorkspacePortError::new(
            "workspace_path_not_directory",
            format!("{field} is not a directory"),
        ));
    }
    Ok(canonical)
}

fn path_text<'a>(path: &'a Path, field: &str) -> Result<&'a str, AgentWorkspacePortError> {
    path.to_str().ok_or_else(|| {
        AgentWorkspacePortError::new(
            "workspace_path_not_utf8",
            format!("{field} cannot be represented as UTF-8 argv"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preimage_observer_hashes_regular_files() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("src")).unwrap();
        fs::write(root.path().join("src/lib.rs"), b"bounded source\n").unwrap();

        let observed = observe_preimage(root.path(), "src/lib.rs").unwrap();

        assert_eq!(observed.path, "src/lib.rs");
        assert_eq!(
            observed.digest,
            Some(format!(
                "sha256:{}",
                hex::encode(Sha256::digest(b"bounded source\n"))
            ))
        );
    }

    #[test]
    fn preimage_observer_rejects_parent_traversal() {
        let root = tempfile::tempdir().unwrap();
        let error = observe_preimage(root.path(), "../outside").unwrap_err();
        assert_eq!(error.code, "workspace_preimage_path_rejected");
    }

    #[cfg(unix)]
    #[test]
    fn preimage_observer_rejects_symlinked_parent() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret"), b"secret").unwrap();
        symlink(outside.path(), root.path().join("linked")).unwrap();

        let error = observe_preimage(root.path(), "linked/secret").unwrap_err();
        assert_eq!(error.code, "workspace_preimage_symlink_rejected");
    }
}
