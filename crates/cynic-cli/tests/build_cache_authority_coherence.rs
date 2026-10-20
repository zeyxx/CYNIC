//! The build cache path has ONE authority: the registry.
//!
//! THE DIVERGENCE THIS LOCKS. The runner used to compile the build
//! cache path into itself (`state_dir().join("shared-target")`) while the
//! registry declared another one (`build-targets/bootstrap`). Both
//! claimed to describe the same directory on the same machine, and they
//! disagreed.
//!
//! WHY IT MATTED. `cynic build reclaim` reads the registry to know
//! what to reclaim. A compiled-in path put the real 30 Go of build
//! cache outside the policy that exists to reclaim it: the reclaim
//! looked for a canonical target that did not exist, reported
//! `canonical_target_state: "missing"`, and the disk filled to 100%.
//!
//! These tests read the COMMITTED registry, so a path that drifts
//! between the registry and the code fails here rather than in
//! production, where the symptom is a full disk and a truncated clone.

use std::path::Path;

fn registry_value() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../infra/registry.json")
        .canonicalize()
        .expect("registry.json is reachable from the crate manifest");
    let bytes = std::fs::read(&path).expect("registry.json is readable");
    serde_json::from_slice(&bytes).expect("registry.json is valid JSON")
}

fn build_runner_node() -> serde_json::Value {
    let registry = registry_value();
    let nodes = registry
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .expect("the registry declares nodes");
    let mut found = nodes
        .iter()
        .filter(|node| node.get("role").and_then(serde_json::Value::as_str) == Some("build_runner"))
        .collect::<Vec<_>>();
    assert_eq!(
        found.len(),
        1,
        "exactly one node declares the build_runner role"
    );
    found.remove(0).clone()
}

#[test]
fn the_build_runner_declares_a_canonical_target() {
    let node = build_runner_node();
    let declared = node
        .pointer("/build_lifecycle/canonical_target_relative_path")
        .and_then(serde_json::Value::as_str)
        .expect("the build runner declares its canonical target");
    assert!(
        !declared.trim().is_empty(),
        "the canonical target is not empty"
    );
}

#[test]
fn the_canonical_target_resolves_from_the_registry() {
    let node = build_runner_node();
    let declared = node
        .pointer("/build_lifecycle/canonical_target_relative_path")
        .and_then(serde_json::Value::as_str)
        .expect("the build runner declares its canonical target");
    let resolved =
        cynic_host::registry::canonical_target::canonical_target_relative_path(&registry_value())
            .expect("the registry resolves the canonical target");
    assert_eq!(
        resolved, declared,
        "the resolver and the registry declare the same canonical target"
    );
}

#[test]
fn the_canonical_target_is_the_directory_the_runner_uses() {
    // The runner resolves the cache against the state root. The registry
    // path is RELATIVE to that same root, so the two are comparable once
    // joined. This is the assertion that would have caught the
    // divergence: the runner used one directory, the policy another.
    let node = build_runner_node();
    let declared = node
        .pointer("/build_lifecycle/canonical_target_relative_path")
        .and_then(serde_json::Value::as_str)
        .expect("the build runner declares its canonical target");
    let state_root = cynic_types::paths::CynicPaths::state_dir();
    let from_registry = state_root.join(declared);
    assert!(
        from_registry.is_absolute(),
        "the canonical target resolves to an absolute path"
    );
    assert_eq!(
        from_registry,
        state_root.join("shared-target"),
        "the registry's canonical target is the directory the runner writes to"
    );
}

#[test]
fn the_canonical_target_is_not_also_a_noncanonical_candidate() {
    let node = build_runner_node();
    let canonical = node
        .pointer("/build_lifecycle/canonical_target_relative_path")
        .and_then(serde_json::Value::as_str)
        .expect("the build runner declares its canonical target");
    let candidates = node
        .pointer("/build_lifecycle/noncanonical_cache_candidates")
        .and_then(serde_json::Value::as_array)
        .expect("the policy declares its non-canonical candidates");
    for candidate in candidates {
        let relative = candidate
            .get("relative_path")
            .and_then(serde_json::Value::as_str)
            .expect("a candidate declares a relative path");
        assert_ne!(
            relative, canonical,
            "a directory cannot be canonical and non-canonical at once"
        );
    }
}

#[test]
fn the_ephemeral_retention_scope_is_declared() {
    // Without a scope there is nothing to reclaim, and the reclaim
    // refuses to act on the ungoverned cache. The 2026-10-20 disk
    // saturation is what made this visible.
    let node = build_runner_node();
    let retention = node
        .pointer("/build_lifecycle/ephemeral_retention")
        .expect("the builder declares its ephemeral retention");
    let scopes = retention
        .get("scopes")
        .and_then(serde_json::Value::as_array)
        .expect("the retention declares its scopes");
    assert!(!scopes.is_empty(), "the retention declares at least one scope");
    let covers_checkouts = scopes.iter().any(|scope| {
        scope.get("relative_path").and_then(serde_json::Value::as_str) == Some("checkouts")
            && scope.get("kind").and_then(serde_json::Value::as_str) == Some("evidence_checkout")
    });
    assert!(
        covers_checkouts,
        "the retention covers the evidence checkouts, which is where the disk went"
    );
}
