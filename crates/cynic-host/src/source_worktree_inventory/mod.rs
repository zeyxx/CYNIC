use std::path::{Path, PathBuf};

use cynic_protocol::{
    SourceWorktreeInventoryObservation, SourceWorktreeInventoryObservationUnknown,
};

const MAX_PORCELAIN_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceWorktreeInventoryCommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub spawn_error: Option<String>,
}

pub trait SourceWorktreeInventoryRunner: Send + Sync {
    fn run(&self, argv: &[&str]) -> SourceWorktreeInventoryCommandOutput;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalSourceWorktreeInventoryRunner;

impl SourceWorktreeInventoryRunner for LocalSourceWorktreeInventoryRunner {
    fn run(&self, argv: &[&str]) -> SourceWorktreeInventoryCommandOutput {
        if argv.is_empty() {
            return SourceWorktreeInventoryCommandOutput {
                exit_code: None,
                stdout: Vec::new(),
                stderr: Vec::new(),
                spawn_error: Some("empty argv".to_string()),
            };
        }
        match std::process::Command::new(argv[0])
            .args(&argv[1..])
            .output()
        {
            Ok(output) => SourceWorktreeInventoryCommandOutput {
                exit_code: output.status.code(),
                stdout: output.stdout,
                stderr: output.stderr,
                spawn_error: None,
            },
            Err(error) => SourceWorktreeInventoryCommandOutput {
                exit_code: None,
                stdout: Vec::new(),
                stderr: Vec::new(),
                spawn_error: Some(format!("spawn {}: {error}", argv[0])),
            },
        }
    }
}

#[cfg(test)]
impl SourceWorktreeInventoryRunner for crate::update_lifecycle::MapArgvRunner {
    fn run(&self, argv: &[&str]) -> SourceWorktreeInventoryCommandOutput {
        let output = crate::update_lifecycle::ArgvRunner::run(self, argv);
        SourceWorktreeInventoryCommandOutput {
            exit_code: output.exit_code,
            stdout: output.stdout.into_bytes(),
            stderr: output.stderr.into_bytes(),
            spawn_error: output.spawn_error,
        }
    }
}

pub fn observe_source_worktree_inventory(
    repository_root: &Path,
    workspace_store: &Path,
    repository_id: &str,
    node_id: &str,
    runner: &impl SourceWorktreeInventoryRunner,
) -> SourceWorktreeInventoryObservation {
    let root = repository_root.to_string_lossy().to_string();
    let argv = ["git", "-C", &root, "worktree", "list", "--porcelain", "-z"];
    let output = runner.run(&argv);
    let observed_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    if output.exit_code != Some(0) || output.spawn_error.is_some() {
        return unknown_observation(
            repository_id,
            node_id,
            observed_at_ms,
            SourceWorktreeInventoryObservationUnknown::GitCommandFailed,
        );
    }

    if output.stdout.len() > MAX_PORCELAIN_BYTES {
        return unknown_observation(
            repository_id,
            node_id,
            observed_at_ms,
            SourceWorktreeInventoryObservationUnknown::OutputLimitExceeded,
        );
    }

    let text = match String::from_utf8(output.stdout) {
        Ok(text) => text,
        Err(error) => {
            return malformed_observation(
                repository_id,
                node_id,
                observed_at_ms,
                cynic_protocol::digest::digest_bytes(error.as_bytes()),
            );
        }
    };

    let repository_root = observed_absolute_path(repository_root);
    let workspace_store = observed_absolute_path(workspace_store);
    let (entries, entry_limit_exceeded, malformed_record_evidence) =
        parse_porcelain(&text, &repository_root, &workspace_store);
    let mut evidence_refs: Vec<String> =
        entries.iter().map(|e| e.evidence_digest.clone()).collect();
    evidence_refs.extend(malformed_record_evidence.iter().cloned());
    let mut observation_unknowns = Vec::new();
    if entry_limit_exceeded {
        observation_unknowns.push(SourceWorktreeInventoryObservationUnknown::EntryLimitExceeded);
    }
    if !malformed_record_evidence.is_empty() {
        observation_unknowns.push(SourceWorktreeInventoryObservationUnknown::MalformedRecord);
    }
    if entries.is_empty() && observation_unknowns.is_empty() {
        observation_unknowns.push(SourceWorktreeInventoryObservationUnknown::EmptyOutput);
    }

    SourceWorktreeInventoryObservation {
        schema: cynic_protocol::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
        repository_id: repository_id.to_string(),
        node_id: node_id.to_string(),
        observed_at_ms,
        entries,
        evidence_refs,
        observation_unknowns,
    }
}

fn unknown_observation(
    repository_id: &str,
    node_id: &str,
    observed_at_ms: u64,
    unknown: SourceWorktreeInventoryObservationUnknown,
) -> SourceWorktreeInventoryObservation {
    SourceWorktreeInventoryObservation {
        schema: cynic_protocol::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
        repository_id: repository_id.to_string(),
        node_id: node_id.to_string(),
        observed_at_ms,
        entries: Vec::new(),
        evidence_refs: Vec::new(),
        observation_unknowns: vec![unknown],
    }
}

fn malformed_observation(
    repository_id: &str,
    node_id: &str,
    observed_at_ms: u64,
    evidence_digest: String,
) -> SourceWorktreeInventoryObservation {
    SourceWorktreeInventoryObservation {
        schema: cynic_protocol::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
        repository_id: repository_id.to_string(),
        node_id: node_id.to_string(),
        observed_at_ms,
        entries: Vec::new(),
        evidence_refs: vec![evidence_digest],
        observation_unknowns: vec![SourceWorktreeInventoryObservationUnknown::MalformedRecord],
    }
}

fn observed_absolute_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

mod entry;
mod parse;
pub(crate) use parse::parse_porcelain;
