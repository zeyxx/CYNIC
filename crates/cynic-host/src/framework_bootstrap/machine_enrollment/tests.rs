use std::os::unix::fs::MetadataExt;

use cynic_protocol::MachineEnrollmentAuthorityPolicy;

use super::{
    apply_machine_enrollment_authority, observe_machine_enrollment_authority,
    plan_machine_enrollment_authority, resolve_machine_enrollment_authority_spec,
};
use crate::framework_bootstrap::machine_enrollment::MachineEnrollmentAuthoritySpec;

fn spec(root: &std::path::Path) -> MachineEnrollmentAuthoritySpec {
    let metadata = std::fs::metadata(root).expect("temp metadata");
    MachineEnrollmentAuthoritySpec {
        authority_role: "kernel_primary".to_string(),
        endpoint_service: "cynic-kernel".to_string(),
        node_id: "cynic-builder".to_string(),
        seed_id: format!("sha256:{}", "a".repeat(64)),
        role: "build_runner".to_string(),
        requested_capabilities: vec!["node_control".to_string(), "build_runner".to_string()],
        policy: MachineEnrollmentAuthorityPolicy {
            authority_id: "runtime-enrollment".to_string(),
            token_ttl_s: 900,
            renewal_authority_ttl_s: 2_592_000,
            max_grant_ttl_s: 86_400,
            allowed_roles: vec!["build_runner".to_string()],
            allowed_capabilities: vec!["node_control".to_string(), "build_runner".to_string()],
        },
        config_root: root.join("etc").display().to_string(),
        staging_root: root.join("stage").display().to_string(),
        owner_id: metadata.uid(),
        group_id: metadata.gid(),
        now_unix_s: 1_000,
    }
}

#[test]
fn absent_authority_plans_one_bounded_issuance_transaction() {
    let directory = tempfile::tempdir().expect("tempdir");
    let spec = spec(directory.path());
    let observed = observe_machine_enrollment_authority(&spec);
    let plan = plan_machine_enrollment_authority(&spec, &observed);

    assert_eq!(plan.status, "planned");
    assert_eq!(plan.actions.len(), 4);
    assert!(!plan.mutates);
}

#[test]
fn explicit_apply_issues_and_reads_back_without_returning_secret_material() {
    let directory = tempfile::tempdir().expect("tempdir");
    let spec = spec(directory.path());
    let report = apply_machine_enrollment_authority(&spec, true);

    assert_eq!(report.status, "converged");
    assert!(report.mutates);
    assert!(!report.secret_material_returned);
    assert_eq!(report.after.staged_token_matches, Some(true));
    let rendered = serde_json::to_string(&report).expect("serialize report");
    let token = std::fs::read_to_string(spec.staged_token_path()).expect("staged token");
    assert!(!rendered.contains(token.trim()));
}

#[test]
fn existing_signing_key_is_reused_across_explicit_token_rotation() {
    let directory = tempfile::tempdir().expect("tempdir");
    let mut spec = spec(directory.path());
    let first = apply_machine_enrollment_authority(&spec, true);
    assert_eq!(first.status, "converged");
    let key_before = std::fs::read(spec.signing_key_path()).expect("signing key");

    spec.seed_id = format!("sha256:{}", "b".repeat(64));
    let second = apply_machine_enrollment_authority(&spec, true);
    assert_eq!(second.status, "converged");
    let key_after = std::fs::read(spec.signing_key_path()).expect("signing key");
    assert_eq!(key_before, key_after);
    assert_eq!(second.after.seed_id.as_deref(), Some(spec.seed_id.as_str()));
}

#[test]
fn authority_issuance_is_bound_to_the_registry_placed_runtime_host() {
    let directory = tempfile::tempdir().expect("tempdir");
    let registry_path = directory.path().join("registry.json");
    let seed_path = directory.path().join("seed.json");
    let capabilities = cynic_protocol::required_node_enrollment_capabilities("build_runner");
    std::fs::write(
        &registry_path,
        serde_json::to_vec(&serde_json::json!({
            "node_enrollment": {
                "schema": "cynic.node_enrollment.desired_state.v1",
                "authority_role": "kernel_primary",
                "endpoint_service": "cynic-kernel",
                "credential_delivery": "machine_enrollment_staging",
                "public_key_delivery": "machine_enrollment_staging",
                "authority_policy": {
                    "authority_id": "runtime-enrollment",
                    "token_ttl_s": 900,
                    "renewal_authority_ttl_s": 2592000,
                    "max_grant_ttl_s": 86400,
                    "allowed_roles": ["build_runner"],
                    "allowed_capabilities": capabilities
                }
            },
            "nodes": [
                {"id":"cortex","aliases":["ubuntu"],"role":"cortex_host"},
                {"id":"kernel","host_node_id":"cortex","role":"kernel_primary","services":[{"name":"cynic-kernel"}]}
            ]
        }))
        .expect("registry json"),
    )
    .expect("write registry");
    std::fs::write(
        &seed_path,
        format!(
            r#"{{"schema":"cynic.framework_seed.v2","seed_id":"sha256:{}","status":"seed_only","enrollment":{{"required":true,"target_role":"build_runner","identity_class":"MachineEnrollment"}}}}"#,
            "a".repeat(64)
        ),
    )
    .expect("write seed");
    let metadata = std::fs::metadata(directory.path()).expect("metadata");
    let args = (
        registry_path.as_path(),
        seed_path.as_path(),
        "builder",
        directory.path(),
        directory.path(),
        metadata.uid(),
        metadata.gid(),
        1_000,
    );
    let wrong_host = resolve_machine_enrollment_authority_spec(
        args.0, args.1, args.2, "builder", args.3, args.4, args.5, args.6, args.7,
    )
    .expect_err("builder must not issue kernel authority");
    assert_eq!(
        wrong_host,
        "machine_enrollment_authority_host_identity_mismatch"
    );
    assert!(
        resolve_machine_enrollment_authority_spec(
            args.0, args.1, args.2, "ubuntu", args.3, args.4, args.5, args.6, args.7,
        )
        .is_ok()
    );
}
