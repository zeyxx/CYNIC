use super::apply::apply_framework_propagation;
use super::decision::decide_framework_propagation;
use super::plan::{enrollment_operations, valid_seed_id};
use super::types::{
    FrameworkPropagationDecision, FrameworkPropagationObservation, FrameworkPropagationPlan,
};
use crate::update_lifecycle::{ArgvOutput, ArgvRunner, MapArgvRunner};

const SEED_ID: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const BASKET_DIGEST: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct PanicRunner;

impl ArgvRunner for PanicRunner {
    fn run(&self, _argv: &[&str]) -> ArgvOutput {
        panic!("runner must not execute without explicit consent")
    }
}

fn empty_observation() -> FrameworkPropagationObservation {
    FrameworkPropagationObservation {
        schema: "cynic.framework_bootstrap.propagation_observation.v1",
        status: "observed",
        ssh_authenticated: true,
        remote_principal_status: "observed",
        remote_principal: Some("root".to_string()),
        runtime_marker_status: "absent",
        runtime_marker: None,
        node_observation_status: "absent",
        node_observation: None,
        probes: Vec::new(),
        unknowns: Vec::new(),
    }
}

fn enrollment_plan() -> FrameworkPropagationPlan {
    FrameworkPropagationPlan {
        schema: "cynic.framework_bootstrap.propagation_plan.v2",
        status: "planned_enrollment",
        mutates: false,
        node_id: "builder".to_string(),
        role: "build_runner".to_string(),
        ssh_target: "builder".to_string(),
        trust: "known_hosts".to_string(),
        enrollment_authority_class: "machine_enrollment".to_string(),
        enrollment_principal: "root".to_string(),
        enrollment_credential_ref: "operator-ssh-agent".to_string(),
        bundle_dir: "/tmp/seed".to_string(),
        seed_id: SEED_ID.to_string(),
        expected_runtime_basket_digest: BASKET_DIGEST.to_string(),
        observation: empty_observation(),
        decision: FrameworkPropagationDecision {
            schema: "cynic.framework_bootstrap.propagation_decision.v1",
            status: "enroll_with_machine_authority",
            next_action: "stage_and_enroll_content_addressed_seed",
            authority_class: Some("MachineEnrollment"),
            blockers: Vec::new(),
        },
        blockers: Vec::new(),
        operations: enrollment_operations("/tmp/seed", "builder", SEED_ID, BASKET_DIGEST),
        stop_condition: "stop",
        rollback_intent: "remove seed",
    }
}

#[test]
fn propagation_uses_only_the_declared_machine_enrollment_entrypoint_for_privilege() {
    let operations = enrollment_operations("/tmp/local-seed", "builder", SEED_ID, BASKET_DIGEST);

    assert_eq!(
        operations.last().expect("operation").phase,
        "node_observation_read_back"
    );
    assert!(
        operations
            .iter()
            .filter(|operation| operation.requires_privilege)
            .all(|operation| operation.phase == "machine_enrollment")
    );
    assert!(operations.iter().all(|operation| {
        !operation.argv.iter().any(|argument| argument == "sudo")
            && !operation
                .argv
                .iter()
                .any(|argument| argument == "install.sh")
    }));
    assert!(
        operations
            .iter()
            .filter(|operation| operation.mutates)
            .all(|operation| matches!(
                operation.phase,
                "prepare" | "transfer" | "machine_enrollment"
            ))
    );
    assert_eq!(
        operations
            .iter()
            .find(|operation| operation.phase == "stage_read_back")
            .and_then(|operation| operation.expected_stdout_sha256.as_deref()),
        Some(BASKET_DIGEST)
    );
}

#[test]
fn fresh_authenticated_node_uses_declared_machine_enrollment_authority() {
    let decision = decide_framework_propagation(
        BASKET_DIGEST,
        "builder",
        "machine_enrollment",
        "root",
        &empty_observation(),
        Vec::new(),
    );

    assert_eq!(decision.status, "enroll_with_machine_authority");
    assert_eq!(decision.authority_class, Some("MachineEnrollment"));
}

#[test]
fn undeclared_or_changed_machine_principal_blocks_before_mutation() {
    let decision = decide_framework_propagation(
        BASKET_DIGEST,
        "builder",
        "machine_enrollment",
        "cynic-enroll",
        &empty_observation(),
        Vec::new(),
    );

    assert_eq!(decision.status, "blocked");
    assert!(
        decision
            .blockers
            .contains(&"machine_enrollment_principal_mismatch".to_string())
    );
}

#[test]
fn existing_other_runtime_is_routed_to_signed_release_instead_of_rebootstrap() {
    let mut observation = empty_observation();
    observation.runtime_marker_status = "observed";
    observation.runtime_marker = Some(serde_json::json!({
        "schema": "cynic.runtime_seed.release_marker.v3",
        "release_epoch": 0,
        "basket_digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
    }));

    let decision = decide_framework_propagation(
        BASKET_DIGEST,
        "builder",
        "machine_enrollment",
        "root",
        &observation,
        Vec::new(),
    );

    assert_eq!(decision.status, "blocked");
    assert!(
        decision
            .blockers
            .contains(&"existing_runtime_requires_signed_release_reconciliation".to_string())
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
    let report = apply_framework_propagation(&enrollment_plan(), false, &PanicRunner);

    assert_eq!(report.status, "blocked");
    assert!(!report.mutates);
    assert!(
        report
            .blockers
            .contains(&"explicit_yes_required".to_string())
    );
}

#[test]
fn successful_machine_enrollment_requires_exact_read_back_before_convergence() {
    let plan = enrollment_plan();
    let runner = MapArgvRunner {
        responses: plan
            .operations
            .iter()
            .map(|operation| {
                let stdout = match operation.phase {
                    "preflight" => "root\n".to_string(),
                    "stage_read_back" => format!(
                        "{}  seed-manifest.json\n",
                        BASKET_DIGEST.trim_start_matches("sha256:")
                    ),
                    "runtime_marker_read_back" => serde_json::json!({
                        "schema": "cynic.runtime_seed.release_marker.v3",
                        "release_epoch": 0,
                        "basket_digest": BASKET_DIGEST,
                    })
                    .to_string(),
                    "node_observation_read_back" => serde_json::json!({
                        "schema": "cynic.runtime.node_observation.v1",
                        "status": "observed",
                        "node_id": "builder",
                    })
                    .to_string(),
                    _ => String::new(),
                };
                (operation.argv.clone(), success(&stdout))
            })
            .collect(),
    };

    let report = apply_framework_propagation(&plan, true, &runner);

    assert_eq!(report.status, "converged");
    assert!(report.blockers.is_empty());
    assert!(report.read_back_evidence.is_some());
}

fn success(stdout: &str) -> ArgvOutput {
    ArgvOutput {
        exit_code: Some(0),
        stdout: stdout.to_string(),
        stderr: String::new(),
        spawn_error: None,
    }
}
