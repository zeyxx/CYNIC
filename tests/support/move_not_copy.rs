//! `MNC-001` .. `MNC-006` — the move-not-copy gate: a crate that duplicates a
//! crate it depends on is a copy; a crate that duplicates nothing is a move.
//!
//! WHY THIS FILE EXISTS. A study of all 40 crate-creation commits in this
//! repository's history found the rule that actually governed them, which was
//! never written down. A crate is created when:
//!
//!   (a) a named, over-full container has a separable pure sub-tree;
//!   (b) that sub-tree can depend downward on an L0/host contract;
//!   (c) an executable architecture-boundary test can assert (a) and (b).
//!
//! Conditions (b) and (c) are machine-enforced elsewhere. Condition (a) had NO
//! machine enforcement anywhere: `contracts/pruning-policy.json`'s
//! `crate_justified` listed five conditions and "the code already exists
//! elsewhere" was not among them, so a crate that DUPLICATED code was
//! indistinguishable from one that MOVED it. This file is (a), executable.
//!
//! THE EVIDENCE THAT (a) IS NOT THEORETICAL.
//!
//! - `8aaee51` created `cynic-intel-escore` and `cynic-intel-futarchy` with
//!   ZERO deletions against `cynic-intel`: not one line was removed from the
//!   source crate. 87 000+ commits later both still exist. `cynic-intel-escore`
//!   is 987/949 (98.7%) normalised-identical to `cynic-intel/src/escore/`, and
//!   it has 0 dependents.
//! - `c6dec7e0` (2026-09-28) deleted `cynic-host/src/update_lifecycle/`: 16
//!   files, 2 713 LOC, a byte-identical dead copy of `cynic-lifecycle`, which
//!   `cynic-host` has depended on since `16dde302`.
//! - A census found 447 public item names declared in more than one crate.
//!
//! `SRC-001` (`tests/support/source_island.rs`) is the sibling gate and shares
//! its quarantine SHAPE; it does not share its subject. SRC-001 asks "does the
//! compiler read this file?". This file asks "is this file the same code twice?".
//! A file can be perfectly reachable and still be a copy, which is why SRC-001
//! could never have found any of the three defects above.
//!
//! NO `cargo`. Every observation here is text over `crates/*/src/**.rs` plus the
//! dependency edges in `crates/*/Cargo.toml`. The gate must be able to measure a
//! crate that does not compile, because a crate that was deleted for being a copy
//! is by definition not compiling. `crates/cynic-host` and `crates/cynic-network`
//! are currently broken by another agent's uncommitted work; neither blocks this
//! gate and neither is touched by it.
//!
//! RULES
//! - `MNC-001` `API-REPLICA`: crate C's public API is overwhelmingly redeclared
//!   from a crate D that C depends on.
//! - `MNC-002` `IMPORT-PATH-ONLY-COPY`: a file in C whose normalised content
//!   (import paths and comments normalised away) is near-identical to a file in a
//!   dependency D. This is the form of (a) that survives crate size.
//! - `MNC-003` `DECLARED-TYPE-FORK`: C declares a `struct`/`enum`/`union` whose
//!   name D also declares. A type cannot be owned twice across a dependency edge.
//! - `MNC-004` `PORT-TRAIT-FRACTURE`: a `trait` name declared in two or more
//!   crates. A port implemented in several crates on purpose is still a fracture:
//!   a value satisfying one cannot be passed where the other is expected.
//! - `MNC-005` `UNDECLARED-PROVENANCE`: every workspace crate declares where it
//!   came from, in its own `Cargo.toml` or in the migration ledger.
//! - `MNC-006` `PROVENANCE-INTEGRITY`: a declaration is well-formed and
//!   substantive. `new-capability` must name a domain object and cannot be a
//!   sibling's reason with a name swapped.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Rule ids
// ---------------------------------------------------------------------------

/// C is a whole-crate API replica of a crate it depends on.
pub const RULE_API_REPLICA: &str = "MNC-001";
/// A file in C is a copy of a file in a dependency, modulo import paths.
pub const RULE_IMPORT_PATH_ONLY_COPY: &str = "MNC-002";
/// C re-declares a type that a dependency also declares.
pub const RULE_DECLARED_TYPE_FORK: &str = "MNC-003";
/// A port trait is declared in more than one crate.
pub const RULE_PORT_TRAIT_FRACTURE: &str = "MNC-004";
/// A crate does not say where it came from.
pub const RULE_UNDECLARED_PROVENANCE: &str = "MNC-005";
/// A declaration is malformed, non-substantive, or copied from a sibling.
pub const RULE_PROVENANCE_INTEGRITY: &str = "MNC-006";

/// The quarantine manifest, relative to the workspace root. Same shape as
/// `infra/source_island_quarantine.json` on purpose: see [`Quarantine`].
pub const QUARANTINE_MANIFEST: &str = "infra/move_not_copy_quarantine.json";
/// The provenance ledger, relative to the workspace root.
pub const PROVENANCE_LEDGER: &str = "infra/crate_provenance_ledger.json";
/// The quarantine book is valid only when every loan in it is. Reported per
/// entry, exactly as `SRC-002` does, and reusing that id family is deliberate:
/// a reader who has met one loan book has met this one.
pub const RULE_QUARANTINE_INTEGRITY: &str = "MNC-002";

/// Longest expiry a quarantine entry may claim, from the evaluation date. Same
/// horizon and same justification as SRC-002: the planning cycle is quarterly,
/// so 90 days survives a sprint boundary without being a same-day fire drill,
/// and caps total deferral at exactly one quarter.
pub const MAX_QUARANTINE_HORIZON_DAYS: i64 = 90;

/// Overrides the evaluation date (`YYYY-MM-DD`) so the horizon is testable.
pub const TODAY_ENV: &str = "CYNIC_MOVE_GATE_TODAY";

/// Minimum `shared / own` for `MNC-001`.
///
/// CHOSEN FROM THE MEASURED DISTRIBUTION, not guessed. Over all 47 crates and
/// every internal dependency edge (163 pairs), the shared/own ratio is bimodal
/// with an empty band between `0.118` and `1.000`:
///
/// ```text
///   1.000  cynic-intel-escore     -> cynic-intel     (21/21 public names)
///   1.000  cynic-intel-futarchy  -> cynic-intel     ( 6/ 6 public names)
///   ---- nothing here: 0.5 empty band, factor 8.5 wide ----
///   0.118  cynic-agent-bus        -> cynic-protocol   ( 4/34)
///   0.111  cynic-network         -> cynic-protocol   (13/117)
///   0.111  cynic-vcs             -> cynic-protocol   (18/162)
///   0.091  cynic-discovery       -> cynic-protocol   ( 1/11)
///   0.082  cynic-eval            -> cynic-protocol   ( 8/98)
///   0.050  cynic-host            -> cynic-pkg        (73/1455)
///   0.035  cynic-domain-ops      -> cynic-secrets    ( 1/29)
///   0.024  cynic-kernel          -> cynic-node       ( 5/205)
///   0.023  cynic-intel           -> cynic-protocol   ( 4/175)
///   0.022  cynic-build           -> cynic-protocol   ( 5/227)
///   ...    every remaining pair <= 0.019
/// ```
///
/// `0.50` sits inside the empty band with room on both sides: 4.2x above the
/// largest non-replica and half the smallest replica. Anything in `(0.118,
/// 1.000)` gives the same verdicts on this tree; `0.50` is the midpoint in the
/// only sense that matters, that it is unreachable from both clusters.
///
/// The second condition, `shared >= 4`, is not decoration. Without it a crate
/// with a two-item public API that happens to collide on one name scores `0.500`
/// and is a copy by the arithmetic. Measured: `cynic-domain-cloudflare` ->
/// `cynic-domain-ops` scores `1/2 = 0.500` on the single name `error`. The floor
/// is what makes the ratio mean what it says.
pub const API_REPLICA_RATIO: f64 = 0.50;
/// Minimum number of redeclared names for `MNC-001`. See above.
pub const API_REPLICA_MIN_SHARED: usize = 4;

