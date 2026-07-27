use super::apply::apply_framework_propagation;
use super::plan::{propagation_operations, valid_seed_id};
use super::types::FrameworkPropagationPlan;
use crate::update_lifecycle::{ArgvOutput, ArgvRunner};

struct PanicRunner;

impl ArgvRunner for PanicRunner {
    fn run(&self, _argv: &[&str]) -> ArgvOutput {
        panic!("runner must not execute without explicit consent")
    }
}

#[test]
fn propagation_is_strict_bounded_and_finishes_with_read_back() {
    let operations = propagation_operations(
        "/tmp/local-seed",
        "builder",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );

    assert_eq!(
        operations.last().expect("operation").phase,
        "evidence_read_back"
    );
    assert!(operations.iter().all(|operation| {
        !operation
            .argv
            .first()
            .is_some_and(|program| program == "ssh")
            || operation
                .argv
                .windows(2)
                .any(|pair| pair[0] == "StrictHostKeyChecking=yes" && pair[1] == "builder")
    }));
    assert!(operations.iter().filter(|operation| operation.mutates).all(
        |operation| operation.phase == "prepare"
            || operation.phase == "transfer"
            || operation.phase == "install"
    ));
    let transfer = operations
        .iter()
        .find(|operation| operation.phase == "transfer")
        .expect("transfer");
    assert!(
        transfer
            .argv
            .iter()
            .any(|argument| argument == "/tmp/local-seed/.")
    );
}

#[test]
fn seed_identity_is_safe_for_a_versioned_remote_path() {
    assert!(valid_seed_id(
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    ));
    assert!(!valid_seed_id("sha256:../../current"));
}

#[test]
fn propagation_does_not_execute_without_yes() {
    let plan = FrameworkPropagationPlan {
        schema: "cynic.framework_bootstrap.propagation_plan.v1",
        status: "planned",
        mutates: false,
        node_id: "builder".to_string(),
        role: "build_runner".to_string(),
        ssh_target: "builder".to_string(),
        trust: "known_hosts".to_string(),
        bundle_dir: "/tmp/seed".to_string(),
        seed_id: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_string(),
        blockers: Vec::new(),
        operations: propagation_operations(
            "/tmp/seed",
            "builder",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        stop_condition: "stop",
        rollback_intent: "remove seed",
    };

    let report = apply_framework_propagation(&plan, false, &PanicRunner);

    assert_eq!(report.status, "blocked");
    assert!(!report.mutates);
    assert!(
        report
            .blockers
            .contains(&"explicit_yes_required".to_string())
    );
}
