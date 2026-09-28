//! `SRC-001` — no closed include island, workspace-wide, with a quarantine that
//! expires.
//!
//! WHY THIS FILE EXISTS. On 2026-09-28, commit `87df2f96` deleted 12 858 LOC from
//! `crates/cynic-mcp/src/`. The mechanism was not "code was too much". It was:
//!
//! - `crates/cynic-mcp/src/lib.rs` declared exactly one module, `pub mod server;`;
//! - `helpers.rs`, `types.rs` and `proxy.rs` were included by nothing, so the
//!   compiler never read them;
//! - those three attached 40 more files through `include!` and `#[path]`;
//! - the result was a CLOSED ISLAND of 43 files, 12 858 LOC, of which 15 were
//!   `*_contract.rs` files asserting contracts that nothing evaluated.
//!
//! A per-crate quarantine list asserted that island against the computed orphan
//! set, so the *stable wrong* answer was a PASSING test. It held 43 entries for
//! weeks. The note defending it claimed that deleting `handler.rs` "breaks the
//! build, which is how the omission was caught". Every clause was false: the build
//! cannot see a file the compiler never reads, so nothing was ever caught.
//!
//! WHAT THE RULE OBSERVES. Pure text over each crate's own `src/`. No `cargo`, no
//! subprocess, no dependency graph, no compilation unit. Reachability is computed
//! with the same resolver as the orphan ratchet (`tests/support/orphan_sources.rs`)
//! so the two rules can never disagree about what is dead; the difference is
//! WHAT is asked. The orphan ratchet asks "is this file reachable?". This rule
//! asks "is there a file that exists only because an unreadable file points at it?"
//! — a strictly stronger signal, because a member of an island cannot be repaired
//! by adding one `mod` line at the bottom of a file nobody compiles: the wall has
//! to be opened first, and until it is, every member is inert.
//!
//! WHY QUARANTINE AND NOT ZERO TOLERANCE. Zero tolerance would have blocked the
//! `cynic-mcp` deletion itself, and a rule that blocks the repair of its own
//! defect gets deleted with it. So a quarantine exists — but it is a LOAN, not a
//! pardon:
//!
//! 1. every entry names an owner and a reason, or the rule fails;
//! 2. every entry carries an explicit expiry date, or the rule fails;
//! 3. an expiry further out than [`MAX_QUARANTINE_HORIZON_DAYS`] is itself a
//!    failure, so the manifest cannot accumulate unbounded future dates — a
//!    re-quarantine is a fresh, dated, owned decision, never a silent extension;
//! 4. an entry past its expiry is a FAILURE, not a pass: the debt must be paid,
//!    not re-dated;
//! 5. an entry naming a file that is gone, or that is no longer in an island, is
//!    a FAILURE, so the list can never become cover for a repaired or live file.
//!
//! HORIZON: 90 days. Justification: the CYNIC planning cycle is quarterly, so 90
//! days lets a quarantine survive a sprint boundary and a holiday without being
//! a same-day fire drill, while capping the total deferral at exactly one
//! quarter. A defect that survives a full quarter of a sovereign runtime has
//! either been promoted to a declared design (in which case it must be wired, not
//! hidden) or it is dead mass that must be deleted. Both outcomes are reachable
//! inside 90 days; neither is reachable by inaction.
//!
//! RULES
//! - `SRC-001` `CLOSED-INCLUDE-ISLAND`: a file that is only ever attached, by
//!   `include!` or `#[path]`, to a file no compilation root reaches.
//! - `SRC-002` `QUARANTINE-INTEGRITY`: missing owner, missing reason, missing or
//!   over-long expiry, expired entry, stale entry, unknown crate, bad manifest.
//! - `SRC-003` `UNPROVABLE-BUILD-PROSE`: prose in `tests/` or `infra/` that
//!   justifies a defect by promising the build will catch it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::orphan_sources;

/// The island rule. Reported per file, never per list comparison.
pub const RULE_CLOSED_INCLUDE_ISLAND: &str = "SRC-001";
/// The quarantine integrity rule. Reported per entry.
pub const RULE_QUARANTINE_INTEGRITY: &str = "SRC-002";
/// The prose rule.
pub const RULE_UNPROVABLE_BUILD_PROSE: &str = "SRC-003";

/// The quarantine manifest, relative to the workspace root.
pub const QUARANTINE_MANIFEST: &str = "infra/source_island_quarantine.json";

/// Longest expiry a quarantine entry may claim, from the evaluation date.
pub const MAX_QUARANTINE_HORIZON_DAYS: i64 = 90;