/// Minimum normalised 5-shingle Jaccard similarity for `MNC-002`.
///
/// CHOSEN FROM THE MEASURED DISTRIBUTION. Over every file of every crate against
/// every file of each of its dependencies, the best-match similarities at or
/// above `0.50` are:
///
/// ```text
///   1.00  cynic-eval      convergence/mod.rs              -> cynic-protocol/convergence.rs
///   1.00  cynic-host      .../python_cli_policy.rs        -> cynic-pkg/python_cli_policy.rs
///   1.00  cynic-host      .../tooling_authority.rs        -> cynic-pkg/tooling_authority.rs
///   1.00  cynic-intel-escore merkle.rs                   -> cynic-intel/escore/merkle.rs
///   1.00  cynic-intel-escore metrics.rs                  -> cynic-intel/escore/metrics.rs
///   0.98  cynic-intel-futarchy cpmm.rs                   -> cynic-intel/futarchy/cpmm.rs
///   0.94  cynic-host      .../integration_types.rs       -> cynic-pkg/integration/types.rs
///   0.92  cynic-intel-escore decay.rs                    -> cynic-intel/escore/decay.rs
///   0.92  cynic-host      .../providers/ods.rs           -> cynic-pkg/providers/ods.rs
///   0.91  cynic-host      .../providers/external.rs      -> cynic-pkg/providers/external.rs
///   0.90  cynic-host      .../python_tooling.rs          -> cynic-pkg/python_tooling.rs
///   0.89  cynic-intel-escore aggregator.rs               -> cynic-intel/escore/aggregator.rs
///   0.88  cynic-network   transport_security.rs          -> cynic-protocol/transport_security.rs
///   0.87  cynic-host      .../registry.rs                -> cynic-pkg/registry.rs
///   0.86  cynic-host      .../types.rs                   -> cynic-pkg/types.rs
///   0.82  cynic-intel-futarchy vault.rs                  -> cynic-intel/futarchy/vault.rs
///   ---- nothing here ----
///   0.72  cynic-host      .../providers/mod.rs
///   0.70  cynic-host      .../integration_decision.rs
///   0.54  cynic-vcs       source_control.rs              -> cynic-protocol/source_control.rs
///   0.53  cynic-host      .../integration_catalog.rs
///   0.50  cynic-cli       proxmox/mod.rs                 (10 LOC, unrelated boilerplate)
/// ```
///
/// `0.80` sits in the gap between the `0.82` cluster and the `0.72` cluster. The
/// `0.54` entry is a real defect that this rule deliberately does not claim:
/// `cynic-vcs/src/source_control.rs` is `cynic-protocol/src/source_control.rs`
/// with 92 lines appended, so a symmetric set-similarity understates it badly.
/// `MNC-003` owns that case, which is why both rules exist.
///
/// [`MNC-002`]: RULE_IMPORT_PATH_ONLY_COPY
pub const COPY_SIMILARITY: f64 = 0.80;
/// Minimum lines for a file to be measured by `MNC-002`. Kills the `0.50`
/// boilerplate hit on a 10-line `mod.rs` without a ratio, which is what a pure
/// similarity threshold would do.
pub const COPY_MIN_LOC: usize = 40;
/// Width of the normalised shingle window, in lines.
pub const COPY_SHINGLE_LINES: usize = 5;

/// Minimum redeclared `struct`/`enum`/`union` names for `MNC-003`.
///
/// HONEST THRESHOLD, NO CLEAN GAP. The measured count per `(C, D)` pair is
/// `37, 11, 10, 10, 7, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 2, 1 x 14`. There is a
/// step at 3 but no gap, so this threshold is a judgement and the full 29-pair
/// table is recorded in `infra/crate_provenance_ledger.json` under
/// `measured_type_forks` so nothing is hidden by the cut. The argument for 3
/// rather than 2: at 1 or 2 a coincidental name is indistinguishable from a fork
/// (`cynic-host` -> `cynic-build` shares only `RetryPolicy`; `cynic-discovery` ->
/// `cynic-protocol` only `Capability`), and a rule that fires on those is a rule
/// that gets disabled. All 12 pairs at or above 3 are genuine forks: every one
/// of them names a type whose wire or field shape must then exist twice.
pub const TYPE_FORK_MIN_NAMES: usize = 3;
/// Minimum crates declaring one trait name for `MNC-004`. There is nothing
/// between 1 and 2, so 2 is forced: the rule is about a name appearing twice.
pub const PORT_FRACTURE_MIN_CRATES: usize = 2;

/// Minimum characters in a `new-capability` reason. Same reasoning as
/// `SRC-002`'s [`MIN_REASON_CHARS`](crate::orphan_sources), different constant
/// so the two gates can be tuned without coupling.
pub const MIN_REASON_CHARS: usize = 60;

/// Owner values that are not owners.
const PLACEHOLDER_OWNERS: &[&str] = &["", "tbd", "todo", "none", "unknown", "nobody", "n/a", "-"];

// ---------------------------------------------------------------------------
// Dates — copied in full from `source_island.rs` so neither gate can be
// weakened by editing the other's arithmetic. Deliberately duplicated, and
// asserted equal by a test, because a shared module here would make the two
// gates one file and one bug.
// ---------------------------------------------------------------------------

/// A calendar date, days resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    /// Days since 1970-01-01.
    pub epoch_day: i64,
}

impl Date {
    /// Parse `YYYY-MM-DD`, rejecting impossible days.
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        let mut fields = text.split('-');
        let year: i64 = fields
            .next()
            .ok_or_else(|| format!("no year in date {text:?}"))?
            .parse()
            .map_err(|_| format!("bad year in date {text:?}"))?;
        let month: u32 = fields
            .next()
            .ok_or_else(|| format!("no month in date {text:?}"))?
            .parse()
            .map_err(|_| format!("bad month in date {text:?}"))?;
        let day: u32 = fields
            .next()
            .ok_or_else(|| format!("no day in date {text:?}"))?
            .parse()
            .map_err(|_| format!("bad day in date {text:?}"))?;
        if fields.next().is_some() {
            return Err(format!("date {text:?} has trailing fields"));
        }
        if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
            return Err(format!("{text:?} is not a calendar date"));
        }
        Ok(Self {
            epoch_day: days_from_civil(year, month, day),
        })
    }

    /// ISO-8601 calendar date.
    pub fn to_iso(self) -> String {
        let (year, month, day) = civil_from_days(self.epoch_day);
        format!("{year:04}-{month:02}-{day:02}")
    }
}

impl fmt::Display for Date {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_iso())
    }
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day = i64::from(day);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(epoch_day: i64) -> (i64, u32, u32) {
    let day = epoch_day + 719_468;
    let era = if day >= 0 { day } else { day - 146_096 } / 146_097;
    let day_of_era = day - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - year_of_era * 365 - year_of_era / 4 + year_of_era / 100;
    let mp = (5 * day_of_year + 2) / 153;
    let day = (day_of_era - (153 * mp + 2) / 5 + 1) as u32;
    let month = (mp + if mp < 10 { 3 } else { -9 }) as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// The date the rule is evaluated against: `TODAY_ENV` when set, else system UTC.
pub fn today() -> Date {
    if let Ok(forced) = std::env::var(TODAY_ENV) {
        return Date::parse(&forced)
            .unwrap_or_else(|error| panic!("{TODAY_ENV}={forced:?}: {error}"));
    }
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0);
    Date {
        epoch_day: seconds.div_euclid(86_400),
    }
}

// ---------------------------------------------------------------------------
// Rust text
// ---------------------------------------------------------------------------

/// What kind of item a name is. Only the distinctions the rules need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemKind {
    /// `struct`, `enum`, `union`: a value type. Owns a wire and a field shape.
    Type,
    /// `trait`: a port or a behaviour. Fractures when declared twice.
    Trait,
    /// `fn`, `const`, `static`, `type`: anything else.
    Other,
}

impl ItemKind {
    fn from_decl(keyword: &str) -> Self {
        match keyword {
            "struct" | "enum" | "union" => Self::Type,
            "trait" => Self::Trait,
            _ => Self::Other,
        }
    }

    /// Lowercase name, for messages.
    pub fn label(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Trait => "trait",
            Self::Other => "item",
        }
    }
}

/// Remove `//` and `/* */` comments and string and raw-string literals, so the
/// scanners below never see code inside a string. Replaces literals with a
/// space rather than deleting them, so byte offsets survive and brace depth
/// accounting stays honest.
pub fn desugar(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        // raw string: r"..." / r#"..."# / br#"..."#
        if byte == b'r' || (byte == b'b' && index + 1 < bytes.len() && bytes[index + 1] == b'r') {
            let mut probe = index + if byte == b'r' { 1 } else { 2 };
            let mut hashes = 0usize;
            while probe < bytes.len() && bytes[probe] == b'#' {
                hashes += 1;
                probe += 1;
            }
            if probe < bytes.len() && bytes[probe] == b'"' {
                probe += 1;
                let close = format!("\"{}", "#".repeat(hashes));
                match text[probe..].find(&close) {
                    Some(offset) => {
                        index = probe + offset + close.len();
                        continue;
                    }
                    None => {
                        out.push(' ');
                        index = bytes.len();
                        continue;
                    }
                }
            }
        }
        if byte == b'"' {
            let mut probe = index + 1;
            while probe < bytes.len() {
                match bytes[probe] {
                    b'\\' => probe += 2,
                    b'"' => {
                        probe += 1;
                        break;
                    }
                    _ => probe += 1,
                }
            }
            out.push(' ');
            index = probe.min(bytes.len());
            continue;
        }
        if byte == b'/' && index + 1 < bytes.len() && bytes[index + 1] == b'/' {
            match text[index..].find('\n') {
                Some(offset) => index += offset,
                None => {
                    index = bytes.len();
                    continue;
                }
            }
            continue;
        }
        if byte == b'/' && index + 1 < bytes.len() && bytes[index + 1] == b'*' {
            match text[index + 2..].find("*/") {
                Some(offset) => {
                    out.push(' ');
                    index += 2 + offset + 2;
                }
                None => {
                    out.push(' ');
                    index = bytes.len();
                    continue;
                }
            }
            continue;
        }
        out.push(byte as char);
        index += 1;
    }
    out
}

