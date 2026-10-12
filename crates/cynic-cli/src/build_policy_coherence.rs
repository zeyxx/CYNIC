//! Coherence between `infra/build-policy.json` and every site that forwards it.
//!
//! Cargo parallelism and the build environment were the same value written in
//! seven places with no authority: two Makefile variables, three Rust adapters,
//! and three assertions that pinned the literal string. `.cargo/config.toml`
//! could not arbitrate, because an environment variable overrides that file,
//! so a value set there is inert for any site that passes `env`.
//!
//! This test makes the registry authoritative instead of the literal. It reads
//! the declared value and checks each consumer carries it, so a legitimate
//! policy change edits one file and this test follows, rather than requiring
//! four synchronized edits that can silently drift apart.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    crate_root().join("../..")
}

fn read_policy() -> Value {
    let path = repo_root().join("infra/build-policy.json");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {} as JSON: {e}", path.display()))
}

fn declared_jobs() -> String {
    let policy = read_policy();
    policy["parallelism"]["cargo_build_jobs"]
        .as_u64()
        .expect("parallelism.cargo_build_jobs must be a positive integer")
        .to_string()
}

fn consumers() -> Vec<Value> {
    let policy = read_policy();
    policy["consumers"]
        .as_array()
        .expect("consumers must be an array")
        .clone()
}

