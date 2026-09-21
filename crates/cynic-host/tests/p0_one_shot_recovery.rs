use std::fs;
use std::path::Path;
use std::process::Command;

use cynic_host::framework_bootstrap::build_runner::{
    BuildRunnerBootstrapRequest, apply_build_runner_bootstrap,
    build_runner_bootstrap_execution_plan, plan_build_runner_bootstrap,
};
use cynic_host::one_shot_authority::{
    LocalOneShotCapabilityAuthority, OneShotCapabilityAuthorityPort,
};
use cynic_lifecycle::LocalArgvRunner;
use cynic_build::{
    build_runner_bootstrap_action_digest,
};
use cynic_protocol::{
    ONE_SHOT_CAPABILITY_OUTCOME_SCHEMA,
    ONE_SHOT_CAPABILITY_SCHEMA,
    OneShotCapability,
    OneShotCapabilityOutcome,
    OneShotCapabilityOutcomeStatus,
};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

fn signed_capability(signing_key: &SigningKey, action_digest: String) -> OneShotCapability {
    let verifying_key = signing_key.verifying_key();
    let mut capability = OneShotCapability {
        schema: ONE_SHOT_CAPABILITY_SCHEMA.to_string(),
        status: "admitted".to_string(),
        authority_class: "MachineEnrollment".to_string(),
        capability_id: "p0-bootstrap-build".to_string(),
        subject: "node-key:sha256:p0-fixture".to_string(),
        scope: "build-runner-bootstrap".to_string(),
        input_digest: format!("sha256:{}", "a".repeat(64)),
        executor_digest: format!("sha256:{}", "b".repeat(64)),
        issued_at_unix_s: 10,
        expires_at_unix_s: 30,
        allowed_action_digests: vec![action_digest],
        authority_key_id: format!(
            "sha256:{}",
            hex::encode(Sha256::digest(verifying_key.as_bytes()))
        ),
        signature_hex: String::new(),
        evidence_refs: vec!["bootstrap_binding:p0-fixture".to_string()],
        budget: None,
    };
    capability.signature_hex = hex::encode(
        signing_key
            .sign(
                &capability
                    .signing_payload()
                    .expect("serialize signing payload"),
            )
            .to_bytes(),
    );
    capability
}

fn git(directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(directory)
        .args(arguments)
        .output()
        .expect("run git fixture command");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        arguments,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git fixture output is utf-8")
        .trim()
        .to_string()
}

#[test]
fn p0_crash_recovery_persists_unknown_outcome_and_refuses_capability_reuse() {
    let state_root = tempfile::tempdir().expect("temporary P0 state root");
    let signing_key = SigningKey::from_bytes(&[11_u8; 32]);
    let public_key_path = state_root.path().join("authority.pub");
    fs::write(
        &public_key_path,
        hex::encode(signing_key.verifying_key().as_bytes()),
    )
    .expect("write public key");
    let action_digest = build_runner_bootstrap_action_digest("make:build-host-exec")
        .expect("canonical bootstrap build action");
    let capability = signed_capability(&signing_key, action_digest.clone());
    let authority = LocalOneShotCapabilityAuthority::new(&public_key_path, state_root.path());

    authority
        .verify_and_consume(
            &capability,
            "node-key:sha256:p0-fixture",
            "build-runner-bootstrap",
            &action_digest,
            20,
        )
        .expect("consume signed capability before simulated crash");

    let recovered = authority
        .recover_unfinished_consumptions(21)
        .expect("persist unknown-after-crash outcome");
    assert_eq!(recovered.len(), 1);
    assert_eq!(
        recovered[0].status,
        OneShotCapabilityOutcomeStatus::UnknownAfterCrash
    );
    assert_eq!(recovered[0].schema, ONE_SHOT_CAPABILITY_OUTCOME_SCHEMA);
    assert_eq!(recovered[0].blockers, vec!["unknown_after_crash"]);

    let persisted: OneShotCapabilityOutcome = serde_json::from_slice(
        &fs::read(state_root.path().join("p0-bootstrap-build.outcome.json"))
            .expect("read durable recovery outcome"),
    )
    .expect("decode durable recovery outcome");
    assert_eq!(persisted, recovered[0]);
    assert!(
        authority
            .recover_unfinished_consumptions(22)
            .expect("recovery is idempotent")
            .is_empty()
    );
    assert!(
        authority
            .verify_and_consume(
                &capability,
                "node-key:sha256:p0-fixture",
                "build-runner-bootstrap",
                &action_digest,
                23,
            )
            .expect_err("a recovered capability cannot be reused")
            .starts_with("capability_already_consumed:")
    );
}

#[test]
fn p0_source_read_back_uses_a_content_addressed_bundle_not_the_calling_worktree() {
    let fixture = tempfile::tempdir().expect("temporary P0 source fixture");
    let source = fixture.path().join("source");
    let workspace = fixture.path().join("controlled-workspace");
    let bundle = fixture.path().join("source.bundle");
    fs::create_dir(&source).expect("create fixture source");
    git(&source, &["init"]);
    git(&source, &["config", "user.email", "p0@example.invalid"]);
    git(&source, &["config", "user.name", "P0 fixture"]);
    fs::create_dir_all(source.join("infra")).expect("create fixture registry directory");
    let registry = source.join("infra/registry.json");
    fs::write(&registry, br#"{"nodes":[]}"#).expect("write fixture registry");
    fs::write(source.join("immutable-source.txt"), "admitted bytes\n")
        .expect("write fixture source");
    git(&source, &["add", "."]);
    git(&source, &["commit", "-m", "P0 immutable source"]);
    let commit = git(&source, &["rev-parse", "HEAD"]);
    git(
        &source,
        &[
            "bundle",
            "create",
            bundle.to_str().expect("bundle path"),
            "HEAD",
        ],
    );
    let bundle_digest = format!(
        "sha256:{}",
        hex::encode(Sha256::digest(fs::read(&bundle).expect("read bundle")))
    );
    let registry_digest = format!(
        "sha256:{}",
        hex::encode(Sha256::digest(fs::read(&registry).expect("read registry")))
    );
    let plan = plan_build_runner_bootstrap(&BuildRunnerBootstrapRequest {
        node_id: "p0-builder".to_string(),
        source_commit: commit.clone(),
        source_bundle_digest: bundle_digest,
        target_phase: "build".to_string(),
    });
    let execution = build_runner_bootstrap_execution_plan(&plan, &workspace, &bundle)
        .expect("build source execution plan");
    fs::create_dir(&workspace).expect("create controlled workspace");
    git(&workspace, &["init"]);

    fs::write(
        source.join("immutable-source.txt"),
        "mutable caller bytes\n",
    )
    .expect("mutate calling worktree after bundle creation");
    let source_proof = apply_build_runner_bootstrap(
        &execution,
        &commit,
        &registry_digest,
        true,
        &LocalArgvRunner,
    )
    .expect("read back detached immutable source");

    assert_eq!(source_proof, format!("git-commit:{commit}"));
    assert_eq!(
        fs::read_to_string(workspace.join("immutable-source.txt")).expect("read detached source"),
        "admitted bytes\n"
    );
    assert_eq!(git(&workspace, &["rev-parse", "HEAD"]), commit);
}