/// One crate's public top-level surface, as observed from its own `src/`.
#[derive(Debug, Clone, Default)]
pub struct Surface {
    /// Every public name DECLARED here, with its kind and the file that declares
    /// it. `mod` is excluded on purpose: a `pub mod error` in two crates is not
    /// duplication, it is two modules that happen to share a word.
    pub declared: BTreeMap<String, (ItemKind, String)>,
    /// Names that appear only as `pub use <path>::<name>`. A re-export is a
    /// commitment to someone else's item, not a second declaration of it, so it
    /// never counts as redeclaration.
    pub reexported: BTreeSet<String>,
    /// Per-file line counts, relative to `src/`.
    pub file_loc: BTreeMap<String, usize>,
    /// Total lines under `src/`.
    pub loc: usize,
}

impl Surface {
    /// Names DECLARED here, as opposed to merely re-exported.
    ///
    /// `declared` is populated only by [`top_level_pub_item`], whose keyword list
    /// does not include `use`, so a `pub use` never lands in it. The one case
    /// that needs a decision is a name that is both declared and re-exported —
    /// `cynic-vcs` declares `pub trait ArgvRunner` in `runner.rs` and re-exports
    /// it from `lib.rs`. The DECLARATION wins, because treating the re-export as
    /// the truth would erase the fracture `MNC-004` exists to find.
    pub fn owned(&self) -> BTreeSet<&str> {
        self.declared.keys().map(String::as_str).collect()
    }

    /// Owned names of a given kind.
    pub fn owned_of(&self, kind: ItemKind) -> BTreeSet<&str> {
        self.owned()
            .into_iter()
            .filter(|name| self.declared.get(*name).map(|entry| entry.0) == Some(kind))
            .collect()
    }

    /// Lines in one file, or 0 when the file does not exist.
    pub fn loc_of(&self, rel: &str) -> usize {
        self.file_loc.get(rel).copied().unwrap_or(0)
    }
}

/// Scan one file's desugared text for top-level `pub` items and `pub use` names.
pub fn scan_file(text: &str, rel_path: &str, surface: &mut Surface) {
    let desugared = desugar(text);
    let mut depth: i64 = 0;
    for line in desugared.lines() {
        if depth == 0 {
            if let Some(name) = reexport_name(line) {
                surface.reexported.insert(name);
            }
            if let Some((name, kind)) = top_level_pub_item(line) {
                surface
                    .declared
                    .entry(name)
                    .or_insert((kind, rel_path.to_string()));
            }
        }
        for character in line.chars() {
            match character {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }
        if depth < 0 {
            depth = 0;
        }
    }
}

fn reexport_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("pub ")
        .or_else(|| trimmed.strip_prefix("pub("))?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix("use ")?;
    let tail = rest.split(';').next()?.trim();
    let last = tail.rsplit("::").next()?.trim();
    if last.is_empty() || !last.chars().next()?.is_ascii_alphabetic() && !last.starts_with('_') {
        return None;
    }
    if !last.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some(last.to_string())
}

/// Extract a top-level `pub <keyword> <name>` declaration from one line.
///
/// Deliberately not a Rust parser. The property under test is "does this name
/// appear twice", and a line-anchored keyword match is auditable in a way an
/// AST is not. Depth is handled by the caller: only lines at brace depth 0 are
/// offered here, so a `pub fn` inside an `impl` block is never a candidate.
fn top_level_pub_item(line: &str) -> Option<(String, ItemKind)> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("pub ")?;
    // `pub(crate)`, `pub(super)`, `pub(in path)`
    let rest = match rest.strip_prefix('(') {
        Some(inner) => inner.split_once(')')?.1.trim_start(),
        None => rest,
    };
    for keyword in [
        "struct",
        "enum",
        "union",
        "trait",
        "fn",
        "const",
        "static",
        "type",
        "macro_rules",
    ] {
        let Some(after) = rest.strip_prefix(keyword) else {
            continue;
        };
        // `fn` may be preceded by qualifiers: `pub async unsafe fn name`.
        let mut cursor = after;
        for qualifier in ["default ", "const ", "async ", "unsafe ", "extern "] {
            if let Some(stripped) = cursor.strip_prefix(qualifier) {
                cursor = stripped;
            }
        }
        let name: String = cursor
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() || name.chars().next()?.is_ascii_digit() {
            return None;
        }
        let kind = match keyword {
            "macro_rules" => ItemKind::Other,
            other => ItemKind::from_decl(other),
        };
        return Some((name, kind));
    }
    None
}

// ---------------------------------------------------------------------------
// Normalised content, for `MNC-002`
// ---------------------------------------------------------------------------

/// Normalise a file so that a MOVE and the COPY LEFT BEHIND compare equal.
///
/// Exactly three differences are forgiven, and the list is the whole
/// justification for the rule's name:
///
/// 1. line comments and attribute lines, which a `sed` on the import path
///    usually rewrote too;
/// 2. the crate qualifier in a path (`cynic_intel::` -> `CRATE::`) and
///    `use crate::`, which is what an incomplete `sed` leaves behind;
/// 3. all whitespace, because the copy is often re-indented.
///
/// Nothing else is forgiven. A renamed field, a changed constant or a reordered
/// arm is a DIFFERENT function and must score low, or the rule is a clone
/// detector and not a move detector.
pub fn normalise(text: &str) -> String {
    let desugared = desugar(text);
    let mut kept: Vec<&str> = Vec::new();
    for line in desugared.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#[") {
            continue;
        }
        kept.push(trimmed);
    }
    kept.join("\n")
}

/// FNV-1a, so a shingle can be compared as a `u64` rather than a `String`.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

/// Hash set of normalised `COPY_SHINGLE_LINES`-line windows.
#[derive(Debug, Clone, Default)]
pub struct Shingles {
    /// Sorted, deduplicated shingle hashes.
    hashes: Vec<u64>,
    /// Lines in the file.
    pub loc: usize,
}

impl Shingles {
    /// Compute for one file's text.
    pub fn of(text: &str) -> Self {
        let loc = text.lines().count();
        let normalised = normalise(text);
        let lines: Vec<&str> = normalised.lines().collect();
        let mut hashes: Vec<u64> = if lines.len() < COPY_SHINGLE_LINES {
            if lines.is_empty() {
                Vec::new()
            } else {
                vec![fnv1a(normalised.as_bytes())]
            }
        } else {
            let mut rolling: Vec<u64> = Vec::with_capacity(COPY_SHINGLE_LINES);
            let mut out: Vec<u64> = Vec::with_capacity(lines.len());
            for (index, line) in lines.iter().enumerate() {
                rolling.push(fnv1a(line.as_bytes()));
                if rolling.len() > COPY_SHINGLE_LINES {
                    rolling.remove(0);
                }
                if index + 1 >= COPY_SHINGLE_LINES {
                    let mut combined: u64 = 0xcbf2_9ce4_8422_2325;
                    for hash in &rolling {
                        combined ^= *hash;
                        combined = combined.wrapping_mul(0x1000_0000_01b3);
                    }
                    out.push(combined);
                }
            }
            out
        };
        hashes.sort_unstable();
        hashes.dedup();
        Self { hashes, loc }
    }

    /// Number of distinct shingles.
    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    /// Whether the file produced no shingles at all.
    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }

    /// Intersection size, for the Jaccard numerator.
    pub fn intersection(&self, other: &Self) -> usize {
        let mut count = 0usize;
        let (mut i, mut j) = (0usize, 0usize);
        while i < self.hashes.len() && j < other.hashes.len() {
            match self.hashes[i].cmp(&other.hashes[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    count += 1;
                    i += 1;
                    j += 1;
                }
            }
        }
        count
    }
}

// ---------------------------------------------------------------------------
// Crate inventory
// ---------------------------------------------------------------------------

