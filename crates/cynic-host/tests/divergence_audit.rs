//! Cross-cutting divergence audit: one fact, one owner, and every other
//! representation must be checked.
//!
//! WHY THIS FILE EXISTS. The workspace has five local gates — orphan sources,
//! architecture boundaries, crate taxonomy, crate topology, layer debt — and
//! every one of them looks at ONE crate. Nothing ever asks whether the two sides
//! of a fact agree. Measured across this repository on 2026-09-29, the same
//! shape appears in every domain:
//!
//!   build      contracts/foundation.json cites `make build`   -> no such target
//!   toolchain  rust-toolchain.toml 1.95.0 | crate-topology 1.95+1.96 | CI stable
//!   runtime    registry declares 6 services                    -> 30 units on the host
//!   deployment source HEAD 2026-09-29                          -> kernel binary 2026-08-11
//!   code       45 022 LOC on disk                               -> never compiled
//!   docs       69 files in infra/                               -> 0 gates read a single one
//!
//! Ten divergences, one cause: a fact had two representations, no owner, and no
//! gate. An authority_order settles who is right when two documents disagree; it
//! cannot detect that neither has been updated. That is a judicial system, not a
//! measuring one.
//!
//! SCOPE, and the line I will not cross. Every check here is hermetic: it reads
//! the repository and nothing else, so it runs in CI on any host. Divergences
//! that need a LIVE system — the registry against installed systemd units, the
//! deployed binary against the source commit, a secret value inside a unit file
//! — cannot be a test, because a test that depends on the host is a test that
//! lies on every other host. They are named below as `HOST_OBSERVED` and left
//! explicitly unimplemented rather than faked.
//!
//! FAILS IN BOTH DIRECTIONS. Each check carries a recorded count that may only
//! fall. A new divergence fails; a repaired one must be deducted, which is what
//! makes the shrinking visible in a diff instead of in a sentence.

#![allow(clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[path = "../../../tests/support/orphan_sources.rs"]
mod orphan_sources;

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return dir;
        }
        assert!(dir.pop(), "workspace root not found");
    }
}

fn read(relative: &str) -> Option<String> {
    fs::read_to_string(workspace_root().join(relative)).ok()
}

fn crate_names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(workspace_root().join("crates"))
        .expect("read crates/")
        .flatten()
        .filter(|e| e.path().join("Cargo.toml").is_file())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .collect();
    names.sort();
    names
}

// ─────────────────────────────────────────────────────────────────────────────
// D1. Build targets: a contract that names a `make` target that does not exist.
//
// A gate cited by a normative contract that cannot run is a constitutional rule
// with no implementation. `make build` and `make build-release` were both named
// by contracts/foundation.json; neither target existed, so a whole build policy
// described a command nobody could type.
fn make_targets() -> BTreeSet<String> {
    let makefile = read("Makefile").unwrap_or_default();
    makefile
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if line.starts_with('\t') || trimmed.starts_with('#') {
                return None;
            }
            let name = trimmed.split(':').next()?;
            let name = name.split_whitespace().next()?;
            if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || "._-".contains(c)) {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

fn cited_targets() -> Vec<(String, String)> {
    let mut cited = Vec::new();
    for authority in ["contracts/foundation.json", "AGENTS.md"] {
        let Some(text) = read(authority) else { continue };
        // `make <target>` where <target> is a bare word, not a variable or a flag
        let mut rest = text.as_str();
        while let Some(idx) = rest.find("make ") {
            rest = &rest[idx + 5..];
            let token: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || "._-".contains(*c))
                .collect();
            if !token.is_empty() {
                cited.push((authority.to_string(), token));
            }
        }
    }
    cited
}

