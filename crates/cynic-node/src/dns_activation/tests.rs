use cynic_host::update_lifecycle::{ArgvOutput, MapArgvRunner};

use super::apply::root_authority_blocker;
use super::decision::plan_dns_activation;
use super::registry::resolve_dns_activation_spec;
use super::rollback::rollback_matches;
use super::types::{
    DnsActivationFileObservation, DnsActivationFileSpec, DnsActivationObservation,
    DnsActivationSpec,
};

fn spec() -> DnsActivationSpec {
    DnsActivationSpec {
        schema: "cynic.dns_activation.spec.v1",
        zone: "synapse.cynic".to_string(),
        bundle_dir: "/bundle".to_string(),
        provider: "coredns".to_string(),
        provider_package: "coredns".to_string(),
        provider_provenance_proved: true,
        service_unit: "coredns-synapse.service".to_string(),
        resolver_unit: "systemd-resolved.service".to_string(),
        service_manager_kind: "systemd".to_string(),
        service_manager_bin: "/test/systemctl".to_string(),
        resolver_bin: "/test/getent".to_string(),
        probe_fqdn: "forgejo.synapse.cynic".to_string(),
        files: (0..4)
            .map(|index| DnsActivationFileSpec {
                source: format!("/bundle/source-{index}"),
                target: format!("/target/file-{index}"),
                sha256: format!("sha256:{}", "a".repeat(64)),
                mode: 0o644,
            })
            .collect(),
        blockers: Vec::new(),
    }
}

fn observation(converged: bool) -> DnsActivationObservation {
    DnsActivationObservation {
        schema: "cynic.dns_activation.observation.v1",
        files: spec()
            .files
            .into_iter()
            .map(|file| DnsActivationFileObservation {
                target: file.target,
                present: converged,
                regular: converged,
                sha256: converged.then_some(file.sha256),
                mode: converged.then_some(0o644),
                owner_uid: converged.then_some(0),
                owner_gid: converged.then_some(0),
            })
            .collect(),
        service_active: Some(converged),
        service_enabled: Some(converged),
        probe_resolves: Some(converged),
        evidence_refs: Vec::new(),
        unknowns: Vec::new(),
    }
}

#[test]
fn incomplete_observation_produces_one_bounded_transaction() {
    let plan = plan_dns_activation(&spec(), &observation(false));
    assert_eq!(plan.status, "planned");
    assert!(plan.mutates);
    assert_eq!(plan.actions.len(), 8);
}

#[test]
fn exact_files_service_and_resolution_are_converged() {
    let plan = plan_dns_activation(&spec(), &observation(true));
    assert_eq!(plan.status, "converged");
    assert!(!plan.mutates);
    assert!(plan.actions.is_empty());
}

#[test]
fn provider_provenance_blocks_before_mutation() {
    let mut desired = spec();
    desired
        .blockers
        .push("dns_activation_provider_provenance_unproved".to_string());
    let plan = plan_dns_activation(&desired, &observation(false));
    assert_eq!(plan.status, "blocked");
}

#[test]
fn bundle_tampering_is_a_pre_mutation_blocker() {
    let out = tempfile::tempdir().expect("tempdir");
    let registry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("infra/registry.json");
    crate::dns_bundle::build_dns_bundle(
        registry.to_str().expect("registry path"),
        out.path().to_str().expect("bundle path"),
    )
    .expect("bundle");
    std::fs::write(out.path().join("synapse.hosts"), "tampered\n").expect("tamper bundle");

    let desired = resolve_dns_activation_spec(
        registry.to_str().expect("registry path"),
        out.path().to_str().expect("bundle path"),
    )
    .expect("spec");

    assert!(
        desired
            .blockers
            .iter()
            .any(|blocker| blocker == "dns_activation_bundle_digest_mismatch:synapse.hosts")
    );
}

#[test]
fn rollback_requires_exact_service_state_read_back() {
    let before = observation(true);
    let mut after = before.clone();
    after.service_enabled = Some(false);

    assert!(!rollback_matches(&before, &after));
    assert!(rollback_matches(&before, &before));
}

#[test]
fn root_authority_is_observed_through_the_injected_runner() {
    let runner = MapArgvRunner {
        responses: vec![(
            vec!["id".to_string(), "-u".to_string()],
            ArgvOutput {
                exit_code: Some(0),
                stdout: "0\n".to_string(),
                stderr: String::new(),
                spawn_error: None,
            },
        )],
    };

    assert_eq!(root_authority_blocker(&runner), None);
}

#[test]
fn missing_root_observation_never_authorizes_mutation() {
    let runner = MapArgvRunner::default();

    assert_eq!(
        root_authority_blocker(&runner).as_deref(),
        Some("dns_activation_root_authority_unobservable")
    );
}