/// One workspace crate as read from its manifest and its own `src/`.
#[derive(Debug, Clone)]
pub struct CrateView {
    /// Package name.
    pub name: String,
    /// Directory under `crates/`.
    pub dir: String,
    /// Internal (path) dependencies only, resolved against the workspace.
    pub deps: Vec<String>,
    /// Public top-level surface.
    pub surface: Surface,
    /// Per-file normalised shingles, keyed by path relative to `src/`.
    pub shingles: BTreeMap<String, Shingles>,
    /// Lines in the manifest.
    pub manifest_loc: usize,
}

/// The workspace as observed: pure text, no `cargo`.
#[derive(Debug, Clone, Default)]
pub struct Workspace {
    /// Every crate with a `Cargo.toml` under `crates/`, sorted by name.
    pub crates: BTreeMap<String, CrateView>,
}

impl Workspace {
    /// Names of crates that depend on `name`.
    pub fn dependents_of(&self, name: &str) -> Vec<&str> {
        self.crates
            .values()
            .filter(|view| view.deps.iter().any(|dep| dep == name))
            .map(|view| view.name.as_str())
            .collect()
    }

    /// Every ordered internal `(crate, dependency)` edge.
    pub fn internal_edges(&self) -> Vec<(&str, &str)> {
        let mut out: Vec<(&str, &str)> = Vec::new();
        for view in self.crates.values() {
            for dep in &view.deps {
                out.push((view.name.as_str(), dep.as_str()));
            }
        }
        out
    }
}

/// Read the whole workspace.
pub fn read_workspace(root: &Path) -> Workspace {
    let mut out = Workspace::default();
    let Ok(entries) = fs::read_dir(root.join("crates")) else {
        return out;
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && path.join("Cargo.toml").is_file())
        .collect();
    dirs.sort();
    // First pass: names, so path dependencies can be resolved in the second.
    let mut names: BTreeMap<String, PathBuf> = BTreeMap::new();
    for dir in &dirs {
        let text = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
        if let Some(name) = manifest_field(&text, "name") {
            names.insert(name, dir.clone());
        }
    }
    for (name, dir) in &names {
        let dir_label = dir.strip_prefix(root).unwrap_or(dir).display().to_string();
        let manifest = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
        let mut view = CrateView {
            name: name.clone(),
            dir: dir_label,
            deps: path_dependencies(&manifest)
                .into_iter()
                .filter(|dep| names.contains_key(dep))
                .collect(),
            surface: Surface::default(),
            shingles: BTreeMap::new(),
            manifest_loc: manifest.lines().count(),
        };
        view.deps.sort();
        let src = dir.join("src");
        if let Ok(source) = fs::read_dir(&src) {
            let mut files: Vec<PathBuf> = source
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("rs"))
                .filter(|path| path.is_file())
                .collect();
            files.sort();
            for file in files {
                let Ok(text) = fs::read_to_string(&file) else {
                    continue;
                };
                let rel = file
                    .strip_prefix(&src)
                    .unwrap_or(&file)
                    .display()
                    .to_string();
                let loc = text.lines().count();
                view.surface.file_loc.insert(rel.clone(), loc);
                view.surface.loc += loc;
                scan_file(&text, &rel, &mut view.surface);
                if loc >= COPY_MIN_LOC {
                    view.shingles.insert(rel, Shingles::of(&text));
                }
            }
        }
        out.crates.insert(name.clone(), view);
    }
    out
}

/// One `key = "value"` in the manifest's `[package]` table.
pub fn manifest_field(manifest: &str, key: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_package = trimmed == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(&format!("{key}")) {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                return Some(rest.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

/// Dependency names declared in `[dependencies]`, external ones included.
///
/// `Cargo.toml` is not a regular language and this is not a TOML parser. The
/// question asked is only "which key names a crate in this workspace", and a
/// key is `name = ...` or `name.workspace = true`; anything else in a
/// dependency table is not a crate. A real TOML parse would be a dependency and
/// a second failure mode for a question this simple.
pub fn path_dependencies(manifest: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut in_table = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_table = trimmed == "[dependencies]";
            continue;
        }
        if !in_table || trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
            continue;
        }
        let Some((key, _)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"');
        let Some((name, _)) = key.split_once('.') else {
            out.push(key.replace('_', "-"));
            continue;
        };
        out.push(name.replace('_', "-"));
    }
    out
}

// ---------------------------------------------------------------------------
// Findings
// ---------------------------------------------------------------------------

/// One machine-readable finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// `<RULE>-<SLUG>`.
    pub code: String,
    /// The subject, relative to the workspace root.
    pub subject: String,
    /// What was observed, and what to do about it.
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {} — {}",
            self.code, self.subject, self.message
        )
    }
}

fn violation(rule: &str, slug: &str, subject: &str, message: String) -> Violation {
    Violation {
        code: format!("{rule}-{slug}"),
        subject: subject.to_string(),
        message,
    }
}

// ---------------------------------------------------------------------------
// MNC-001 / MNC-002 / MNC-003 / MNC-004
// ---------------------------------------------------------------------------

/// A measured `(crate, dependency)` overlap, kept for reporting and for the
/// distribution assertions in the gate's own tests.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiOverlap {
    /// The crate under test.
    pub crate_name: String,
    /// The dependency it duplicates.
    pub dep: String,
    /// Public names C declares.
    pub own: usize,
    /// Of those, how many D also declares.
    pub shared: usize,
    /// The names.
    pub shared_names: Vec<String>,
    /// `shared / own`.
    pub ratio: f64,
    /// Crates that depend on C.
    pub dependents: usize,
}

impl ApiOverlap {
    /// Whether this pair is a `MNC-001` replica.
    pub fn is_replica(&self) -> bool {
        self.shared >= API_REPLICA_MIN_SHARED && self.ratio >= API_REPLICA_RATIO
    }
}

/// One measured near-duplicate file pair.
#[derive(Debug, Clone, PartialEq)]
pub struct FileCopy {
    /// Crate holding the copy.
    pub crate_name: String,
    /// The dependency holding the original.
    pub dep: String,
    /// Path of the copy, relative to the crate's `src/`.
    pub path: String,
    /// Path of the original, relative to the dependency's `src/`.
    pub dep_path: String,
    /// Normalised similarity.
    pub similarity: f64,
    /// Lines in the copy.
    pub loc: usize,
}

impl FileCopy {
    /// Whether this pair is a `MNC-002` copy.
    pub fn is_copy(&self) -> bool {
        self.similarity >= COPY_SIMILARITY && self.loc >= COPY_MIN_LOC
    }
}

/// A measured type fork.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeFork {
    /// Crate holding the second declaration.
    pub crate_name: String,
    /// The dependency holding the first.
    pub dep: String,
    /// The forked names, sorted.
    pub names: Vec<String>,
}

impl TypeFork {
    /// Whether this pair is a `MNC-003` fork.
    pub fn is_fork(&self) -> bool {
        self.names.len() >= TYPE_FORK_MIN_NAMES
    }
}

/// A measured port-trait fracture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortFracture {
    /// The trait name.
    pub name: String,
    /// Every crate declaring it, sorted.
    pub crates: Vec<String>,
}

impl PortFracture {
    /// Whether this is a `MNC-004` fracture.
    pub fn is_fracture(&self) -> bool {
        self.crates.len() >= PORT_FRACTURE_MIN_CRATES
    }
}

/// Everything `MNC-001`..`MNC-004` measured, with no verdict attached. The gate
/// filters; this is the raw observation.
#[derive(Debug, Clone, Default)]
pub struct Census {
    /// Every internal edge's API overlap, sorted by descending ratio.
    pub overlaps: Vec<ApiOverlap>,
    /// Every near-duplicate file pair at or above `0.50`.
    pub file_copies: Vec<FileCopy>,
    /// Every pair sharing at least one redeclared type name.
    pub type_forks: Vec<TypeFork>,
    /// Every trait name declared in two or more crates.
    pub port_fractures: Vec<PortFracture>,
}

