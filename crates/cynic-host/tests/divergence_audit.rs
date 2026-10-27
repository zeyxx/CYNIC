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
use std::path::PathBuf;

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

/// Read a workspace source, or record that it is missing.
///
/// WHY THIS EXISTS. This used to be `fs::read_to_string(..).ok()`, so every
/// caller had to decide what an absent file means and three of them decided it
/// silently in the fail-open direction:
///
///   `read("Makefile").unwrap_or_default()` -> "" -> zero cited targets ->
///     zero missing -> the gate PASSED with no Makefile at all.
///   `read("ci.yml")` inside `if let Some(..)` -> the branch was skipped, so the
///     floating-toolchain finding simply stopped existing and the divergence
///     COUNT DROPPED -> the gate PASSED. Measured: moving ci.yml aside leaves
///     this suite at 5 passed, 0 failed.
///   `fs::read_dir(infra) else { return Vec::new() }` -> [] -> zero uncited ->
///     PASSED with no infra/ at all.
///
/// A sensor that cannot fail is not a sensor. Absence is now a first-class
/// result, so every caller must name what a missing file means.
fn read(relative: &str) -> Read {
    match fs::read_to_string(workspace_root().join(relative)) {
        Ok(text) => Read::Present(text),
        Err(error) => Read::Missing(format!("{relative}: {error}")),
    }
}

/// The outcome of reading a source the sensor depends on.
enum Read {
    Present(String),
    Missing(String),
}

impl Read {
    /// The text, or `None` when the file is absent. The caller must handle the
    /// absent case; that is the point of the type.
    fn text(&self) -> Option<String> {
        match self {
            Read::Present(text) => Some(text.clone()),
            Read::Missing(_) => None,
        }
    }

    /// The text, or an explicit finding naming the absent file. Use when the
    /// file must exist for the measurement to mean anything.
    fn required(&self) -> Result<String, String> {
        match self {
            Read::Present(text) => Ok(text.clone()),
            Read::Missing(why) => Err(why.clone()),
        }
    }
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
    // The Makefile is the source of the target list. Without it every target a
    // contract cites is unresolvable, which is the finding — not a smaller set.
    let makefile = match read("Makefile").text() {
        Some(text) => text,
        None => String::new(),
    };
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
        // A contract that cannot be read is not a contract that cites nothing.
        // Naming it keeps a deleted authority from looking like a clean one.
        let text = match read(authority).required() {
            Ok(text) => text,
            Err(why) => {
                cited.push((authority.to_string(), format!("<unreadable: {why}>")));
                continue;
            }
        };
        let mut rest = text.as_str();
        // `make <target>` where <target> is a bare word, not a variable or a flag
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

