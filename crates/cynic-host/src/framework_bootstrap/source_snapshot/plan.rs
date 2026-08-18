use std::path::Path;

use crate::update_lifecycle::ArgvRunner;

use super::types::SourceSnapshotPlan;

pub fn plan_source_snapshot(
    repository_root: &str,
    source_commit: &str,
    output_dir: &str,
    runner: &dyn ArgvRunner,
) -> SourceSnapshotPlan {
    let mut blockers = Vec::new();
    let repository = Path::new(repository_root);
    let output = Path::new(output_dir);
    if !safe_absolute_path(repository) {
        blockers.push("source_snapshot_repository_path_unsafe".to_string());
    }
    if !safe_absolute_path(output) {
        blockers.push("source_snapshot_output_path_unsafe".to_string());
    }
    let repository_available = matches!(
        std::fs::symlink_metadata(repository),
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink()
    );
    if !repository_available {
        blockers.push("source_snapshot_repository_unavailable".to_string());
    }
    if !valid_commit(source_commit) {
        blockers.push("source_snapshot_commit_invalid".to_string());
    }
    let rev_parse = vec![
        "git".to_string(),
        "-C".to_string(),
        repository_root.to_string(),
        "rev-parse".to_string(),
        format!("{source_commit}^{{commit}}"),
    ];
    if blockers.is_empty() {
        let observed = runner.run(&rev_parse.iter().map(String::as_str).collect::<Vec<_>>());
        if !observed.success() || observed.stdout.trim() != source_commit {
            blockers.push("source_snapshot_commit_read_back_mismatch".to_string());
        }
        let status = runner.run(&[
            "git",
            "-C",
            repository_root,
            "status",
            "--porcelain",
            "--untracked-files=all",
        ]);
        if !status.success() {
            blockers.push("source_snapshot_worktree_unobservable".to_string());
        } else if !status.stdout.trim().is_empty() {
            blockers.push("source_snapshot_worktree_mutable".to_string());
        }
    }
    blockers.sort();
    blockers.dedup();
    SourceSnapshotPlan {
        schema: "cynic.source_snapshot.plan.v1",
        status: if blockers.is_empty() {
            "planned"
        } else {
            "blocked"
        },
        mutates: false,
        repository_root: repository_root.to_string(),
        source_commit: source_commit.to_string(),
        output_dir: output_dir.to_string(),
        format: "git_bundle_v1",
        planned_argv: if blockers.is_empty() {
            vec![
                rev_parse,
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    repository_root.to_string(),
                    "update-ref".to_string(),
                    "<temporary-snapshot-ref>".to_string(),
                    source_commit.to_string(),
                    "<zero-object-id>".to_string(),
                ],
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    repository_root.to_string(),
                    "bundle".to_string(),
                    "create".to_string(),
                    "<atomic-staging-path>".to_string(),
                    "<temporary-snapshot-ref>".to_string(),
                ],
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    repository_root.to_string(),
                    "update-ref".to_string(),
                    "-d".to_string(),
                    "<temporary-snapshot-ref>".to_string(),
                    source_commit.to_string(),
                ],
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    repository_root.to_string(),
                    "show-ref".to_string(),
                    "--verify".to_string(),
                    "--quiet".to_string(),
                    "<temporary-snapshot-ref>".to_string(),
                ],
                vec![
                    "git".to_string(),
                    "bundle".to_string(),
                    "verify".to_string(),
                    "<content-addressed-path>".to_string(),
                ],
                vec![
                    "git".to_string(),
                    "bundle".to_string(),
                    "list-heads".to_string(),
                    "<content-addressed-path>".to_string(),
                ],
            ]
        } else {
            Vec::new()
        },
        blockers,
        stop_condition: "stop before publication on commit, digest, bundle verification, or list-heads read-back mismatch",
        rollback_intent: "delete the exact temporary Git ref, remove only the uncommitted staging bundle, and remove any newly-created empty output directory",
    }
}

fn valid_commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn safe_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
}