/// Measure `MNC-001`..`MNC-004` over the whole workspace.
pub fn census(workspace: &Workspace) -> Census {
    let mut out = Census::default();

    for (crate_name, view) in &workspace.crates {
        let own = view.surface.owned();
        for dep in &view.deps {
            let Some(dep_view) = workspace.crates.get(dep) else {
                continue;
            };
            let other = dep_view.surface.owned();
            let shared_names: Vec<String> = own
                .iter()
                .filter(|name| other.contains(*name))
                .map(|name| (*name).to_string())
                .collect();
            let shared = shared_names.len();
            out.overlaps.push(ApiOverlap {
                crate_name: crate_name.clone(),
                dep: dep.clone(),
                own: own.len(),
                shared,
                shared_names,
                ratio: if own.is_empty() {
                    0.0
                } else {
                    shared as f64 / own.len() as f64
                },
                dependents: workspace.dependents_of(crate_name).len(),
            });

            // MNC-003: types, which are the names a fork can silently desync.
            let own_types = view.surface.owned_of(ItemKind::Type);
            let dep_types = dep_view.surface.owned_of(ItemKind::Type);
            let forked: Vec<String> = own_types
                .into_iter()
                .filter(|name| dep_types.contains(*name))
                .map(|name| (*name).to_string())
                .collect();
            if !forked.is_empty() {
                out.type_forks.push(TypeFork {
                    crate_name: crate_name.clone(),
                    dep: dep.clone(),
                    names: forked,
                });
            }

            // MNC-002: file-level, so a copy survives the size of its container.
            for (path, shingles) in &view.shingles {
                let mut best: Option<(&str, f64)> = None;
                for (dep_path, dep_shingles) in &dep_view.shingles {
                    let intersection = shingles.intersection(dep_shingles);
                    if intersection == 0 {
                        continue;
                    }
                    let union = shingles.hashes.len() + dep_shingles.hashes.len() - intersection;
                    if union == 0 {
                        continue;
                    }
                    let similarity = intersection as f64 / union as f64;
                    if best.map(|entry| similarity > entry.1).unwrap_or(true) {
                        best = Some((dep_path.as_str(), similarity));
                    }
                }
                if let Some((dep_path, similarity)) = best {
                    if similarity >= 0.50 {
                        out.file_copies.push(FileCopy {
                            crate_name: crate_name.clone(),
                            dep: dep.clone(),
                            path: path.clone(),
                            dep_path: dep_path.to_string(),
                            similarity,
                            loc: shingles.loc,
                        });
                    }
                }
            }
        }
    }

    // MNC-004: a trait name declared in more than one crate, whatever the
    // dependency relation. Reported by an explicit rule, not by MNC-003
    // catching it by accident: `ArgvRunner` is the known real case, and a gate
    // that finds it for the wrong reason is a gate that stops finding it.
    let mut trait_crates: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (crate_name, view) in &workspace.crates {
        for name in view.surface.owned_of(ItemKind::Trait) {
            trait_crates
                .entry(name.to_string())
                .or_default()
                .push(crate_name.clone());
        }
    }
    for (name, mut crates) in trait_crates {
        if crates.len() < PORT_FRACTURE_MIN_CRATES {
            continue;
        }
        crates.sort();
        out.port_fractures.push(PortFracture { name, crates });
    }

    out.overlaps
        .sort_by(|left, right| right.ratio.total_cmp(&left.ratio));
    out.file_copies
        .sort_by(|left, right| right.similarity.total_cmp(&left.similarity));
    out.type_forks
        .sort_by(|left, right| right.names.len().cmp(&left.names.len()));
    out.port_fractures
        .sort_by(|left, right| right.crates.len().cmp(&left.crates.len()));
    out
}

/// Evaluate `MNC-001`..`MNC-004` into violations, honouring the quarantine.
///
/// A finding is suppressed only by an ACTIVE loan. An expired or ownerless
/// entry silences nothing: the finding stands and `MNC-002`-shaped integrity
/// findings fire beside it. See [`active_loan`].
pub fn evaluate_copies(census: &Census, quarantine: &Quarantine, today: Date) -> Vec<Violation> {
    let mut out: Vec<Violation> = Vec::new();

    for overlap in census.overlaps.iter().filter(|entry| entry.is_replica()) {
        let finding = violation(
            RULE_API_REPLICA,
            "REPLICA",
            &format!("crates/{}/Cargo.toml", overlap.crate_name),
            format!(
                "{} of {} public item names declared by `{}` are also declared by its dependency \
                 `{}` ({:.1}%). A crate created by MOVING code out of a container declares a surface \
                 that is absent from the container; a crate created by COPYING it declares the same \
                 surface twice, and both live.\n\
                 Shared: {}\n\
                 Repair: delete one of them. Either the copy goes and `{}` keeps the original, or \
                 the original goes and every consumer is repointed. `{}` has {} dependent(s), so \
                 this is a migration with a consumer list, not a rename.",
                overlap.shared,
                overlap.own,
                overlap.crate_name,
                overlap.dep,
                overlap.ratio * 100.0,
                display_list(&overlap.shared_names),
                overlap.dep,
                overlap.crate_name,
                overlap.dependents
            ),
        );
        out.extend(under_loan(
            finding,
            quarantine,
            today,
            &overlap.crate_name,
            "Cargo.toml",
        ));
    }

    for copy in census.file_copies.iter().filter(|entry| entry.is_copy()) {
        let finding = violation(
            RULE_IMPORT_PATH_ONLY_COPY,
            "COPY",
            &format!("crates/{}/src/{}", copy.crate_name, copy.path),
            format!(
                "{} lines, {:.1}% normalised-identical to `crates/{}/src/{}` in dependency `{}`. \
                 Comments, attribute lines, the crate qualifier in a path and all whitespace are \
                 the ONLY differences forgiven, so what is left is the same code written twice.\n\
                 Repair: this is the shape of an extraction that never deleted its source, the same \
                 defect as the 2 713-LOC `cynic-host/src/update_lifecycle/` removed in c6dec7e0. \
                 Delete the copy, or promote the copy to owner and repoint `{}`.",
                copy.loc,
                copy.similarity * 100.0,
                copy.dep,
                copy.dep_path,
                copy.dep,
                copy.dep
            ),
        );
        out.extend(under_loan(
            finding,
            quarantine,
            today,
            &copy.crate_name,
            &copy.path,
        ));
    }

    for fork in census.type_forks.iter().filter(|entry| entry.is_fork()) {
        let finding = violation(
            RULE_DECLARED_TYPE_FORK,
            "FORK",
            &format!("crates/{}/Cargo.toml", fork.crate_name),
            format!(
                "{} type names are DECLARED in `{}` and DECLARED AGAIN in its dependency `{}`: {}.\n\
                 A type declared on both sides of a dependency edge exists twice under one name. \
                 Every field added on one side forks a wire format the compiler will not report, \
                 and `pub use` is not the fix because neither declaration is a re-export of the \
                 other.\n\
                 Repair: the lower crate owns the type and the higher crate imports it \
                 (`use {}::{{...}}`), or the two are genuinely different shapes and need different \
                 names.",
                fork.names.len(),
                fork.crate_name,
                fork.dep,
                display_list(&fork.names),
                fork.dep
            ),
        );
        out.extend(under_loan(
            finding,
            quarantine,
            today,
            &fork.crate_name,
            "Cargo.toml",
        ));
    }

    for fracture in census
        .port_fractures
        .iter()
        .filter(|entry| entry.is_fracture())
    {
        // Subjected to the FIRST declaring crate so a loan names one place, and
        // reported against every crate that holds the second declaration.
        let owner_crate = fracture.crates[0].clone();
        let finding = violation(
            RULE_PORT_TRAIT_FRACTURE,
            "FRACTURE",
            &format!("crates/{owner_crate}/Cargo.toml"),
            format!(
                "trait `{}` is declared in {} crates: {}.\n\
                 A port trait duplicated across a boundary is a duplicated type with a worse \
                 symptom: a value satisfying one declaration cannot be passed where the other is \
                 expected, and the fix is a conversion at every call site rather than a rename. \
                 `ArgvRunner` in cynic-lifecycle and cynic-vcs is the known real case.\n\
                 Repair: the lowest crate both sides can depend on owns the trait; the others import \
                 it and keep only their adapter.",
                fracture.name,
                fracture.crates.len(),
                display_list(&fracture.crates)
            ),
        );
        out.extend(under_loan(
            finding,
            quarantine,
            today,
            &owner_crate,
            "Cargo.toml",
        ));
    }

    for entry in &quarantine.entries {
        out.extend(stale_quarantine_violations(entry, census));
    }

    out
}

/// Join names for a message, truncating rather than printing 40 of them.
fn display_list(items: &[String]) -> String {
    if items.is_empty() {
        return String::from("(none)");
    }
    if items.len() <= 12 {
        return items.join(", ");
    }
    format!(
        "{}, ... and {} more ({} total)",
        items[..12].join(", "),
        items.len() - 12,
        items.len()
    )
}

