//! Verification of dynamic Headscale admission predicates and policies.

use cynic_network::fleet_membership::{MeshAdmissionPolicy, evaluate_preauth_spec};
use cynic_types::fleet_mesh::PreAuthKeySpec;

#[test]
fn admission_policy_accepts_valid_ephemeral_spec() {
    let policy = MeshAdmissionPolicy::default();
    let spec = PreAuthKeySpec {
        user: "cynic".to_string(),
        reusable: false,
        ephemeral: true,
        expiration: Some("2026-10-12T18:00:00Z".to_string()),
        acl_tags: vec!["tag:agent".to_string()],
    };
    let verdict = evaluate_preauth_spec(&policy, &spec);
    assert!(verdict.admitted);
}

#[test]
fn admission_policy_rejects_foreign_user() {
    let policy = MeshAdmissionPolicy::default();
    let spec = PreAuthKeySpec {
        user: "attacker".to_string(),
        reusable: false,
        ephemeral: true,
        expiration: None,
        acl_tags: vec!["tag:agent".to_string()],
    };
    let verdict = evaluate_preauth_spec(&policy, &spec);
    assert!(!verdict.admitted);
    assert!(verdict.reason.contains("invalid user"));
}

#[test]
fn admission_policy_rejects_non_ephemeral_when_mandated() {
    let policy = MeshAdmissionPolicy::default();
    let spec = PreAuthKeySpec {
        user: "cynic".to_string(),
        reusable: true,
        ephemeral: false,
        expiration: None,
        acl_tags: vec!["tag:agent".to_string()],
    };
    let verdict = evaluate_preauth_spec(&policy, &spec);
    assert!(!verdict.admitted);
    assert!(verdict.reason.contains("ephemeral"));
}

#[test]
fn admission_policy_rejects_missing_or_invalid_tags() {
    let policy = MeshAdmissionPolicy::default();
    let mut spec = PreAuthKeySpec {
        user: "cynic".to_string(),
        reusable: false,
        ephemeral: true,
        expiration: None,
        acl_tags: vec![],
    };
    let verdict = evaluate_preauth_spec(&policy, &spec);
    assert!(!verdict.admitted);

    spec.acl_tags = vec!["invalid_tag".to_string()];
    let verdict2 = evaluate_preauth_spec(&policy, &spec);
    assert!(!verdict2.admitted);
    assert!(verdict2.reason.contains("tag:"));
}