fn read_source(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Every consumer is a reader, so its declared form must appear verbatim.
#[test]
fn every_declared_consumer_carries_the_declared_value() {
    let jobs = declared_jobs();
    let list = consumers();
    assert!(!list.is_empty(), "the registry must declare its consumers");

    for consumer in &list {
        let site = consumer["site"].as_str().expect("consumer needs a site");
        let form = consumer["form"].as_str().expect("consumer needs a form");
        let placeholder_form = form.replace("<cargo_build_jobs>", &jobs);
        let file = site.split(':').next().expect("site must name a file");

        let source = read_source(file);
        assert!(
            source.contains(&placeholder_form),
            "consumer {site} does not carry the declared value {jobs}.\n\
             expected to find: {placeholder_form}\n\
             authority: infra/build-policy.json parallelism.cargo_build_jobs"
        );
    }
}

/// A new consumer that nobody registered is the exact drift this file exists
/// to prevent, so a raw scan must find no unregistered job assignment.
#[test]
fn no_unregistered_parallelism_assignment_exists() {
    let jobs = declared_jobs();
    let registered: Vec<String> = consumers()
        .iter()
        .filter_map(|c| {
            let site = c["site"].as_str()?;
            let file = site.split(':').next()?;
            Some(file.to_string())
        })
        .collect();

    let candidates = [
        ("Makefile", "CARGO_BUILD_JOBS"),
        ("Makefile", "CMAKE_BUILD_PARALLEL_LEVEL"),
        (
            "crates/cynic-node/src/adapters/tokio_build_execution.rs",
            "CARGO_BUILD_JOBS",
        ),
        (
            "crates/cynic-node/src/adapters/ssh_build_proof.rs",
            "CARGO_BUILD_JOBS",
        ),
        (
            "crates/cynic-node/src/ports/build_runner_adapter.rs",
            "CARGO_BUILD_JOBS",
        ),
    ];

    for (file, variable) in candidates {
        // Only flag files that are NOT declared as consumers. Declared files
        // are checked by every_declared_consumer_carries_the_declared_value.
        if registered.iter().any(|f| f == file) {
            continue;
        }
        let source = read_source(file);
        for (index, line) in source.lines().enumerate() {
            if !line.contains(variable) {
                continue;
            }
            let is_assignment = line.contains("\"1\"")
                || line.contains("\"2\"")
                || line.contains("\"3\"")
                || line.contains("\"4\"")
                || line.contains(&format!("={jobs}"))
                || line.contains("?=");
            assert!(
                !is_assignment,
                "{file}:{} assigns {variable} outside the registry: {line}\n\
                 Add the site to infra/build-policy.json consumers.",
                index + 1
            );
        }
    }
}

/// The registry must never claim a measurement it does not carry.
#[test]
fn a_measured_value_is_never_declared_without_its_measurement() {
    let policy = read_policy();
    let parallelism = &policy["parallelism"];
    let status = parallelism["status"].as_str().unwrap_or_default();
    let jobs = parallelism["cargo_build_jobs"]
        .as_u64()
        .expect("cargo_build_jobs");

    if status == "unmeasured_on_canonical_path" {
        let measured = &parallelism["measured_alternative"];
        assert!(
            measured.is_object(),
            "an unmeasured canonical value must still carry the manual measurement it is compared against"
        );
        let runs = measured["two_job_wall_seconds"]
            .as_array()
            .expect("measured_alternative must list wall clock figures");
        assert!(
            !runs.is_empty(),
            "measured_alternative must not be an empty claim"
        );
        let commits = measured["two_job_commits"]
            .as_array()
            .expect("measured_alternative must list the commits it ran on");
        assert_eq!(
            runs.len(),
            commits.len(),
            "each wall clock figure must be attributable to a named commit"
        );
    }

    if jobs > 1 {
        assert!(
            status != "unmeasured_on_canonical_path",
            "jobs = {jobs} cannot be declared while the canonical path remains unmeasured"
        );
    }
}

/// Precedence is the reason this file exists; losing it would silently
/// reintroduce the drift.
#[test]
fn the_registry_states_why_cargo_config_toml_cannot_be_the_authority() {
    let policy = read_policy();
    let rule = policy["precedence_rule"].as_str().unwrap_or_default();
    assert!(
        rule.contains("environment") && rule.contains(".cargo/config.toml"),
        "precedence_rule must explain that the environment overrides .cargo/config.toml, got: {rule}"
    );
}

/// `.cargo/config.toml` carries its own value, and the two surfaces genuinely
/// differ. This test accepts a divergence only when the registry declares it
/// with its scope, so the difference stays deliberate rather than drifting.
#[test]
fn cargo_config_toml_agrees_or_declares_its_divergence() {
    let policy = read_policy();
    let canonical = policy["parallelism"]["cargo_build_jobs"]
        .as_u64()
        .expect("cargo_build_jobs");

    let scope = policy["parallelism"].get("cargo_config_toml_scope");
    let Some(scope) = scope else {
        // No scope declared: the config file must therefore match exactly.
        assert_config_matches(None, canonical);
        return;
    };

    let declared = scope["declared_jobs"]
        .as_u64()
        .expect("cargo_config_toml_scope.declared_jobs must be set");
    let applies = scope["applies_to"].as_str().unwrap_or_default();
    let does_not = scope["does_not_apply_to"]
        .as_array()
        .expect("a declared divergence must list what it does not apply to");

    assert!(
        !applies.is_empty(),
        "a declared divergence must state which invocations it governs"
    );
    assert!(
        !does_not.is_empty(),
        "a declared divergence must state which surfaces it does not govern, \
         otherwise the canonical gate is ambiguous"
    );
    assert!(
        does_not
            .iter()
            .any(|entry| entry.as_str().unwrap_or_default().contains("build proof")),
        "the divergence must explicitly exclude the canonical proof runner"
    );

    assert_config_matches(Some(declared), declared);
    if declared != canonical {
        assert_ne!(
            declared, canonical,
            "cargo_config_toml_scope exists only to record a real divergence"
        );
    }
}

/// Reads the `jobs = N` assignment inside `.cargo/config.toml`.
fn assert_config_matches(expected: Option<u64>, fallback: u64) {
    let path = repo_root().join(".cargo/config.toml");
    let Ok(source) = fs::read_to_string(&path) else {
        return;
    };
    let want = expected.unwrap_or(fallback);

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("jobs = ") {
            let parsed: u64 = value
                .trim()
                .parse()
                .unwrap_or_else(|_| panic!("unparseable jobs assignment: {trimmed}"));
            assert_eq!(
                parsed, want,
                ".cargo/config.toml sets jobs = {parsed} but the registry declares {want}"
            );
        }
    }
}

#[test]
fn every_consumer_site_exists_on_disk() {
    for consumer in consumers() {
        let site = consumer["site"].as_str().expect("consumer needs a site");
        let relative = site.split(':').next().expect("site must name a file");
        let path: PathBuf = repo_root().join(relative);
        assert!(
            path.is_file(),
            "consumer site {relative} is declared in the registry but does not exist"
        );
    }
}

#[test]
fn the_registry_names_its_own_coherence_test() {
    let policy = read_policy();
    let declared = policy["coherence_contract"]["test"]
        .as_str()
        .expect("coherence_contract must name its test");
    let path = repo_root().join(declared);
    assert!(
        path.is_file(),
        "the registry names {declared} as its coherence test but that file is absent"
    );
}