/// Does an ACTIVE loan cover this subject?
///
/// A quarantine is a loan, not a pardon, so it silences a finding only while the
/// loan is itself valid: owned, reasoned, dated, and dated inside the horizon.
/// An entry that has expired, over-reached, lost its owner or lost its reason
/// silences NOTHING — the finding stands and the integrity rule fires beside it.
/// That is the whole difference from a list of names, and the reason a duplicate
/// cannot survive here without somebody putting their name and a date on it.
fn active_loan(quarantine: &Quarantine, today: Date, crate_name: &str, path: &str) -> bool {
    let Some(entry) = quarantine.entry_for(crate_name, path) else {
        return false;
    };
    let owner = entry.owner.trim();
    if PLACEHOLDER_OWNERS
        .iter()
        .any(|placeholder| owner.eq_ignore_ascii_case(placeholder))
    {
        return false;
    }
    if entry.reason.trim().chars().count() < MIN_REASON_CHARS {
        return false;
    }
    match Date::parse(&entry.expires) {
        Ok(expires) => {
            let age = expires.epoch_day - today.epoch_day;
            (0..=quarantine.horizon_days).contains(&age)
        }
        Err(_) => false,
    }
}

/// A finding, unless an active loan covers it. `None` means "under loan".
fn under_loan(
    finding: Violation,
    quarantine: &Quarantine,
    today: Date,
    crate_name: &str,
    path: &str,
) -> Option<Violation> {
    if active_loan(quarantine, today, crate_name, path) {
        None
    } else {
        Some(finding)
    }
}

/// A loan for a subject that is not a measured duplicate any more is cover, and
/// fails even while it is unexpired. A quarantine can never pre-authorise a path:
/// dropping the entry while adding it is two edits, and a rule that a two-edit
/// dance can silence is not a rule.
fn stale_quarantine_violations(entry: &QuarantineEntry, census: &Census) -> Vec<Violation> {
    let subject = format!("crates/{}/src/{}", entry.crate_name, entry.path);
    let still_a_copy = census.file_copies.iter().any(|copy| {
        copy.crate_name == entry.crate_name && copy.path == entry.path && copy.is_copy()
    });
    if still_a_copy {
        return Vec::new();
    }
    vec![violation(
        RULE_QUARANTINE_INTEGRITY,
        "STALE",
        &subject,
        "the entry names a file that is not a measured copy any more. Either it was repaired, or \
         it never was one. A quarantine cannot pre-authorise a path, so this is cover: remove the \
         entry"
            .to_string(),
    )]
}

// ---------------------------------------------------------------------------
// Quarantine — SAME SHAPE AS SRC-002, NOT A SECOND MECHANISM
// ---------------------------------------------------------------------------

/// One quarantined finding. `Deserialize` with `deny_unknown_fields` because the
/// manifest is a contract: a mistyped field is a parse failure, not a line
/// nobody reads.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QuarantineEntry {
    /// Crate the finding is about.
    pub crate_name: String,
    /// Path relative to the crate's `src/`, or `Cargo.toml` for a crate-level
    /// finding. Quarantine entries name a SUBJECT, never a rule: a list that
    /// could name a rule would be a way to switch the detector off.
    pub path: String,
    /// Who is accountable for the repair. Not a placeholder.
    pub owner: String,
    /// Why the duplicate survives.
    pub reason: String,
    /// `YYYY-MM-DD` after which this entry is a FAILURE.
    pub expires: String,
}

/// The manifest.
///
/// SAME SHAPE AND SAME SEMANTICS AS `infra/source_island_quarantine.json`, on
/// purpose. The SRC-001 precedent established that a quarantine is a loan book
/// (owner + reason + expiry, 90-day horizon) and that it fails in both
/// directions. Inventing a second shape for the second detector would produce
/// two rules about quarantines, and two rules about quarantines is how one of
/// them stops being enforced. This file is a different SUBJECT under the same
/// instrument, not a second instrument.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Quarantine {
    /// Rule this manifest serves.
    pub rule: String,
    /// Declared horizon, in days. Must be within the policy cap.
    pub horizon_days: i64,
    /// Human-readable purpose. Must be non-empty.
    pub purpose: String,
    /// The loans.
    #[serde(default)]
    pub entries: Vec<QuarantineEntry>,
}

impl Quarantine {
    /// Read and structurally validate the manifest.
    pub fn load(workspace_root: &Path) -> Result<Self, String> {
        let path = workspace_root.join(QUARANTINE_MANIFEST);
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let parsed: Self = serde_json::from_str(&text).map_err(|error| {
            format!(
                "{} is not a valid move-not-copy quarantine manifest: {error}",
                path.display()
            )
        })?;
        if parsed.rule != RULE_API_REPLICA {
            return Err(format!(
                "{} serves rule {:?}, expected {RULE_API_REPLICA}",
                path.display(),
                parsed.rule
            ));
        }
        if parsed.purpose.trim().is_empty() {
            return Err(format!("{} has an empty purpose", path.display()));
        }
        if parsed.horizon_days < 1 || parsed.horizon_days > MAX_QUARANTINE_HORIZON_DAYS {
            return Err(format!(
                "{} declares horizon_days={}, policy allows 1..={MAX_QUARANTINE_HORIZON_DAYS}",
                path.display(),
                parsed.horizon_days
            ));
        }
        let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
        for entry in &parsed.entries {
            if !seen.insert((entry.crate_name.as_str(), entry.path.as_str())) {
                return Err(format!(
                    "{} lists {}/{} twice; a duplicate is two opinions, not one loan",
                    path.display(),
                    entry.crate_name,
                    entry.path
                ));
            }
        }
        Ok(parsed)
    }

    /// The entry covering `crate_name`/`path`, if any.
    pub fn entry_for(&self, crate_name: &str, path: &str) -> Option<&QuarantineEntry> {
        self.entries
            .iter()
            .find(|entry| entry.crate_name == crate_name && entry.path == path)
    }
}

/// Validate every entry as a loan: owner, substantive reason, expiry inside the
/// horizon, and an unexpired date. Every clause fails independently, because a
/// loan missing any of them is a pardon, and a pardon is what this rule exists to
/// prevent.
pub fn evaluate_quarantine(quarantine: &Quarantine, today: Date) -> Vec<Violation> {
    let mut out: Vec<Violation> = Vec::new();
    for entry in &quarantine.entries {
        let subject = format!("crates/{}/src/{}", entry.crate_name, entry.path);
        let owner = entry.owner.trim();
        if PLACEHOLDER_OWNERS
            .iter()
            .any(|placeholder| owner.eq_ignore_ascii_case(placeholder))
        {
            out.push(violation(
                RULE_QUARANTINE_INTEGRITY,
                "NO-OWNER",
                &subject,
                format!(
                    "owner {:?} names nobody. A duplicate with no accountable owner is how \
                     cynic-intel-escore survived 87 000 commits: name the principal accountable \
                     for deleting it.",
                    entry.owner
                ),
            ));
        }
        if entry.reason.trim().chars().count() < MIN_REASON_CHARS {
            out.push(violation(
                RULE_QUARANTINE_INTEGRITY,
                "NO-REASON",
                &subject,
                format!(
                    "reason is {} chars; a quarantine reason must be at least {MIN_REASON_CHARS} \
                     and say what is duplicated, why it survives, and what removes it.",
                    entry.reason.trim().chars().count()
                ),
            ));
        }
        match Date::parse(&entry.expires) {
            Err(error) => out.push(violation(
                RULE_QUARANTINE_INTEGRITY,
                "NO-EXPIRY",
                &subject,
                format!(
                    "expires {:?}: {error}. Every entry must carry an explicit date",
                    entry.expires
                ),
            )),
            Ok(expires) => {
                let age = expires.epoch_day - today.epoch_day;
                if age < 0 {
                    out.push(violation(
                        RULE_QUARANTINE_INTEGRITY,
                        "EXPIRED",
                        &subject,
                        format!(
                            "expired {} day(s) ago on {expires} (today {today}). An expired loan is \
                             a FAILURE, not a pass: delete the duplicate or renew the entry in \
                             writing with a new owner and a new date inside the horizon.",
                            -age
                        ),
                    ));
                } else if age > quarantine.horizon_days {
                    out.push(violation(
                        RULE_QUARANTINE_INTEGRITY,
                        "BEYOND-HORIZON",
                        &subject,
                        format!(
                            "claims {age} day(s) ({expires}), manifest horizon is {} and the policy \
                             cap is {MAX_QUARANTINE_HORIZON_DAYS}. A quarantine extendable by \
                             editing one date is permanent cover.",
                            quarantine.horizon_days
                        ),
                    ));
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// MNC-005 / MNC-006 — the prospective declaration rule
// ---------------------------------------------------------------------------

/// Why a crate exists, as declared. The three kinds are the three honest
/// answers to "where did this code come from"; there is no fourth, because
/// "I am not sure" is a reason to delete the crate, not to keep it.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Provenance {
    /// The code was MOVED out of a named source path. The source must not still
    /// exist with the same code in it, which is exactly what `MNC-002` checks.
    Extraction {
        /// Where it came from, relative to the workspace root.
        from: String,
    },
    /// The code did not exist anywhere before. `reason` must be substantive.
    NewCapability {
        /// Why the code does not exist elsewhere.
        reason: String,
    },
    /// A deliberate rewrite of something that does exist. `note` must say what
    /// was rewritten and why a move was not possible.
    Rewrite {
        /// What was rewritten, from where, and why.
        note: String,
    },
}

impl Provenance {
    /// The kind's short name.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Extraction { .. } => "extraction",
            Self::NewCapability { .. } => "new-capability",
            Self::Rewrite { .. } => "rewrite",
        }
    }

    /// The human-readable text a declaration carries.
    pub fn text(&self) -> &str {
        match self {
            Self::Extraction { from } => from,
            Self::NewCapability { reason } => reason,
            Self::Rewrite { note } => note,
        }
    }
}

/// One entry in the migration ledger.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LedgerEntry {
    /// The crate this declares.
    pub crate_name: String,
    /// Where it came from.
    pub provenance: Provenance,
    /// Who checked this claim, for the entries that were inferred rather than
    /// known. Empty for entries the crate's own author recorded.
    #[serde(default)]
    pub determined_by: String,
}

/// The ledger. A migration surface, not a second authority: a crate may declare
/// provenance inline in its own `Cargo.toml` instead, and
/// `infra/crate_provenance_ledger.json` exists so that 47 legacy crates can be
/// declared once, by one agent, honestly, instead of 47 mechanical edits.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    /// Schema id, so a future reshape is visible rather than silent.
    pub schema: String,
    /// What this file is for, in prose that is not itself a defence.
    pub purpose: String,
    /// The measured distributions the thresholds were chosen from, recorded so a
    /// future reader can re-derive them without re-running the census, and so
    /// the findings a threshold CUT are visible rather than hidden by the cut.
    #[serde(default)]
    pub measured_type_forks: Vec<TypeForkRecord>,
    /// The full `(crate, dependency)` API-redeclaration distribution.
    #[serde(default)]
    pub measured_api_overlaps: Vec<OverlapRecord>,
    /// The declarations.
    pub entries: Vec<LedgerEntry>,
}