/// Overrides the evaluation date (`YYYY-MM-DD`) so the horizon is testable.
pub const TODAY_ENV: &str = "CYNIC_SOURCE_GATE_TODAY";

/// Minimum substance for a quarantine reason. A one-word reason is a shrug.
const MIN_REASON_CHARS: usize = 40;

/// Owner values that are not owners.
const PLACEHOLDER_OWNERS: &[&str] = &["", "tbd", "todo", "none", "unknown", "nobody", "n/a", "-"];

// ---------------------------------------------------------------------------
// Dates
// ---------------------------------------------------------------------------

/// A calendar date, days resolution, no dependency and no clock dependency
/// beyond `SystemTime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    /// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
    pub epoch_day: i64,
}

impl Date {
    /// Parse `YYYY-MM-DD`, rejecting impossible days (2026-02-30 is not a date).
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
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u32;
    let month = (mp + if mp < 10 { 3 } else { -9 }) as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// The date the rule is evaluated against: `TODAY_ENV` when set (deterministic
/// runs, and a way to prove an expiry without waiting a quarter), else the
/// system UTC date.
pub fn today() -> Date {
    if let Ok(forced) = std::env::var(TODAY_ENV) {
        return Date::parse(&forced).unwrap_or_else(|error| panic!("{TODAY_ENV}={forced:?}: {error}"));
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
// The island
// ---------------------------------------------------------------------------

/// One file inside a closed island.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IslandMember {
    /// Path relative to the crate's `src/`.
    pub path: String,
    /// Lines in the file.
    pub loc: usize,
    /// Unreachable files that attach this one, sorted. The wall.
    pub attached_by_dead: Vec<String>,
    /// Dead files this one attaches, sorted. The rest of the island.
    pub attaches_dead: Vec<String>,
    /// True when this member is attached directly by a wall file, i.e. opening
    /// the wall is what makes it compile. Every member of a one-file island has
    /// this shape; a longer chain does not at its far end.
    pub attached_to_wall: bool,
}

/// The whole closed island of one crate, or the empty set when there is none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Island {
    /// Members, sorted by path.
    pub members: Vec<IslandMember>,
    /// Unreachable files that attach island members but are not members
    /// themselves: the wall, sorted. This is where the repair starts.
    pub wall: Vec<String>,
    /// Lines in the wall files: the cost of opening the island.
    pub wall_loc: usize,
}

impl Island {
    /// Total lines the compiler never sees.
    pub fn loc(&self) -> usize {
        self.members.iter().map(|member| member.loc).sum()
    }

