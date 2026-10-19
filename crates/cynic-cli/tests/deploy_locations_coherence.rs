//! Coherence guard for the deploy location registry.
//!
//! The deploy locations registry (`infra/deploy-locations.json`) is the single
//! source of truth for WHERE the cynic binary lives on each fleet machine.
//! This test locks the properties that made the previous registry
//! (`infra/binary-locations.json`) wrong:
//!
//! 1. every declared machine resolves to a deployable path;
//! 2. no machine-local absolute path is hardcoded (SEC-001) — paths use the
//!    registry expansion vocabulary;
//! 3. a declared sha256 is a real sha256, and a declared git_sha is a real
//!    40-char SHA — the old registry carried a sha256 that existed on no
//!    machine;
//! 4. the policy points at this registry, and the superseded file is gone;
//! 5. every machine carries a smoke command and an identity command — a
//!    deploy without read-back is not a deploy.

use cynic_release::{
    decide_deploy_target, resolve_deploy_path, DeployLocationsContract,
    DEPLOY_LOCATIONS_SCHEMA,
};
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().parent().unwrap().to_path_buf()
}

fn load_locations() -> DeployLocationsContract {
    let path = repo_root().join("infra/deploy-locations.json");
    let content = std::fs::read_to_string(&path).expect("infra/deploy-locations.json must exist");
    let contract =
        DeployLocationsContract::parse(&content).expect("deploy locations must be valid JSON");
    assert_eq!(
        contract.schema, DEPLOY_LOCATIONS_SCHEMA,
        "deploy locations schema must be the declared one"
    );
    contract
}

#[test]
fn every_declared_machine_resolves_to_a_deployable_path() {
    let locations = load_locations();
    assert!(
        !locations.machines.is_empty(),
        "the deploy locations registry must declare at least one machine"
    );
    for machine in &locations.machines {
        let target = decide_deploy_target(&locations, &machine.hostname)
            .unwrap_or_else(|refusal| panic!("{} refused: {}", machine.hostname, refusal.reason));
        // The template must resolve against the standard roots, and the
        // resolved path must be absolute.
        let resolved = resolve_deploy_path(
            &target.deploy_path,
            Some("/var/lib/cynic"),
            Some("/home/operator"),
        )
        .unwrap_or_else(|refusal| {
            panic!("{} path unresolved: {}", machine.hostname, refusal.reason)
        });
        assert!(
            resolved.starts_with('/'),
            "{} resolved path must be absolute, got {}",
            machine.hostname,
            resolved
        );
    }
}

#[test]
fn no_machine_local_absolute_path_is_hardcoded() {
    let path = repo_root().join("infra/deploy-locations.json");
    let content = std::fs::read_to_string(&path).expect("deploy locations must exist");
    for (number, line) in content.lines().enumerate() {
        // Operator home paths are machine-local: they must stay templated.
        assert!(
            !line.contains("/home/"),
            "infra/deploy-locations.json:{} leaks a machine-local home path: {}",
            number + 1,
            line.trim()
        );
    }
}

#[test]
fn declared_shas_are_well_formed() {
    let locations = load_locations();
    for machine in &locations.machines {
        let measured = machine
            .measured
            .as_ref()
            .unwrap_or_else(|| panic!("{} must carry a measured state", machine.hostname));
        assert_eq!(
            measured.deploy_path_sha256.len(),
            64,
            "{} deploy sha256 must be a full sha256",
            machine.hostname
        );
        assert!(
            measured
                .deploy_path_sha256
                .chars()
                .all(|c| c.is_ascii_hexdigit()),
            "{} deploy sha256 must be hexadecimal",
            machine.hostname
        );
        assert_eq!(
            measured.deploy_path_git_sha.len(),
            40,
            "{} deploy git_sha must be a full 40-char SHA",
            machine.hostname
        );
    }
}

#[test]
fn every_machine_carries_the_read_back_surfaces() {
    let locations = load_locations();
    for machine in &locations.machines {
        assert!(
            !machine.readback.smoke_command.is_empty(),
            "{} must declare a smoke command",
            machine.hostname
        );
        assert!(
            !machine.readback.identity_command.is_empty(),
            "{} must declare an identity command",
            machine.hostname
        );
        assert!(
            !machine.readback.sha256_command.is_empty(),
            "{} must declare a sha256 read-back command",
            machine.hostname
        );
    }
}

#[test]
fn the_policy_points_at_this_registry() {
    let path = repo_root().join("infra/deploy-policy.json");
    let content = std::fs::read_to_string(&path).expect("infra/deploy-policy.json must exist");
    let policy: serde_json::Value = serde_json::from_str(&content).expect("valid policy JSON");
    let registry = policy["artifact"]["binary_locations_registry"]
        .as_str()
        .expect("the policy must name its binary locations registry");
    assert_eq!(
        registry, "infra/deploy-locations.json",
        "the deploy policy must point at the single deploy locations registry"
    );
}

#[test]
fn the_superseded_false_registry_is_gone() {
    let path = repo_root().join("infra/binary-locations.json");
    assert!(
        !path.exists(),
        "infra/binary-locations.json carried a sha256 that existed on no machine; \
         it must not come back"
    );
}

#[test]
fn every_planned_machine_is_declared_in_the_registry() {
    let path = repo_root().join("infra/deploy-policy.json");
    let content = std::fs::read_to_string(&path).expect("infra/deploy-policy.json must exist");
    let policy: serde_json::Value = serde_json::from_str(&content).expect("valid policy JSON");
    let order: Vec<String> = policy["canary_order"]
        .as_array()
        .expect("the policy must declare a canary order")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    assert!(
        !order.is_empty(),
        "the policy must declare a canary order"
    );
    let locations = load_locations();
    for machine in &order {
        assert!(
            locations.machine(machine).is_some(),
            "the canary order names {machine}, which the deploy locations registry does not declare"
        );
    }
}