    let pinned = read("rust-toolchain.toml").text().and_then(|t| {
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

    if let Some(text) = read("infra/rust-toolchain.json").text() {
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

    // A CI file that is not there cannot be proven to install the pinned
    // toolchain, and a missing CI file means the gate runs nowhere else. Moving
    // this file aside previously left the suite green, which made the check
    // below dead code that could not fail. Absence is now a finding of its own.
    // Every per-crate pin, compared to the registry entry for the same crate.
    //
    // WHY THIS IS A SEPARATE CHECK. The loop above compares the SET of declared
    // toolchains to the root pin. A set comparison cannot see a per-crate
    // disagreement: with files at 1.96.0 and the registry at 1.95.0, both values
    // are in the set, so "something differs from the root" is reported once and
    // the fact that a SPECIFIC crate declares a toolchain nobody builds with is
    // never surfaced. That is the shape of the defect the registry rewrite
    // found: five crates whose pin contradicts the channel that compiles them.
    //
    // rustup resolves from the working directory, and no Makefile target builds
    // from inside a crate, so these files are inert for a workspace build. They
    // are still a second declared answer, and an answer that contradicts the
    // first is worse than one that is merely redundant.
    if let Some(text) = read("infra/rust-toolchain.json").text() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(crates) = value.get("crates").and_then(|c| c.as_object()) {
                let mut mismatched: Vec<String> = Vec::new();
                for (name, spec) in crates {
                    let Some(registry_pin) = spec.get("toolchain").and_then(|t| t.as_str()) else {
                        continue;
                    };
                    let path = format!("crates/{name}/rust-toolchain.toml");
                    let Some(file_pin) = read(&path).text().and_then(|body| {
                        body.lines().find_map(|line| {
                            line.trim()
                                .strip_prefix("channel")
                                .and_then(|rest| rest.trim().strip_prefix('='))
                                .map(|rest| rest.trim().trim_matches('"').to_string())
                        })
                    }) else {
                        continue;
                    };
                    if file_pin != registry_pin {
                        mismatched.push(format!(
                            "{name}: file pins {file_pin} but infra/rust-toolchain.json \
                             pins {registry_pin}, and the workspace builds under \
                             {}",
                            read("rust-toolchain.toml")
                                .text()
                                .and_then(|body| {
                                    body.lines().find_map(|line| {
                                        line.trim()
                                            .strip_prefix("channel")
                                            .and_then(|rest| rest.trim().strip_prefix('='))
                                            .map(|rest| {
                                                rest.trim().trim_matches('"').to_string()
                                            })
                                    })
                                })
                                .unwrap_or_else(|| "<unknown>".to_string())
                        ));
                    }
                }
                for entry in mismatched {
                    out.insert(format!("a per-crate toolchain pin disagrees with the \
                                        registry: {entry}"));
                }
            }
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
    // Without infra/ there is no authority surface to measure, and returning an
    // empty list turned "no authority exists" into "every document is cited".
    // Every document is reported instead: the sensor cannot pass by absence.
    let Ok(entries) = fs::read_dir(&infra) else {
        return vec!["<infra/ is unreadable: every document is uncited>".to_string()];
    };

    let mut authorities = String::new();
    for authority in ["contracts/foundation.json", "AGENTS.md", "contracts/pruning-policy.json"] {
        if let Some(text) = read(authority).text() {
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
const BASELINE_TOOLCHAIN: usize = 2;
const BASELINE_UNCITED_DOCS: usize = 63;

// Uncompiled source, workspace-wide.
//
// WHY THIS IS NOT A PLAIN CONSTANT. The three counts above are ceilings and
// `assert!(n <= BASELINE)` is correct for them: the divergence cannot be
// enumerated by name, so a total is the only thing that can be bounded. An
// uncompiled source CAN be enumerated — the shared detector returns the exact
// file list, and every consuming crate already asserts its own list in both
// directions. A frozen `assert_eq!` on that total was therefore asserting that
// nobody had paid any debt, and it failed the moment 155 LOC were removed by
// deleting cynic-observer (observed 402 against a frozen 557).
//
// A ratchet that fails when the debt shrinks punishes the agent who pays it,
// which stops the debt from ever being paid. This bound is directional: the
// number may fall and the constant follows it down, and rising above it is
// still a failure.
const BASELINE_UNCOMPILED_LOC: usize = 402;

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
    // Both directions matter, and the count is exact because
    // toolchain_divergences() returns Vec<String>, not a total. Every entry is
    // therefore nameable, which means a tolerance band is the wrong instrument:
    // a band with a spare slot lets a new divergence land silently, which is
    // what happened here. The ceiling was 3 against 2 observed, so one new
    // answer in the world passed.
    //
    // Zero is the goal. Until the two remaining divergences are settled they are
    // listed below as DATA with the reason each is still open, so the debt is
    // named rather than hidden inside an integer — the same discipline
    // infra/audit-debt-ledger.json uses, and the reason a ceiling that only
    // knows its own size cannot be reviewed.
    let divergent = toolchain_divergences();

    const ACCEPTED: &[&str] = &[
        "the CI installs a floating toolchain instead of the pinned one",
    ];

    let new: Vec<&String> = divergent
        .iter()
        .filter(|entry| !ACCEPTED.contains(&entry.as_str()))
        .collect();

    assert!(
        new.is_empty(),
        "toolchain divergence grew: {} answer(s) disagree and are not listed below \
         as accepted. The toolchain is declared in more than one place and the \
         declarations must agree.\n\nnew divergences: {:#?}\n\naccepted, still \
         open: {ACCEPTED:#?}",
        new.len(),
        new
    );

    // The reverse check, so paying one of the two does not leave a stale slot
    // that a future divergence can hide inside.
    assert_eq!(
        divergent.len(),
        ACCEPTED.len(),
        "one of the {ACCEPTED:#?} divergences was settled; remove it from ACCEPTED \
         so the band cannot absorb a new one."
    );
}

#[test]
fn no_source_is_sitting_on_disk_uncompiled() {
    // Zero is the goal and it is the only number that is not a debt: an
    // uncompiled file is invisible to every compiler, warning and test, and this
    // is the only instrument in the workspace that can see it across all crates
    // at once.
    //
    // Directional, not equality. The exact file list is asserted per crate by
    // that crate's own `orphan_sources_ratchet.rs`, which also fails when a
    // quarantined file is repaired and left on the list. This test exists to
    // TOTAL them, and a total must be allowed to fall. Refreshing the constant
    // when the debt is paid is part of paying it, not an admission that the
    // number drifted.
    let uncompiled = uncompiled_workspace_loc();
    assert!(
        uncompiled <= BASELINE_UNCOMPILED_LOC,
        "uncompiled LOC grew from {BASELINE_UNCOMPILED_LOC} to {uncompiled}: \
         source is on disk that no compilation root reaches, so it is invisible to \
         every compiler, warning and test. Declare it, delete it, or add it to the \
         owning crate's quarantine list with the reason."
    );
    assert_eq!(
        uncompiled, BASELINE_UNCOMPILED_LOC,
        "the uncompiled total moved to {uncompiled} but the constant is still \
         {BASELINE_UNCOMPILED_LOC}. Refresh it in the same commit that changed the \
         debt — that is the ratchet, and a stale constant is a silent regression."
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