    /// Member paths, sorted.
    pub fn paths(&self) -> Vec<&str> {
        self.members.iter().map(|member| member.path.as_str()).collect()
    }
}

/// Compute the closed include island of one crate, given its paths explicitly.
///
/// A file is in the island when it is NOT reachable from any compilation root
/// AND it exists only because an unreachable file attaches it textually. The
/// closure is transitive: the 43 `cynic-mcp` files were one component.
pub fn closed_include_island_at(crate_dir: &Path, src_root: &Path) -> Island {
    let attachments = orphan_sources::attachments_at(crate_dir, src_root);
    let on_disk = orphan_sources::all_sources(src_root);
    let reachable = orphan_sources::reachable(crate_dir, src_root);

    // Seed: unreachable files that something attaches. A loose orphan that
    // nothing points at is a different defect (the orphan ratchet owns it).
    let mut members: BTreeSet<String> = on_disk
        .difference(&reachable)
        .filter(|path| attachments.targeted.contains(*path))
        .cloned()
        .collect();

    // Transitive closure over attachment edges between unreachable files.
    let mut frontier: Vec<String> = members.iter().cloned().collect();
    while let Some(path) = frontier.pop() {
        let Some(targets) = attachments.edges.get(&path) else {
            continue;
        };
        for target in targets {
            if reachable.contains(target) || !on_disk.contains(target) {
                continue;
            }
            if members.insert(target.clone()) {
                frontier.push(target.clone());
            }
        }
    }

    let wall: BTreeSet<String> = members
        .iter()
        .filter_map(|path| attachments.attached_by.get(path))
        .flatten()
        .filter(|parent| !members.contains(*parent))
        .cloned()
        .collect();

    let loc = |rel: &str| {
        fs::read_to_string(src_root.join(rel))
            .map(|text| text.lines().count())
            .unwrap_or(0)
    };

    let members = members
        .iter()
        .map(|path| IslandMember {
            path: path.clone(),
            loc: loc(path),
            attached_by_dead: attachments
                .attached_by
                .get(path)
                .map(|set| {
                    set.iter()
                        .filter(|parent| !reachable.contains(*parent))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default(),
            attaches_dead: attachments
                .edges
                .get(path)
                .map(|set| {
                    set.iter()
                        .filter(|target| members.contains(*target))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default(),
            attached_to_wall: attachments
                .attached_by
                .get(path)
                .map(|set| set.iter().any(|parent| wall.contains(parent)))
                .unwrap_or(false),
        })
        .collect();

    Island {
        members,
        wall: wall.iter().cloned().collect(),
        wall_loc: wall.iter().map(|rel| loc(rel)).sum(),
    }
}

/// The closed island of a workspace crate, read from the real tree.
pub fn closed_include_island(crate_name: &str) -> Island {
    let root = orphan_sources::workspace_root();
    let crate_dir = root.join("crates").join(crate_name);
    match orphan_sources::crate_src_root(crate_name) {
        Some(src_root) => closed_include_island_at(&crate_dir, &src_root),
        None => Island::default(),
    }
}

/// Every closed island in the workspace, keyed by crate name.
pub fn workspace_islands() -> BTreeMap<String, Island> {
    orphan_sources::crate_names()
        .iter()
        .map(|name| (name.clone(), closed_include_island(name)))
        .filter(|(_, island)| !island.members.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// Quarantine
// ---------------------------------------------------------------------------

/// One quarantined island member. `Deserialize` is the whole point: the
/// manifest is a contract, so an unknown or mistyped field is a parse failure
/// rather than a silently ignored line.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QuarantineEntry {
    /// Crate the file belongs to.
    pub crate_name: String,
    /// Path relative to the crate's `src/`.
    pub path: String,
    /// Who is accountable for the deletion. Not a placeholder.
    pub owner: String,
    /// Why the island member is still on disk.
    pub reason: String,
    /// `YYYY-MM-DD` after which this entry is a FAILURE.
    pub expires: String,
}

/// The manifest. `horizon_days` is asserted against
/// [`MAX_QUARANTINE_HORIZON_DAYS`], so a manifest cannot widen the policy.
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
        let parsed: Self = serde_json::from_str(&text)
            .map_err(|error| format!("{} is not a valid quarantine manifest: {error}", path.display()))?;
        if parsed.rule != RULE_CLOSED_INCLUDE_ISLAND {
            return Err(format!(
                "{} serves rule {:?}, expected {RULE_CLOSED_INCLUDE_ISLAND}",
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
            let key = (entry.crate_name.as_str(), entry.path.as_str());
            if !seen.insert(key) {
                return Err(format!(
                    "{} lists {} twice; a duplicate is two opinions, not one loan",
                    path.display(),
                    key.0
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

// ---------------------------------------------------------------------------
// Violations
// ---------------------------------------------------------------------------

/// One machine-readable finding. `code` is the greppable id, `message` carries
/// the evidence and the repair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// `<RULE>-<SLUG>`.
    pub code: String,
    /// The file the finding is about, relative to the workspace root.
    pub subject: String,
    /// What was observed, and what to do about it.
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {} — {}", self.code, self.subject, self.message)
    }
}

fn violation(rule: &str, slug: &str, subject: &str, message: String) -> Violation {
    Violation {
        code: format!("{rule}-{slug}"),
        subject: subject.to_string(),
        message,
    }
}

/// Evaluate `SRC-001` and `SRC-002` over the whole workspace.
///
/// Fails on a new island member, and fails on a quarantine entry that has
/// expired, over-reached, lost its owner, or stopped describing a real island
/// member. Both directions are failures on purpose: a list that can only grow is
/// the defect this rule exists to end.
pub fn evaluate_islands(
    islands: &BTreeMap<String, Island>,
    quarantine: &Quarantine,
    today: Date,
) -> Vec<Violation> {
    let mut out: Vec<Violation> = Vec::new();
    let declared_horizon = quarantine.horizon_days;

    for (crate_name, island) in islands {
        for member in &island.members {
            let subject = format!("crates/{crate_name}/src/{}", member.path);
            let Some(entry) = quarantine.entry_for(crate_name, &member.path) else {
                out.push(violation(
                    RULE_CLOSED_INCLUDE_ISLAND,
                    "UNQUARANTINED",
                    &subject,
                    format!(
                        "{} lines, attached only by unreachable file(s) {}. No compilation \
                         root names it, so the compiler never reads it: no error, no test, no \
                         warning, and any contract it asserts is inert.\n\
                         Repair: delete it, or declare the wall ({}) and wire the member, or \
                         quarantine it in {QUARANTINE_MANIFEST} with an owner, a reason, and \
                         an expiry within {declared_horizon} day(s).",
                        member.loc,
                        display_list(&member.attached_by_dead),
                        display_list(&island.wall),
                    ),
                ));
                continue;
            };
            out.extend(entry_violations(
                entry,
                &subject,
                crate_name,
                &member.path,
                today,
                declared_horizon,
                Some(member.loc),
            ));
        }
    }

    for entry in &quarantine.entries {
        let subject = format!("crates/{}/src/{}", entry.crate_name, entry.path);
        let is_member = islands
            .get(&entry.crate_name)
            .map(|island| island.members.iter().any(|member| member.path == entry.path))
            .unwrap_or(false);
        if is_member {
            continue; // already checked above
        }
        let exists = orphan_sources::crate_src_root(&entry.crate_name)
            .map(|src| src.join(&entry.path).is_file())
            .unwrap_or(false);
        let message = if orphan_sources::crate_src_root(&entry.crate_name).is_none() {
            format!(
                "quarantined crate `{}` is not a crate with a src/ directory in this workspace",
                entry.crate_name
            )
        } else if !exists {
            format!("the file is gone; the loan was paid. Remove the entry")
        } else {
            format!(
                "the file is on disk and NOT in a closed island: it is either reachable, or \
                 an orphan nothing attaches. Either way this entry is cover for a live or \
                 unrelated file. Remove the entry"
            )
        };
        out.push(violation(
            RULE_QUARANTINE_INTEGRITY,
            "STALE",
            &subject,
            message,
        ));
    }

    out
}

fn entry_violations(
    entry: &QuarantineEntry,
    subject: &str,
    crate_name: &str,
    path: &str,
    today: Date,
    declared_horizon: i64,
    loc: Option<usize>,
) -> Vec<Violation> {
    let mut out: Vec<Violation> = Vec::new();
    let lines = loc
        .map(|loc| format!("{loc} lines"))
        .unwrap_or_else(|| String::from("unknown size"));
    let owner = entry.owner.trim();
    if PLACEHOLDER_OWNERS
        .iter()
        .any(|placeholder| owner.eq_ignore_ascii_case(placeholder))
    {
        out.push(violation(
            RULE_QUARANTINE_INTEGRITY,
            "NO-OWNER",
            subject,
            format!(
                "owner {:?} names nobody. An unowned quarantine is how {} of `{}` stayed \
                 invisible: the file is {lines}. Name the principal accountable for deleting it.",
                entry.owner, path, crate_name
            ),
        ));
    }
    if entry.reason.trim().chars().count() < MIN_REASON_CHARS {
        out.push(violation(
            RULE_QUARANTINE_INTEGRITY,
            "NO-REASON",
            subject,
            format!(
                "reason is {chars} chars; a quarantine reason must be at least {MIN_REASON_CHARS} \
                 and say what the island member is, why it survives, and what removes it.",
                chars = entry.reason.trim().chars().count()
            ),
        ));
    }
    match Date::parse(&entry.expires) {
        Err(error) => out.push(violation(
            RULE_QUARANTINE_INTEGRITY,
            "NO-EXPIRY",
            subject,
            format!("expires {:?}: {error}. Every entry must carry an explicit date", entry.expires),
        )),
        Ok(expires) => {
            let age = expires.epoch_day - today.epoch_day;
            if age < 0 {
                out.push(violation(
                    RULE_QUARANTINE_INTEGRITY,
                    "EXPIRED",
                    subject,
                    format!(
                        "expired {} day(s) ago on {expires} (today {today}). An expired loan is a \
                         failure, not a pass: delete `crates/{crate_name}/src/{path}` or renew the \
                         entry in writing with a new owner and a new date inside the horizon.",
                        -age
                    ),
                ));
            } else if age > declared_horizon {
                out.push(violation(
                    RULE_QUARANTINE_INTEGRITY,
                    "BEYOND-HORIZON",
                    subject,
                    format!(
                        "claims {age} day(s) ({expires}), manifest horizon is {declared_horizon} \
                         and the policy cap is {MAX_QUARANTINE_HORIZON_DAYS}. A quarantine that \
                         can be extended by editing one date is permanent cover; re-quarantine with \
                         a fresh justification."
                    ),
                ));
            }
        }
    }
    out
}

fn display_list(items: &[String]) -> String {
    if items.is_empty() {
        return String::from("(none)");
    }
    items.join(", ")
}

// ---------------------------------------------------------------------------
// `SRC-003` — prose that promises the build a proof it cannot give
// ---------------------------------------------------------------------------

/// Affirmative build guarantees. Each is a promise the compiler cannot keep for
/// a file it never reads, which is exactly the reasoning that kept the
/// `cynic-mcp` island alive.
///
/// SCOPE, DELIBERATELY NARROW. Reachability phrases ("reachable only", "reached
/// only") are NOT in this list, though the first draft of the rule had them. A
/// reachability claim is *checkable* — by the very resolver in
/// `tests/support/orphan_sources.rs` — so the two honest sentences that use it
/// (`infra/CLI_SURFACE_AUDIT.md` §7, `infra/WORKSPACE_ORPHAN_DECISIONS.md` §6,
/// both saying a macro-reachable module "would be misclassified") are
/// limitations of the instrument, not lies about the build. Banning the words
/// would have meant rewriting true statements to satisfy a lint. Only claims
/// about what the BUILD will do are policed here.
const BUILD_GUARANTEE_PHRASES: &[&str] = &[
    "breaks the build",
    "break the build",
    "breaks compilation",
    "fails to compile",
    "fail to compile",
    "will not compile",
    "compile error",
    "compiler rejects",
    "compiler catches",
    "caught by the build",
    "the build catches",
    "the build will fail",
];

/// Markers that make a line a REFUTATION of the guarantee rather than a claim of
/// it. A line carrying one of these is documenting the trap, which is the only
/// legitimate reason to name it.
const REFUTATION_MARKERS: &[&str] = &[
    "not ", "n't", "cannot", "never", "no ", "false", "falsely", "wrong", "myth",
    "used to", "was claimed", "never been", "instead of",
];

/// One prose finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProseFinding {
    /// File relative to the workspace root.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// The offending text, trimmed.
    pub text: String,
    /// The phrase that fired.
    pub phrase: String,
}

/// Scan `dirs` for prose that claims a build guarantee.
///
/// Deliberately a phrase check, not a parser: the property under test is narrow
/// ("does any note in `tests/` or `infra/` justify a defect by promising the
/// build will catch it?"), and a phrase list is auditable in a way a semantic
/// model is not. A false positive costs one rewritten sentence; a false negative
/// costs the island.
pub fn unprovable_build_prose(workspace_root: &Path, dirs: &[&str]) -> Vec<ProseFinding> {
    let mut out: Vec<ProseFinding> = Vec::new();
    for dir in dirs {
        let base = workspace_root.join(dir);
        let mut stack = vec![base];
        while let Some(current) = stack.pop() {
            let Ok(entries) = fs::read_dir(&current) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if path.file_name().and_then(|n| n.to_str()) != Some("fixtures") {
                        stack.push(path);
                    }
                    continue;
                }
                let is_source = path.extension().and_then(|e| e.to_str()) == Some("rs");
                let is_doc = path.extension().and_then(|e| e.to_str()) == Some("md");
                if !is_source && !is_doc {
                    continue;
                }
                let Ok(text) = fs::read_to_string(&path) else {
                    continue;
                };
                let relative = path
                    .strip_prefix(workspace_root)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                for (index, line) in text.lines().enumerate() {
                    let trimmed = line.trim();
                    // A quoted phrase is this rule's own vocabulary, not a claim
                    // about the build: the phrase table below is the only place
                    // these strings are allowed to appear.
                    if trimmed.starts_with('"') {
                        continue;
                    }
                    let lowered = line.to_lowercase();
                    if REFUTATION_MARKERS.iter().any(|marker| lowered.contains(marker)) {
                        continue;
                    }
                    for phrase in BUILD_GUARANTEE_PHRASES {
                        if lowered.contains(phrase) {
                            out.push(ProseFinding {
                                file: relative.clone(),
                                line: index + 1,
                                text: line.trim().to_string(),
                                phrase: (*phrase).to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    out.sort_by(|left, right| (&left.file, left.line).cmp(&(&right.file, right.line)));
    out
}

/// Render the `SRC-003` finding as a violation.
pub fn prose_violation(finding: &ProseFinding) -> Violation {
    violation(
        RULE_UNPROVABLE_BUILD_PROSE,
        "BUILD-GUARANTEE",
        &format!("{}:{}", finding.file, finding.line),
        format!(
            "prose claims {:?} ({:?}). A build cannot catch a source file it never reads, and a \
             quarantine note that leans on that claim is how an island stays invisible. Either \
             state the observation that is checkable (which compilation root names this file, or \
             which rule measures it), or refute the guarantee explicitly.",
            finding.text, finding.phrase
        ),
    )
}

/// Directories `SRC-003` reads. Kept narrow and explicit: the rule is about
/// notes that defend a source-tree defect, not about every Markdown file.
pub const PROSE_DIRS: &[&str] = &["tests", "infra"];
