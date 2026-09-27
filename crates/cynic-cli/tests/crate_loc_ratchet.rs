//! God-object ceiling and legacy-debt ratchets, measured workspace-wide.
//!
//! `contracts/foundation.json:74` makes it a constitutional violation to have a
//! "God-object orchestrator exceeding 25k LOC". `infra/CANONICAL_CRATE_MAP.md`
//! records the census. Neither was executable: the only two files in the
//! workspace matching `25_000` were a token count and a float ratio in
//! unrelated test assertions. `crate_audit` bounds entrypoints and per-file
//! hotspots, never crate size.
//!
//! Two independent ratchets over one measurement, because one bound is not
//! enough to see the whole debt:
//!
//! 1. **Ceiling** — on-disk LOC per crate against 25 000. Measured on disk, not
//!    on compiled LOC, so that wiring a crate's dead mass into the build graph
//!    cannot make an alert disappear. Only extraction lowers it.
//! 2. **Legacy debt** — on-disk LOC minus compiled LOC per crate. Mass that
//!    exists but that no compilation root names: invisible to the compiler, to
//!    every warning, and to the ceiling. `cynic-mcp` is 95% dead this way and
//!    is not a god object; `cynic-kernel` is 85% dead and is both.
//!
//! Both fail when a recorded value RISES, and both fail when a recorded entry is
//! no longer true, so neither list can harden into permanent cover. The lists
//! are measured debt, not a tolerated baseline.
//!
//! The measurement is `orphan_sources::crate_loc_census`, which shares the
//! reachability rules the orphan ratchet uses. Two implementations of the same
//! rule would eventually disagree inside one repository.
//!
//! Definition, pinned so a divergence from the census is debuggable rather than
//! mysterious: every `.rs` file under a crate's `src/`, counted with
//! `str::lines().count()`. Both axes count `lib.rs` and `main.rs`, being
//! compilation roots in their own right; counting a crate root as dead mass would
//! put a floor under every gap. This yields a workspace on-disk total of
//! 388 255 against the 388 257 recorded in `infra/CANONICAL_CRATE_MAP.md`.

#[path = "../../../tests/support/orphan_sources.rs"]
mod orphan_sources;

use orphan_sources::{CrateLocCensus, crate_loc_census, crate_names};

/// `contracts/foundation.json:74`. Constitutional, not a preference.
const GOD_OBJECT_CEILING: usize = 25_000;

/// A gap below this is noise. Above it, the crate holds dead mass worth naming.
const MATERIAL_GAP: usize = 500;

/// Crates above the ceiling, with the on-disk LOC that put them there.
///
/// Recorded as explicit data with the debt visible, NOT tolerated. Every entry
/// is a known constitutional violation. The list may only shrink, and an entry
/// that no longer breaches must be removed so it cannot become cover.
const GOD_OBJECTS: &[(&str, usize)] = &[
    ("cynic-host", 93_206),
    ("cynic-node", 58_327),
    ("cynic-kernel", 40_434),
    ("cynic-protocol", 34_779),
    ("cynic-cli", 34_735),
    ("cynic-inference", 28_301),
];

/// Crates whose uncompiled mass is material, with that mass in LOC.
///
/// Same discipline: measured, visible, and shrink-only.
const LEGACY_DEBT: &[(&str, usize)] = &[
    ("cynic-kernel", 34_328),
    ("cynic-mcp", 12_812),
    ("cynic-cli", 10_708),
    ("cynic-host", 8_202),
    ("cynic-node", 3_995),
    ("cynic-inference", 2_988),
    ("cynic-askesis", 2_070),
    ("cynic-protocol", 1_481),
];

/// Workspace-wide uncompiled mass. Bounds the total, so moving debt between
/// crates without reducing it still fails.
const WORKSPACE_GAP: usize = 77_099;

fn census_all() -> Vec<(String, CrateLocCensus)> {
    crate_names()
        .into_iter()
        .map(|name| {
            let census = crate_loc_census(&name);
            (name, census)
        })
        .collect()
}

fn report(all: &[(String, CrateLocCensus)]) -> String {
    let mut out = String::from("\ncrate                 on_disk  compiled      gap\n");
    for (name, c) in all {
        out.push_str(&format!(
            "{name:20} {:8} {:9} {:7}\n",
            c.on_disk,
            c.compiled,
            c.gap()
        ));
    }
    out.push_str(&format!(
        "{:20} {:8} {:9} {:7}\n",
        "TOTAL",
        all.iter().map(|(_, c)| c.on_disk).sum::<usize>(),
        all.iter().map(|(_, c)| c.compiled).sum::<usize>(),
        all.iter().map(|(_, c)| c.gap()).sum::<usize>()
    ));
    out
}