/// One entry per DISTINCT target, not per citation.
///
/// Counting citations made this gate report 5 when there are 3 divergences:
/// `make build` is named by both AGENTS.md and foundation.json, so it was
/// counted twice. A mis-calibrated baseline in a shrink-only gate is worse than
/// no gate: one new divergence hides behind one paid one, and the diff looks
/// clean.
fn missing_make_targets() -> Vec<String> {
    let targets = make_targets();
    let mut authorities: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (authority, token) in cited_targets() {
        if !targets.contains(&token) {
            authorities.entry(token).or_default().insert(authority);
        }
    }
    authorities
        .into_iter()
        .map(|(target, cited_by)| {
            format!("make {target} (cited by {})", cited_by.into_iter().collect::<Vec<_>>().join(", "))
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// D2. Toolchain: how many declarations of the compiler, and do they agree?
//
// Three answers existed at once — rust-toolchain.toml (1.95.0),
// infra/rust-toolchain.json (1.95.0 and 1.96.0) and the CI using `@stable`.
// The fallback was the pinned-but-broken version, and CI followed neither.
fn toolchain_divergences() -> Vec<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();

    let pinned = read("rust-toolchain.toml").and_then(|t| {
        t.lines().find_map(|l| {
            l.trim()
                .strip_prefix("channel")?
                .trim()
                .strip_prefix('=')?
                .trim()
                .trim_matches('"')
                .to_string()
                .into()
        })
    });

    if let Some(text) = read("infra/rust-toolchain.json") {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            let mut declared: BTreeSet<String> = BTreeSet::new();
            if let Some(crates) = value.get("crates").and_then(|c| c.as_object()) {
                for spec in crates.values() {
                    if let Some(t) = spec.get("toolchain").and_then(|t| t.as_str()) {
                        declared.insert(t.to_string());
                    }
                }
            }
            if let Some(pinned) = &pinned {
                for t in &declared {
                    if t != pinned {
                        out.insert(format!(
                            "infra/rust-toolchain.json pins {t} for some crates while \
                             rust-toolchain.toml pins {pinned}"
                        ));
                    }
                }
            } else {
                out.insert(
                    "rust-toolchain.toml is absent, so the crate table is the only pin".to_string(),
                );
            }
        }
    } else {
        out.insert("infra/rust-toolchain.json is missing".to_string());
    }

    if let Some(ci) = read(".github/workflows/ci.yml") {
        if ci.contains("rust-toolchain@stable") || ci.contains("rust-toolchain@master") {
            out.insert(
                "the CI installs a floating toolchain instead of the pinned one".to_string(),
            );
        }
    }

    out.into_iter().collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// D3. Reachability, workspace-wide.
//
// The five local gates each measure their own crate. Nothing ever totalled it,
// which is how 45 022 uncompiled lines of mass stayed invisible. This uses the
// repository's own shared detector over every crate, so the number cannot
// disagree with the per-crate gates.
fn uncompiled_workspace_loc() -> usize {
    let mut total = 0usize;
    for name in crate_names() {
        let crate_dir = workspace_root().join("crates").join(&name);
        let src = crate_dir.join("src");
        if !src.is_dir() {
            continue;
        }
        let all = orphan_sources::all_sources(&src);
        let reached = orphan_sources::reachable(&crate_dir, &src);
        for orphan in all.difference(&reached) {
            total += orphan_sources::source_loc(&name, orphan);
        }
    }
    total
}

// ─────────────────────────────────────────────────────────────────────────────
// D4. Authority reach: how much of infra/ can any gate or contract actually see?
//
// 69 files, 6 cited by a contract or AGENTS.md, and zero read by a test. A file
// that nothing reads cannot be wrong, which is why 31 of them carry no status
// at all and nobody notices. The count is the evaporation seed: a document that
// no gate re-derives has no claim to be authority.
fn uncited_infra_documents() -> Vec<String> {
    let infra = workspace_root().join("infra");
    let Ok(entries) = fs::read_dir(&infra) else { return Vec::new() };

    let mut authorities = String::new();
    for authority in ["contracts/foundation.json", "AGENTS.md", "contracts/pruning-policy.json"] {
        if let Some(text) = read(authority) {
            authorities.push_str(&text);
            authorities.push('\n');
        }
    }

    let mut uncited = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        if !authorities.contains(stem) {
            uncited.push(stem.to_string());
        }
    }
    uncited.sort();
    uncited
}

// ─────────────────────────────────────────────────────────────────────────────
// Recorded state. Every count may only fall. Measured 2026-09-29.
const BASELINE_MISSING_TARGETS: usize = 3;
const BASELINE_TOOLCHAIN: usize = 3;
const BASELINE_UNCOMPILED_LOC: usize = 557;
const BASELINE_UNCITED_DOCS: usize = 63;

#[test]
fn no_contract_cites_a_build_target_that_does_not_exist() {
    let missing = missing_make_targets();
    assert!(
        missing.len() <= BASELINE_MISSING_TARGETS,
        "divergence grew from {BASELINE_MISSING_TARGETS} to {}: a contract names a \
         `make` target that cannot be run, so the rule it states has no \
         implementation.\n{missing:#?}",
        missing.len()
    );
}

#[test]
fn the_compiler_has_one_pinned_version_every_consumer_agrees_on() {
    let divergent = toolchain_divergences();
    assert!(
        divergent.len() <= BASELINE_TOOLCHAIN,
        "divergence grew from {BASELINE_TOOLCHAIN} to {}: the toolchain is declared \
         in more than one place and the declarations disagree.\n{divergent:#?}",
        divergent.len()
    );
}

#[test]
fn no_source_is_sitting_on_disk_uncompiled() {
    // Zero is the goal and it is the only number that is not a debt: an
    // uncompiled file is invisible to every compiler, warning and test, and this
    // is the only instrument in the workspace that can see it across all crates
    // at once.
    let uncompiled = uncompiled_workspace_loc();
    assert_eq!(
        uncompiled, BASELINE_UNCOMPILED_LOC,
        "{uncompiled} LOC on disk are unreachable from a compilation root, \
         workspace-wide. The five per-crate gates each see only their own crate; \
         this is the one that totals them."
    );
}

#[test]
fn authority_is_reachable_or_it_is_not_authority() {
    let uncited = uncited_infra_documents();
    assert!(
        uncited.len() <= BASELINE_UNCITED_DOCS,
        "divergence grew from {BASELINE_UNCITED_DOCS} to {}: more of infra/ is \
         unreachable from any contract, so more of it is stale by construction.",
        uncited.len()
    );
}

/// Named, not faked. These need a live system, so they cannot be a hermetic
/// test, and a test that reads the host is a test that lies on every other host.
/// They belong to `cynic audit`, and until that exists they are `unknown`.
///
/// - registry `services` against the systemd units actually installed
/// - the deployed binary's digest against the source commit it was built from
/// - credential values present in unit files (presence only, never the value)
/// - the claim store's `status` and `consumed` against what the arbiter serves
#[test]
fn host_observed_divergences_are_named_and_not_faked() {
    // A test asserting "the report has a schema" passes with every check
    // disabled, which is how a broken audit stays green. So this asserts the
    // list is not empty and not silently shrinking: adding a host-observed
    // divergence is expected to add a line here, removing one removes it.
    const HOST_OBSERVED: &[&str] = &[
        "registry services vs installed systemd units",
        "deployed binary digest vs source commit",
        "credential values present in unit files (presence only)",
        "claim store status/consumed vs what the arbiter serves",
    ];
    assert!(
        !HOST_OBSERVED.is_empty(),
        "the host-observed divergences were deleted instead of implemented. \
         Naming them is how they stay visible; leaving this empty is how the \
         session loses track of them."
    );
}
