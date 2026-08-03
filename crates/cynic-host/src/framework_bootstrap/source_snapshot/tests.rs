use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};

use super::apply::prove_snapshot_import;
use super::plan_source_snapshot;

#[test]
fn plan_requires_exact_commit_read_back() {
    let repository = tempfile::tempdir().expect("repository");
    let output = tempfile::tempdir().expect("output");
    let commit = "a".repeat(40);
    let runner = MapArgvRunner {
        responses: vec![(
            vec![
                "git".to_string(),
                "-C".to_string(),
                repository.path().display().to_string(),
                "rev-parse".to_string(),
                format!("{commit}^{{commit}}"),
            ],
            ArgvOutput {
                exit_code: Some(0),
                stdout: format!("{}\n", "b".repeat(40)),
                stderr: String::new(),
                spawn_error: None,
            },
        )],
    };

    let plan = plan_source_snapshot(
        &repository.path().display().to_string(),
        &commit,
        &output.path().display().to_string(),
        &runner,
    );

    assert_eq!(plan.status, "blocked");
    assert!(plan
        .blockers
        .contains(&"source_snapshot_commit_read_back_mismatch".to_string()));
}

#[test]
fn consumer_import_failure_is_blocking_and_probe_is_removed() {
    let output = tempfile::tempdir().expect("output");
    let bundle = output.path().join("snapshot.bundle");
    let commit = "a".repeat(40);
    let probe = output.path().join(format!(
        ".cynic-source-snapshot-import-{}",
        std::process::id()
    ));
    let runner = MapArgvRunner {
        responses: vec![
            (
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    probe.display().to_string(),
                    "init".to_string(),
                    "--bare".to_string(),
                ],
                success_output(String::new()),
            ),
            (
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    probe.display().to_string(),
                    "fetch".to_string(),
                    "--no-tags".to_string(),
                    bundle.display().to_string(),
                    format!("{commit}:refs/cynic/source-snapshot-import-proof"),
                ],
                ArgvOutput {
                    exit_code: Some(1),
                    stdout: String::new(),
                    stderr: "index-pack failed".to_string(),
                    spawn_error: None,
                },
            ),
        ],
    };
    let mut executed = Vec::new();

    let result = prove_snapshot_import(&bundle, &commit, output.path(), &runner, &mut executed);

    assert_eq!(
        result,
        Err("source_snapshot_consumer_import_failed".to_string())
    );
    assert!(!probe.exists());
    assert_eq!(executed.len(), 2);
}

fn success_output(stdout: String) -> ArgvOutput {
    ArgvOutput {
        exit_code: Some(0),
        stdout,
        stderr: String::new(),
        spawn_error: None,
    }
}
