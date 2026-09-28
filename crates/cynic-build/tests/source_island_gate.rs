// The shared detectors are `#[path]`-included by crates that use different
// halves of them; `dead_code` on the unused half is not a finding.
#![allow(dead_code)]
//! `SRC-001` / `SRC-002` / `SRC-003` — the closed-include-island gate, run over
//! the WHOLE workspace from ONE test file.
//!
//! WHY HERE AND NOT IN TWELVE PER-CRATE FILES. The detector lives in
//! `tests/support/`, shared by `#[path]`, and the defect it catches crossed file
//! boundaries inside one crate: `cynic-mcp/src/lib.rs` named one module while
//! three unreadable files attached forty more. A per-crate gate reproduces the
//! exact failure mode: twelve lists, twelve chances to leave one untouched, and
//! no rule about what a list is allowed to contain. One test, one rule, one
//! manifest, every crate measured — including the crates that have no gate at
//! all, which is where the mass was.
//!
//! WHY IN `cynic-build` AND NOT AT THE REPO ROOT. The root `Cargo.toml` is a
//! virtual manifest (no `[package]`), so `tests/` there is a directory of shared
//! sources, not a Cargo test target: `cargo test --test <name>` at the root
//! cannot exist without turning the workspace into a package, which would change
//! the build for 47 crates to host one test. `cynic-build` is the build/validation
//! domain, has no dependency on the crates it measures (so a broken crate cannot
//! hide the finding), and compiles independently of the `cynic-host` audit
//! engine. It is a host for the rule, not an owner of it: the rule id, the
//! policy, and the manifest all live outside this crate.
//!
//! HONEST STATE. `cynic-mcp` is clean (43 -> 0). This test is RED on the crates
//! whose islands predate it — see the failure output, which names every file and
//! the wall that hides it. The list was NOT weakened to make the tree green.

#[path = "../../../tests/support/orphan_sources.rs"]
mod orphan_sources;
#[path = "../../../tests/support/source_island.rs"]
mod source_island;

use std::collections::BTreeMap;
use std::fs;

use source_island::{Date, Quarantine, QuarantineEntry, Violation};

fn workspace_root() -> std::path::PathBuf {
    orphan_sources::workspace_root()
}

fn load_quarantine() -> Quarantine {
    Quarantine::load(&workspace_root()).unwrap_or_else(|error| panic!("{error}"))
}

/// The gate's own report, for tests that want to read rather than assert.
fn report() -> Vec<Violation> {
    let root = workspace_root();
    let today = source_island::today();
    let quarantine = load_quarantine();
    let islands = source_island::workspace_islands();
    let mut out = source_island::evaluate_islands(&islands, &quarantine, today);
    for finding in source_island::unprovable_build_prose(&root, source_island::PROSE_DIRS) {
        out.push(source_island::prose_violation(&finding));
    }
    out
}