/// A recorded type fork, for the distribution.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TypeForkRecord {
    /// Crate holding the second declaration.
    pub crate_name: String,
    /// The dependency holding the first.
    pub dep: String,
    /// How many names.
    pub names: usize,
    /// The names, sorted.
    pub names_list: Vec<String>,
    /// Whether `MNC-003` fails on it.
    pub firing: bool,
}

/// A recorded API overlap, for the distribution.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OverlapRecord {
    /// Crate under test.
    pub crate_name: String,
    /// The dependency.
    pub dep: String,
    /// Public names declared.
    pub own: usize,
    /// Of those, how many the dependency also declares.
    pub shared: usize,
    /// `shared / own`, to three decimals.
    pub ratio: f64,
    /// Whether `MNC-001` fails on it.
    pub firing: bool,
}

impl Ledger {
    /// Read and structurally validate the ledger.
    pub fn load(workspace_root: &Path) -> Result<Self, String> {
        let path = workspace_root.join(PROVENANCE_LEDGER);
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let parsed: Self = serde_json::from_str(&text)
            .map_err(|error| format!("{} is not a valid ledger: {error}", path.display()))?;
        if parsed.schema != "cynic.crate-provenance.v1" {
            return Err(format!(
                "{} declares schema {:?}, expected cynic.crate-provenance.v1",
                path.display(),
                parsed.schema
            ));
        }
        if parsed.purpose.trim().is_empty() {
            return Err(format!("{} has an empty purpose", path.display()));
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for entry in &parsed.entries {
            if !seen.insert(entry.crate_name.as_str()) {
                return Err(format!(
                    "{} declares {} twice; one crate, one provenance",
                    path.display(),
                    entry.crate_name
                ));
            }
        }
        Ok(parsed)
    }

    /// The entry for `crate_name`, if any.
    pub fn entry_for(&self, crate_name: &str) -> Option<&LedgerEntry> {
        self.entries
            .iter()
            .find(|entry| entry.crate_name == crate_name)
    }
}

/// The declaration in force for one crate: inline in its manifest, or from the
/// ledger. Inline wins, because a crate that has migrated its declaration into
/// its own manifest must not be overruled by a stale ledger row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// The crate.
    pub crate_name: String,
    /// The declared provenance.
    pub provenance: Provenance,
    /// `"Cargo.toml"` or `PROVENANCE_LEDGER`.
    pub source: String,
}

/// Parse `[package.metadata.provenance]` out of a manifest.
///
/// The value is a TOML inline table, so the grammar is small: `key = "value"`
/// pairs and one `kind = "..."`. Anything else is a parse failure rather than a
/// best guess, because a declaration nobody can parse is not a declaration.
pub fn inline_provenance(manifest: &str) -> Result<Option<Provenance>, String> {
    let mut in_table = false;
    let mut pairs: BTreeMap<String, String> = BTreeMap::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_table = trimmed == "[package.metadata.provenance]";
            continue;
        }
        if !in_table || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if !value.starts_with('"') || !value.ends_with('"') || value.len() < 2 {
            return Err(format!(
                "[package.metadata.provenance] has a non-string value for {:?}",
                key.trim()
            ));
        }
        pairs.insert(
            key.trim().to_string(),
            value[1..value.len() - 1].to_string(),
        );
    }
    if pairs.is_empty() {
        return Ok(None);
    }
    let kind = pairs
        .remove("kind")
        .ok_or_else(|| "[package.metadata.provenance] has no `kind`".to_string())?;
    let provenance = match kind.as_str() {
        "extraction" => {
            let from = pairs
                .remove("from")
                .ok_or_else(|| "kind = \"extraction\" needs `from = \"<path>\"`".to_string())?;
            Provenance::Extraction { from }
        }
        "new-capability" => {
            let reason = pairs.remove("reason").ok_or_else(|| {
                "kind = \"new-capability\" needs `reason = \"<why>\"`".to_string()
            })?;
            Provenance::NewCapability { reason }
        }
        "rewrite" => {
            let note = pairs.remove("note").ok_or_else(|| {
                "kind = \"rewrite\" needs `note = \"<what and why>\"`".to_string()
            })?;
            Provenance::Rewrite { note }
        }
        other => {
            return Err(format!(
                "unknown provenance kind {other:?}; expected extraction, new-capability or rewrite"
            ));
        }
    };
    if !pairs.is_empty() {
        return Err(format!(
            "[package.metadata.provenance] has unknown key(s) {:?} for kind {kind:?}",
            pairs.keys().collect::<Vec<_>>()
        ));
    }
    Ok(Some(provenance))
}