#[test]
fn god_object_ceiling_ratchet_holds() {
    let all = census_all();

    let mut grew = Vec::new();
    let mut stale = Vec::new();
    for (name, recorded) in GOD_OBJECTS {
        let actual = all
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name}: recorded as a god object but the crate is gone"))
            .1
            .on_disk;
        if actual > *recorded {
            grew.push(format!("{name}: {actual} LOC, was {recorded}"));
        } else if actual <= GOD_OBJECT_CEILING {
            stale.push(format!(
                "{name}: {actual} LOC is at or under the {GOD_OBJECT_CEILING} ceiling — remove it"
            ));
        }
    }

    let undeclared: Vec<String> = all
        .iter()
        .filter(|(name, c)| c.on_disk > GOD_OBJECT_CEILING && !GOD_OBJECTS.iter().any(|(n, _)| n == name))
        .map(|(name, c)| format!("{name}: {} LOC now exceeds the ceiling", c.on_disk))
        .collect();

    assert!(
        grew.is_empty() && stale.is_empty() && undeclared.is_empty(),
        "god-object ceiling ratchet is {}.\n\n\
         grew past its recorded size (a god object absorbed new mass):\n  {}\n\n\
         no longer a god object but still listed (permanent cover, remove it):\n  {}\n\n\
         newly above the ceiling and not recorded:\n  {}\n\n\
         Extraction is the only way down: deleting files lowers the number but \
         discards the material, and wiring them into the build graph would make \
         the alert vanish without reducing the debt.{}", 
        if grew.is_empty() && stale.is_empty() && undeclared.is_empty() { "ok" } else { "VIOLATED" },
        grew.join("\n  "),
        stale.join("\n  "),
        undeclared.join("\n  "),
        report(&all)
    );
}

#[test]
fn legacy_debt_ratchet_holds() {
    let all = census_all();

    let mut grew = Vec::new();
    let mut stale = Vec::new();
    for (name, recorded) in LEGACY_DEBT {
        let actual = all
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name}: recorded as holding dead mass but the crate is gone"))
            .1
            .gap();
        if actual > *recorded {
            grew.push(format!(
                "{name}: {actual} LOC uncompiled, was {recorded} ({} more LOC that no \
                 compilation root names)",
                actual - recorded
            ));
        } else if actual < MATERIAL_GAP {
            stale.push(format!(
                "{name}: {actual} LOC gap is under the {MATERIAL_GAP} threshold — remove it"
            ));
        }
    }

    let undeclared: Vec<String> = all
        .iter()
        .filter(|(name, c)| c.gap() >= MATERIAL_GAP && !LEGACY_DEBT.iter().any(|(n, _)| n == name))
        .map(|(name, c)| format!("{name}: {} LOC uncompiled", c.gap()))
        .collect();

    assert!(
        grew.is_empty() && stale.is_empty() && undeclared.is_empty(),
        "legacy-debt ratchet is {}.\n\n\
         gained uncompiled mass:\n  {}\n\n\
         no longer material but still listed (permanent cover, remove it):\n  {}\n\n\
         newly above the {MATERIAL_GAP} LOC threshold and not recorded:\n  {}\n\n\
         The gap is mass that exists on disk but that no compilation root names. \
         Nothing reports it: not rustc, not the orphan ratchet's own error \
         count, not any target in the canonical gate.{}", 
        if grew.is_empty() && stale.is_empty() && undeclared.is_empty() { "ok" } else { "VIOLATED" },
        grew.join("\n  "),
        stale.join("\n  "),
        undeclared.join("\n  "),
        report(&all)
    );
}

#[test]
fn workspace_gap_ratchet_holds() {
    let all = census_all();
    let total: usize = all.iter().map(|(_, c)| c.gap()).sum();
    assert!(
        total <= WORKSPACE_GAP,
        "workspace uncompiled mass is {total} LOC, was {WORKSPACE_GAP} ({} more).\n\n\
         This bound exists so that moving dead mass between crates cannot make the \
         per-crate lists look stable while the total grows.{}",
        total as i64 - WORKSPACE_GAP as i64,
        report(&all)
    );
}