fn render<'a>(violations: impl IntoIterator<Item = &'a Violation>) -> String {
    violations
        .into_iter()
        .map(Violation::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn cynic_mcp_has_no_closed_include_island() {
    // The regression this rule was written for. `cynic-mcp/src/lib.rs` declared
    // exactly one module while three unreadable files attached 40 more; 87df2f96
    // deleted the 43. This asserts the island is empty AND that the crate's whole
    // `src/` is one component, so a new island cannot hide behind an old one.
    let island = source_island::closed_include_island("cynic-mcp");
    assert_eq!(
        island.members.len(),
        0,
        "cynic-mcp: {} closed-island member(s) came back, {} line(s) the compiler never reads: \
         {:?}\n\nwall: {:?}\n\n{}",
        island.members.len(),
        island.loc(),
        island.paths(),
        island.wall,
        island
            .members
            .iter()
            .map(|member| format!("  {} ({} loc, attached by {:?})", member.path, member.loc, member.attached_by_dead))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    assert!(island.wall.is_empty(), "cynic-mcp: wall {:?}", island.wall);
}

#[test]
fn no_workspace_crate_declares_a_closed_include_island() {
    // SRC-001 + SRC-002 over every crate in `crates/`, not a hand-picked list.
    let violations = report();
    let islands: Vec<Violation> = violations
        .iter()
        .filter(|violation| violation.code.starts_with(source_island::RULE_CLOSED_INCLUDE_ISLAND))
        .cloned()
        .collect();
    assert!(
        islands.is_empty(),
        "{} closed-island finding(s) across the workspace:\n\n{}\n\nPer crate:\n{}",
        islands.len(),
        render(islands.iter()),
        island_summary(),
    );
}

#[test]
fn the_quarantine_manifest_is_a_valid_loan_book() {
    // SRC-002 alone: the manifest must parse, name the rule, declare a horizon
    // inside the policy cap, and hold no entry that has expired or over-reached.
    // A manifest that cannot be loaded is a FAILURE, not a skip.
    let today = source_island::today();
    let quarantine = load_quarantine();
    assert_eq!(quarantine.rule, source_island::RULE_CLOSED_INCLUDE_ISLAND);
    assert!(
        (1..=source_island::MAX_QUARANTINE_HORIZON_DAYS).contains(&quarantine.horizon_days),
        "horizon_days {} is outside 1..={}",
        quarantine.horizon_days,
        source_island::MAX_QUARANTINE_HORIZON_DAYS
    );
    let violations = source_island::evaluate_islands(
        &source_island::workspace_islands(),
        &quarantine,
        today,
    );
    let integrity: Vec<Violation> = violations
        .iter()
        .filter(|violation| violation.code.starts_with(source_island::RULE_QUARANTINE_INTEGRITY))
        .cloned()
        .collect();
    assert!(
        integrity.is_empty(),
        "{} quarantine-integrity finding(s):\n\n{}",
        integrity.len(),
        render(integrity.iter())
    );
}

#[test]
fn no_prose_claims_a_build_guarantee_the_build_cannot_give() {
    // SRC-003. The note that defended the `cynic-mcp` island said removing
    // `handler.rs` "breaks the build, which is how the omission was caught".
    // Nothing catches it, because the build cannot see a file it never reads.
    let findings =
        source_island::unprovable_build_prose(&workspace_root(), source_island::PROSE_DIRS);
    let violations: Vec<Violation> = findings
        .iter()
        .map(source_island::prose_violation)
        .collect();
    assert!(
        violations.is_empty(),
        "{} unprovable build-guarantee claim(s) in tests/ or infra/:\n\n{}\n\nA build cannot \
         catch a source file it never reads. State the checkable observation instead.",
        violations.len(),
        render(violations.iter())
    );
}

#[test]
fn the_island_rule_fires_on_a_synthetic_island() {
    // The rule proving itself. A fixture that reproduces the exact `cynic-mcp`
    // shape — one root module, one unreadable file, one file attached to it — so
    // a future edit that neuters the closure fails here and not in production.
    let fixture = crate_island_fixture("gate");
    let crate_dir = fixture.join("crates/synthetic");
    let island = source_island::closed_include_island_at(&crate_dir, &crate_dir.join("src"));
    assert_eq!(island.paths(), vec!["island/inner.rs"]);
    assert_eq!(island.wall, vec!["bridge.rs"]);
    assert!(island.members[0].attached_to_wall);
    assert_eq!(island.members[0].attached_by_dead, vec!["bridge.rs"]);
    assert!(island.members[0].attaches_dead.is_empty());
    assert!(island.loc() > 0);

    // Unquarantined: SRC-001 fires.
    let islands = BTreeMap::from([("synthetic".to_string(), island.clone())]);
    let empty = Quarantine {
        rule: source_island::RULE_CLOSED_INCLUDE_ISLAND.to_string(),
        horizon_days: 90,
        purpose: "fixture".to_string(),
        entries: Vec::new(),
    };
    let today = source_island::today();
    let fired = source_island::evaluate_islands(&islands, &empty, today);
    assert_eq!(fired.len(), 1, "expected one SRC-001 finding, got {fired:?}");
    assert_eq!(fired[0].code, "SRC-001-UNQUARANTINED");

    // Quarantined with an owner but no reason and no expiry: SRC-002 fires
    // twice, and the loan is not a pardon.
    let weak = Quarantine {
        rule: source_island::RULE_CLOSED_INCLUDE_ISLAND.to_string(),
        horizon_days: 90,
        purpose: "fixture".to_string(),
        entries: vec![QuarantineEntry {
            crate_name: "synthetic".to_string(),
            path: "island/inner.rs".to_string(),
            owner: "tbd".to_string(),
            reason: "later".to_string(),
            expires: String::new(),
        }],
    };
    let weak_violations = source_island::evaluate_islands(&islands, &weak, today);
    let codes: Vec<&str> = weak_violations
        .iter()
        .map(|violation| violation.code.as_str())
        .collect();
    for expected in [
        "SRC-002-NO-OWNER",
        "SRC-002-NO-REASON",
        "SRC-002-NO-EXPIRY",
    ] {
        assert!(codes.contains(&expected), "missing {expected} in {codes:?}");
    }

    // A valid, dated loan: silent, until the horizon passes it.
    let dated = |expires: String| Quarantine {
        rule: source_island::RULE_CLOSED_INCLUDE_ISLAND.to_string(),
        horizon_days: 90,
        purpose: "fixture".to_string(),
        entries: vec![QuarantineEntry {
            crate_name: "synthetic".to_string(),
            path: "island/inner.rs".to_string(),
            owner: "zeyxx:source-island-gate".to_string(),
            reason: "Superseded provider adapter kept for one bounded migration window.".to_string(),
            expires,
        }],
    };
    // Inside the horizon: silent. This is the ONLY shape a quarantine may take.
    let soon = Date { epoch_day: today.epoch_day + 30 }.to_iso();
    let valid = source_island::evaluate_islands(&islands, &dated(soon.clone()), today);
    assert!(valid.is_empty(), "a dated, owned loan inside the horizon must be silent: {valid:?}");
    let past = source_island::evaluate_islands(&islands, &dated("2020-01-01".to_string()), today);
    assert_eq!(past.len(), 1, "{past:?}");
    assert_eq!(past[0].code, "SRC-002-EXPIRED", "an expired loan must FAIL, not pass");
    let over_reach =
        source_island::evaluate_islands(&islands, &dated("2200-01-01".to_string()), today);
    assert_eq!(over_reach.len(), 1, "{over_reach:?}");
    assert_eq!(over_reach[0].code, "SRC-002-BEYOND-HORIZON");

    // An entry for a file that is NOT an island member is stale cover, and fails
    // even while it is unexpired: the list cannot pre-authorise a path. Dropping
    // the member's own entry in the process also re-opens SRC-001 on that member,
    // which is the point — cover for one file is never cover for another.
    let stale = Quarantine {
        entries: vec![QuarantineEntry {
            crate_name: "synthetic".to_string(),
            path: "loose.rs".to_string(),
            owner: "zeyxx:source-island-gate".to_string(),
            reason: "Pre-authorising a path that the compiler already reads.".to_string(),
            expires: soon.clone(),
        }],
        ..dated(soon)
    };
    let stale_violations = source_island::evaluate_islands(&islands, &stale, today);
    let stale_codes: Vec<&str> = stale_violations
        .iter()
        .map(|violation| violation.code.as_str())
        .collect();
    assert!(stale_codes.contains(&"SRC-002-STALE"), "{stale_codes:?}");
    assert!(
        stale_codes.contains(&"SRC-001-UNQUARANTINED"),
        "an entry for the wrong file must not cover the real member: {stale_codes:?}"
    );

    let _ = fs::remove_dir_all(&fixture);
}

#[test]
fn dates_round_trip_and_reject_impossible_days() {
    for iso in ["1970-01-01", "2026-09-28", "2000-02-29", "2024-12-31"] {
        let date = Date::parse(iso).unwrap_or_else(|error| panic!("{iso}: {error}"));
        assert_eq!(date.to_iso(), iso);
    }
    for bad in ["2026-02-30", "2026-13-01", "2026-00-10", "not-a-date", "2026-9-28x"] {
        assert!(Date::parse(bad).is_err(), "{bad} must not parse");
    }
    // 2026-03-01 minus 2026-02-28 is one day, across a non-leap February.
    let march = Date::parse("2026-03-01").unwrap_or_else(|error| panic!("{error}"));
    let february = Date::parse("2026-02-28").unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(march.epoch_day - february.epoch_day, 1);
}

fn island_summary() -> String {
    let islands = source_island::workspace_islands();
    if islands.is_empty() {
        return String::from("  (no closed island anywhere)");
    }
    islands
        .iter()
        .map(|(name, island)| {
            format!(
                "  {name}: {} member(s), {} line(s); wall {:?} ({} line(s))",
                island.members.len(),
                island.loc(),
                island.wall,
                island.wall_loc
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Build a throwaway workspace with the `cynic-mcp` shape:
///
/// ```text
/// src/lib.rs              pub mod server;              <- the only compilation root
/// src/server.rs
/// src/bridge.rs           include!("island/inner.rs"); <- nothing declares bridge.rs
/// src/island/inner.rs                                     <- attached to the unreadable file
/// src/loose.rs                                           <- orphan, nothing attaches it
/// ```
fn crate_island_fixture(label: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "cynic-source-island-fixture-{}-{label}",
        std::process::id()
    ));
    let src = base.join("crates/synthetic/src");
    fs::create_dir_all(src.join("island")).unwrap_or_else(|error| panic!("{error}"));
    fs::write(base.join("crates/synthetic/Cargo.toml"), "[package]\nname = \"synthetic\"\n")
        .unwrap_or_else(|error| panic!("{error}"));
    fs::write(src.join("lib.rs"), "pub mod server;\n").unwrap_or_else(|error| panic!("{error}"));
    fs::write(src.join("server.rs"), "pub fn ping() {}\n").unwrap_or_else(|error| panic!("{error}"));
    fs::write(
        src.join("bridge.rs"),
        "include!(\"island/inner.rs\");\n",
    )
    .unwrap_or_else(|error| panic!("{error}"));
    fs::write(
        src.join("island/inner.rs"),
        "pub fn inner() -> u8 { 7 }\n// padding line to make the mass visible\n",
    )
    .unwrap_or_else(|error| panic!("{error}"));
    fs::write(src.join("loose.rs"), "pub fn loose() {}\n").unwrap_or_else(|error| panic!("{error}"));
    base
}

/// The fixture reproduces the two real shapes, not one convenient one.
#[test]
fn the_fixture_shape_is_what_the_rule_documents() {
    let fixture = crate_island_fixture("shape");
    let crate_dir = fixture.join("crates/synthetic");
    let island = source_island::closed_include_island_at(&crate_dir, &crate_dir.join("src"));
    // `loose.rs` is a loose orphan: nothing attaches it. It is NOT an island
    // member, because that defect belongs to the orphan ratchet. Keeping the two
    // apart is what stops this rule from re-reporting SRC-001 for SRC-000, and it
    // is the `cynic-kernel` shape: `storage/memory.rs` attaches nine files that
    // are reachable elsewhere and one that is not.
    assert_eq!(island.paths(), vec!["island/inner.rs"]);
    let orphans = orphan_sources::all_sources(&crate_dir.join("src"));
    assert!(orphans.contains("loose.rs"), "{orphans:?}");
    assert!(orphans.contains("bridge.rs"), "{orphans:?}");
    let _ = fs::remove_dir_all(&fixture);
}