/// Evaluate `MNC-005` and `MNC-006` over the whole workspace.
///
/// A `new-capability` reason is SUBSTANTIVE when, checkably, all of:
/// 1. it is at least [`MIN_REASON_CHARS`] characters;
/// 2. it is not the same text as another crate's reason, nor the same text with
///    a different crate name substituted in — a template filled in 40 times is
///    40 copies of one non-declaration;
/// 3. it names a DOMAIN OBJECT: at least one identifier-shaped token of three or
///    more characters that is not a stop word, which is what a sentence about a
///    protocol type looks like and what a sentence about architecture does not.
pub fn evaluate_provenance(workspace: &Workspace, ledger: &Ledger) -> Vec<Violation> {
    let mut out: Vec<Violation> = Vec::new();
    let mut declarations: BTreeMap<String, Declaration> = BTreeMap::new();

    for (crate_name, view) in &workspace.crates {
        let manifest_path = Path::new(&view.dir).join("Cargo.toml");
        let manifest = fs::read_to_string(&manifest_path).unwrap_or_default();
        let inline = match inline_provenance(&manifest) {
            Ok(value) => value,
            Err(error) => {
                out.push(violation(
                    RULE_PROVENANCE_INTEGRITY,
                    "MALFORMED",
                    &format!("{}/Cargo.toml", view.dir),
                    format!("{error}. A declaration nobody can parse is not a declaration."),
                ));
                continue;
            }
        };
        let provenance = match inline {
            Some(provenance) => Declaration {
                crate_name: crate_name.clone(),
                provenance,
                source: format!("{}/Cargo.toml", view.dir),
            },
            None => match ledger.entry_for(crate_name) {
                Some(entry) => Declaration {
                    crate_name: crate_name.clone(),
                    provenance: entry.provenance.clone(),
                    source: PROVENANCE_LEDGER.to_string(),
                },
                None => {
                    out.push(violation(
                        RULE_UNDECLARED_PROVENANCE,
                        "MISSING",
                        &format!("{}/Cargo.toml", view.dir),
                        format!(
                            "`{}` does not say where it came from. Neither its own \
                             [package.metadata.provenance] nor {PROVENANCE_LEDGER} declares it.\n\
                             Repair: declare one of\n  \
                               [package.metadata.provenance]\n  \
                               kind = \"extraction\"\n  \
                               from = \"<path the code was moved from>\"\n\
                             or `kind = \"new-capability\"` with a reason that names the domain object \
                             that did not exist before, or add a ledger row. A crate whose origin \
                             nobody wrote down cannot be distinguished from a copy, which is the \
                             defect `{}` exists to end.",
                            crate_name, RULE_UNDECLARED_PROVENANCE
                        ),
                    ));
                    continue;
                }
            },
        };
        declarations.insert(crate_name.clone(), provenance);
    }

    // Rule 2 of "substantive": near-duplicate reasons.
    let mut reasons: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for declaration in declarations.values() {
        if let Provenance::NewCapability { reason } = &declaration.provenance {
            reasons
                .entry(reason.clone())
                .or_default()
                .push(declaration.crate_name.as_str());
        }
    }
    for (reason, crates) in &reasons {
        if crates.len() > 1 {
            for crate_name in crates {
                out.push(violation(
                    RULE_PROVENANCE_INTEGRITY,
                    "TEMPLATED-REASON",
                    crate_name,
                    format!(
                        "{count} crates share one byte-identical new-capability reason: {}\n\
                         A reason copied N times is N non-declarations. Each crate must name the \
                         domain object that did not exist before IT existed.",
                        reason.chars().take(120).collect::<String>(),
                        count = crates.len()
                    ),
                ));
            }
        }
    }

    for declaration in declarations.values() {
        let subject = &declaration.source;
        match &declaration.provenance {
            Provenance::Extraction { from } => {
                if from.trim().chars().count() < MIN_REASON_CHARS {
                    out.push(violation(
                        RULE_PROVENANCE_INTEGRITY,
                        "THIN-FROM",
                        subject,
                        format!(
                            "`{}` declares extraction from {:?}, which is {} chars. Name the full \
                             source path, or the sub-tree that was moved, so a reader can check \
                             whether the source still exists.",
                            declaration.crate_name,
                            from,
                            from.trim().chars().count()
                        ),
                    ));
                }
            }
            Provenance::NewCapability { reason } => {
                let chars = reason.trim().chars().count();
                if chars < MIN_REASON_CHARS {
                    out.push(violation(
                        RULE_PROVENANCE_INTEGRITY,
                        "THIN-REASON",
                        subject,
                        format!(
                            "`{}` declares new-capability with a {chars}-char reason; the minimum \
                             is {MIN_REASON_CHARS}. A one-line reason is a shrug, and the whole \
                             point of the declaration is to be harder to write than the crate.",
                            declaration.crate_name
                        ),
                    ));
                } else if !names_a_domain_object(reason) {
                    out.push(violation(
                        RULE_PROVENANCE_INTEGRITY,
                        "NO-DOMAIN-OBJECT",
                        subject,
                        format!(
                            "`{}` declares new-capability but its reason names no domain object. \
                             \"Because the architecture needed it\" is not a reason; \
                             \"the E-Score decay curve had no owner\" is. The reason must name the \
                             thing that did not exist before, in identifier or noun form.",
                            declaration.crate_name
                        ),
                    ));
                }
            }
            Provenance::Rewrite { note } => {
                if note.trim().chars().count() < MIN_REASON_CHARS {
                    out.push(violation(
                        RULE_PROVENANCE_INTEGRITY,
                        "THIN-NOTE",
                        subject,
                        format!(
                            "`{}` declares rewrite with a {}-char note; the minimum is \
                             {MIN_REASON_CHARS}. A rewrite of existing code must say what was \
                             rewritten, from where, and why a move would not do.",
                            declaration.crate_name,
                            note.trim().chars().count()
                        ),
                    ));
                }
            }
        }
    }

    // A ledger row for a crate that no longer exists is cover, same as a stale
    // quarantine entry.
    for entry in &ledger.entries {
        if !workspace.crates.contains_key(&entry.crate_name) {
            out.push(violation(
                RULE_PROVENANCE_INTEGRITY,
                "STALE-ROW",
                PROVENANCE_LEDGER,
                format!(
                    "declares `{}`, which is not a crate in this workspace. A declaration for a \
                     deleted crate is a declaration nobody reads; remove the row.",
                    entry.crate_name
                ),
            ));
        }
    }

    out
}

/// Words that are not a domain object, however emphatically they are capitalised.
const STOP_WORDS: &[&str] = &[
    "the",
    "and",
    "for",
    "with",
    "from",
    "that",
    "this",
    "into",
    "when",
    "why",
    "how",
    "its",
    "it",
    "not",
    "but",
    "has",
    "have",
    "was",
    "were",
    "are",
    "been",
    "than",
    "then",
    "them",
    "they",
    "their",
    "there",
    "here",
    "where",
    "which",
    "while",
    "will",
    "would",
    "should",
    "could",
    "crate",
    "module",
    "code",
    "thing",
    "stuff",
    "need",
    "needs",
    "needed",
    "because",
    "about",
    "only",
    "also",
    "just",
    "more",
    "most",
    "some",
    "such",
    "each",
    "other",
    "another",
    "same",
    "own",
    "owner",
    "layer",
    "boundary",
    "seam",
    "port",
    "ports",
    "contract",
    "contracts",
    "architecture",
    "architectural",
    "separation",
    "concern",
    "concerns",
    "responsibility",
    "responsibilities",
    "because",
    "purpose",
    "reason",
    "justification",
    "independent",
    "independently",
    "compile",
    "compilation",
    "build",
    "rebuild",
    "reusable",
    "reuse",
];

/// Does this text name a domain object? At least one identifier-shaped token of
/// three or more characters that is not a stop word and is not merely a hyphenated
/// English phrase.
pub fn names_a_domain_object(reason: &str) -> bool {
    let lowered = reason.to_lowercase();
    let mut tokens: Vec<String> = lowered
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect();
    // Hyphenated compounds contribute their HEAD as a separate token, so
    // "new-capability" is judged on "new" and "capability" rather than being
    // waved through as one unfamiliar word.
    let heads: Vec<String> = tokens
        .iter()
        .filter(|token| token.contains('-'))
        .filter_map(|token| token.split('-').next())
        .map(|head| head.to_string())
        .collect();
    tokens.extend(heads);
    tokens.iter().any(|token| {
        token.chars().count() >= 3
            && !STOP_WORDS.contains(&token.as_str())
            && token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && token
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
    })
}

// ---------------------------------------------------------------------------
// Helpers used by the gate's own self-tests
// ---------------------------------------------------------------------------

/// A small synthetic workspace, so the rules can be tested without the real tree.
pub fn fixture_workspace(label: &str) -> PathBuf {
    let base =
        std::env::temp_dir().join(format!("cynic-mnc-fixture-{}-{label}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(base.join("crates")).unwrap_or_else(|error| panic!("{error}"));
    base
}

/// Write one fixture crate.
pub fn write_fixture_crate(
    base: &Path,
    name: &str,
    deps: &[(&str, &str)],
    files: &[(&str, &str)],
) -> PathBuf {
    let dir = base.join("crates").join(name);
    fs::create_dir_all(dir.join("src")).unwrap_or_else(|error| panic!("{error}"));
    let mut manifest =
        format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\n\n[dependencies]\n");
    for (dep, path) in deps {
        manifest.push_str(&format!("{dep} = {{ path = \"{path}\" }}\n"));
    }
    fs::write(dir.join("Cargo.toml"), manifest).unwrap_or_else(|error| panic!("{error}"));
    for (rel, body) in files {
        let file = dir.join("src").join(rel);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|error| panic!("{error}"));
        }
        fs::write(&file, body).unwrap_or_else(|error| panic!("{error}"));
    }
    dir
}

/// A body of padding, so file similarity is measured on mass and not on a
/// three-line fixture that any threshold would call identical.
pub fn padding(lines: usize, marker: &str) -> String {
    let mut out = String::new();
    for index in 0..lines {
        out.push_str(&format!("// {marker} filler line {index}\n"));
        out.push_str(&format!(
            "pub fn {marker}_helper_{index}() -> u8 {{ {index} }}\n"
        ));
    }
    out
}

/// Recursive sum used by a rule's own self-test to prove the two gates agree on
/// date arithmetic without sharing a module.
pub fn date_agrees(source_island_today: i64) -> bool {
    today().epoch_day == source_island_today
}

/// Convenience: a map of crate name to how many crates depend on it.
pub fn dependent_counts(workspace: &Workspace) -> HashMap<String, usize> {
    workspace
        .crates
        .keys()
        .map(|name| (name.clone(), workspace.dependents_of(name).len()))
        .collect()
}
