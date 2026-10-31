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
//! WHAT "ONE AUTHORITY" MEANS. One value, not one declaration. The disk
//! gate reads `build_lifecycle` per runner (`minimum_available_bytes_for_runner`):
//! a runner without a declaration has its gate silently off, which is why
//! every build_runner declares one (builder_role_parity.rs). The production
//! resolver accepts any number of declarations and refuses only when they
//! disagree (`build_runner_canonical_target_ambiguous`). These tests hold the
//! same line: every declaration must meet the same bar, and they must all
//! name the same canonical target.
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

/// Every build_runner that declares a build_lifecycle. At least one must exist.
fn declaring_build_runners() -> Vec<serde_json::Value> {
    let registry = registry_value();
    let nodes = registry
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .expect("the registry declares nodes");
    let found = nodes
        .iter()
        .filter(|node| {
            node.get("role").and_then(serde_json::Value::as_str) == Some("build_runner")
                && node.get("build_lifecycle").is_some()
        })
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        !found.is_empty(),
        "at least one build_runner declares the build_lifecycle"
    );
    found
}

fn node_id(node: &serde_json::Value) -> &str {
    node.get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("<unknown>")
}

fn canonical_target(node: &serde_json::Value) -> &str {
    node.pointer("/build_lifecycle/canonical_target_relative_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("{} declares no canonical target", node_id(node)))
}

#[test]
fn every_declaring_runner_declares_a_canonical_target() {
    for node in declaring_build_runners() {
        assert!(
            !canonical_target(&node).trim().is_empty(),
            "{}: the canonical target is not empty",
            node_id(&node)
        );
    }
}

#[test]
fn all_declarations_agree_on_the_canonical_target() {
    let nodes = declaring_build_runners();
    let mut distinct: Vec<&str> = nodes.iter().map(canonical_target).collect();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        1,
        "build_runners declare different canonical targets {distinct:?}: the resolver refuses \
         this as ambiguous, and a reclaim would not know which cache is governed"
    );
}

#[test]
fn the_canonical_target_resolves_from_the_registry() {
    let nodes = declaring_build_runners();
    let declared = canonical_target(&nodes[0]);
    let registry_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../infra/registry.json");
    let resolved =
        cynic_build::cache::build_runner::canonical_target_relative_path(&registry_path)
            .expect("the registry resolves the canonical target");
    assert_eq!(
        resolved, declared,
        "the resolver and the registry declare the same canonical target"
    );
}

#[test]
fn the_canonical_target_is_a_governed_build_targets_profile() {
    // 2026-10-22: `shared-target` was the Forgejo-runner warm cache until
    // a reclaim from a stale checkout purged it behind its active writer.
    // Writers now resolve under `build-targets/<profile>` (scheduler) and
    // `build-targets/fgate` (gate flow); the registry declares the gate
    // profile canonical. There is no single "runner directory" anymore, so
    // this test pins the declared intent instead of a discovery: change
    // the pin below only when the declared canonical moves — and migrate
    // the writer first.
    let state_root = cynic_types::paths::CynicPaths::state_dir();
    for node in declaring_build_runners() {
        let from_registry = state_root.join(canonical_target(&node));
        assert!(
            from_registry.is_absolute(),
            "{}: the canonical target resolves to an absolute path",
            node_id(&node)
        );
        assert_eq!(
            from_registry,
            state_root.join("build-targets/fgate"),
            "{}: the registry's canonical target is the declared gate cache; move it \
             deliberately, with the writer",
            node_id(&node)
        );
    }
}

#[test]
fn the_canonical_target_is_not_also_a_noncanonical_candidate() {
    for node in declaring_build_runners() {
        let canonical = canonical_target(&node);
        let candidates = node
            .pointer("/build_lifecycle/noncanonical_cache_candidates")
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| {
                panic!("{} declares no non-canonical candidates", node_id(&node))
            });
        for candidate in candidates {
            let relative = candidate
                .get("relative_path")
                .and_then(serde_json::Value::as_str)
                .expect("a candidate declares a relative path");
            assert_ne!(
                relative,
                canonical,
                "{}: a directory cannot be canonical and non-canonical at once",
                node_id(&node)
            );
        }
    }
}

#[test]
fn the_ephemeral_retention_scope_is_declared() {
    // Without a scope there is nothing to reclaim, and the reclaim
    // refuses to act on the ungoverned cache. The 2026-10-20 disk
    // saturation is what made this visible.
    for node in declaring_build_runners() {
        let retention = node
            .pointer("/build_lifecycle/ephemeral_retention")
            .unwrap_or_else(|| {
                panic!("{} declares no ephemeral retention", node_id(&node))
            });
        let scopes = retention
            .get("scopes")
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| panic!("{} retention declares no scopes", node_id(&node)));
        assert!(
            !scopes.is_empty(),
            "{}: the retention declares at least one scope",
            node_id(&node)
        );
        let covers_checkouts = scopes.iter().any(|scope| {
            scope
                .get("relative_path")
                .and_then(serde_json::Value::as_str)
                == Some("checkouts")
                && scope.get("kind").and_then(serde_json::Value::as_str)
                    == Some("evidence_checkout")
        });
        assert!(
            covers_checkouts,
            "{}: the retention covers the evidence checkouts, which is where the disk went",
            node_id(&node)
        );
    }
}
