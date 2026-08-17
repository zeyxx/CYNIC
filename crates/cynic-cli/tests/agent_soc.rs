//! TDD — `cynic agent soc` read-back contract (2N + 2N+1).
//!
//! RED→GREEN :
//!  - RED : sans runtime_proof.v1 → runtime_publisher.granted == false (déclaratif)
//!  - GREEN : avec runtime_proof.v1 signé → granted == true (2N+1 admission) +
//!    human_admin_role.zero_touch_runtime == true (invariant HumanAdmin)
//!
//! Ce test bloque toute regression : si jamais un commit force granted=true sans
//! proof (fake claim) ou brise le zero-touch invariant → test échoue.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use cynic_cli::agent_surface::scan::scan_agent_soc;

// Unique sandbox subdir per test to avoid parallel-test file races on the
// shared temp_dir() (the RED test removes runtime_proof.v1 — isolation is
// required for deterministic RED→GREEN across threads).
static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn tmp_sandbox() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "cynic-tdd-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).expect("create sandbox");
    dir
}

fn tmp_roles_path(sandbox: &PathBuf) -> PathBuf {
    sandbox.join("cynic-agent-roles-tdd.json")
}

fn tmp_proof_path(sandbox: &PathBuf) -> PathBuf {
    sandbox.join("runtime_proof.v1")
}

/// Écrit un agent-roles.json minimal (déclaratif, HumanAdmin + runtime_publisher).
fn write_declarative_roles(sandbox: &PathBuf) -> PathBuf {
    let path = tmp_roles_path(sandbox);
    let content = r#"{
        "$schema": "cynic.agent_roles.v1",
        "rules": [],
        "gitops_rules": [
            "HumanAdmin zero-touch rule: never touches runtime mutations",
            "runtime_publisher granted only via kernel/node-signed admission"
        ],
        "roles": {
            "human_admin": {
                "authority_class": "HumanAdmin",
                "secret_source_class": "HumanAdmin",
                "required_human_secret": true,
                "can_mutate": [],
                "can_observe": ["*"],
                "granted_via": "seed_one_shot",
                "zero_touch_runtime": true
            },
            "runtime_publisher": {
                "authority_class": "RuntimeMachineIdentity",
                "secret_source_class": "RuntimeMachineIdentity",
                "required_human_secret": false,
                "can_mutate": ["publish_artifact_proof"],
                "can_observe": ["runtime_history"],
                "granted_via": "kernel/node-signed BuildRunnerBootstrapCapabilityBinding (post-seed)",
                "zero_touch_runtime": true
            }
        },
        "domains": {
            "runtime": {"agent_role": "runtime_publisher"}
        }
    }"#;
    std::fs::write(&path, content).expect("write roles");
    path
}

/// Écrit infra/runtime_proof.v1 (kernel/node-signed) → granted: true.
fn write_runtime_proof(sandbox: &PathBuf) -> PathBuf {
    let path = tmp_proof_path(sandbox);
    let content = r#"{
        "schema": "cynic.runtime_proof.v1",
        "granted": true,
        "roles": {
            "runtime_publisher": {
                "granted": true,
                "authority_class": "RuntimeMachineIdentity",
                "granted_via": "kernel/node-signed runtime_proof.v1 (2N+1 admission)"
            }
        },
        "signature_ed25519": "DEADBEEF_TDD_PROOF_SIGNATURE"
    }"#;
    std::fs::write(&path, content).expect("write proof");
    path
}

// --- RED : sans runtime_proof.v1 → granted == false (déclaratif) ---

#[test]
fn red_declarative_granted_false_without_proof() {
    let sandbox = tmp_sandbox();
    let roles_path = write_declarative_roles(&sandbox);
    // N.B. scan_agent_soc(roles_path) lit runtime_proof.v1 dans le MEME répertoire
    // que roles_path. On s'assure qu'aucun proof n'existe → granted: false.
    let _ = std::fs::remove_file(roles_path.with_file_name("runtime_proof.v1"));

    let report = scan_agent_soc(&roles_path);
    let rp = report
        .roles
        .get("runtime_publisher")
        .expect("runtime_publisher role in report");

    // RED : sans proof, granted == false (déclaratif, pas d'assomption)
    assert!(
        !rp.granted,
        "RED : granted should be false without runtime_proof.v1, got {}",
        rp.granted
    );
    assert!(
        report.human_admin_role.zero_touch_runtime,
        "invariant : HumanAdmin must remain zero-touch"
    );
}

// --- GREEN : avec runtime_proof.v1 → granted == true (2N+1) + zero-touch ---

#[test]
fn green_granted_true_with_runtime_proof() {
    let sandbox = tmp_sandbox();
    let roles_path = write_declarative_roles(&sandbox);
    // Écrit le proof dans le même dir que roles_path (scan.rs with_file_name).
    let proof_path = write_runtime_proof(&sandbox);
    assert_eq!(
        proof_path,
        roles_path.with_file_name("runtime_proof.v1")
    );

    let report = scan_agent_soc(&roles_path);
    let rp = report
        .roles
        .get("runtime_publisher")
        .expect("runtime_publisher role in report");

    // GREEN : 2N+1 admission validée → granted == true
    assert!(
        rp.granted,
        "GREEN : granted should be true after runtime_proof.v1 admission, got {}",
        rp.granted
    );
    assert_eq!(
        rp.authority_class, "RuntimeMachineIdentity",
        "2N : authority_class must be RuntimeMachineIdentity"
    );
    assert!(
        rp.zero_touch_runtime,
        "2N invariant : runtime_publisher must be zero-touch"
    );

    // Invariant HumanAdmin — jamais runtime mutator, même post-admission.
    assert!(
        report.human_admin_role.zero_touch_runtime,
        "HumanAdmin zero-touch must hold after 2N+1 admission"
    );

    // Read-back proof : residual_unknowns vide (no guess).
    assert!(
        report.residual_unknowns.is_empty(),
        "residual_unknowns must be empty when proof observed, got {:?}",
        report.residual_unknowns
    );
}
