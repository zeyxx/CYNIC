use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};

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
    assert!(
        plan.blockers
            .contains(&"source_snapshot_commit_read_back_mismatch".to_string())
    );
}
