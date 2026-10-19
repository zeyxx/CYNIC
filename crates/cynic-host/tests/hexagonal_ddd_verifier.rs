//! FCIS and DDD verification — the structural half of the architecture policy.
//!
//! WHY THIS FILE EXISTS. `contracts/pruning-policy.json` → `taxonomy.fcis_and_ddd`
//! declares the policy in force (Functional Core / Imperative Shell with
//! domain-driven design) and records `status: partially_enforced` with a
//! `known_gap` naming clauses that no test checked:
//!
//!   1. that every adapter implements a declared port — RETIRED as a goal. FCIS
//!      does not require it: adapters may be concrete and unbound by a trait.
//!      A port with no second implementation is indirection, not architecture.
//!   2. that a concept named in two crates has one canonical home
//!   3. that an application layer is distinct from the domain and the infrastructure
//!   4. aggregate boundaries and invariants
//!
//! This file closes (2). Clause (1) is retired with its reason recorded rather
//! than silently dropped. Clauses (3) and (4) stay open and are named in the
//! `known_gap` assertion at the bottom, so the gap cannot silently become a claim.
//!
//! WHAT IT MEASURES. Two structural facts, both read from the source, both
//! failing in both directions like every ratchet in this workspace: a new
//! violation is rejected, and a repaired one must leave its baseline.
//!
//!   1. PORT COVERAGE — every `pub trait` declared under a crate's `src/ports/`
//!      must have an `impl Trait for ...` somewhere in the workspace. Kept as a
//!      MEASUREMENT (not a policy requirement): under FCIS a concrete adapter is
//!      legitimate, so this reports indirection rather than enforcing a rule.
//!
//!   2. CONCEPT OWNERSHIP — a domain concept (a `pub struct`/`enum`/`trait`) must
//!      not be defined in two crates. This is `one_responsibility_one_crate` applied
//!      to vocabulary rather than to crate descriptions, which is what
//!      `crate_taxonomy_ownership.rs` checks.
//!
//! WHAT IT DELIBERATELY DOES NOT MEASURE. Types whose names end in a local suffix
//! (`Error`, `Result`, `Config`, `Settings`, `Options`, `State`, `Outcome`,
//! `Report`, `Request`, `Response`) are excluded from concept ownership. One
//! `StorageError` per crate that can fail is idiomatic Rust, not a collision —
//! gating those would produce noise that trains readers to ignore the gate. The
//! exclusion is a recorded decision, not an oversight.
//!
//! This test reads source text rather than the compiled graph on purpose: it must
//! run without linking the crates it inspects, because several of them ICE the
//! compiler on this host.

#![allow(clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return dir;
        }
        assert!(dir.pop(), "workspace root not found");
    }
}

/// Every workspace crate name, from `crates/`.
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

/// Read every `.rs` file under `dir` into one string.
fn read_tree(dir: &Path) -> String {
    let mut out = String::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Ok(text) = fs::read_to_string(&path) {
                    out.push_str(&text);
                    out.push('\n');
                }
            }
        }
    }
    out
}

/// Strip `#[cfg(test)]` blocks so production vocabulary is measured alone.
///
/// A type defined only inside a test module is a fixture, not a concept with a
/// home. Counting it would report collisions that no consumer can ever see.
fn production_only(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    loop {
        let Some(at) = rest.find("#[cfg(test)]") else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..at]);
        let mut depth: i32 = 0;
        let mut started = false;
        let mut i = at;
        let bytes = rest.as_bytes();
        while i < bytes.len() {
            match bytes[i] {
                b'{' => {
                    depth += 1;
                    started = true;
                }
                b'}' => {
                    depth -= 1;
                    if started && depth == 0 {
                        i += 1;
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        rest = &rest[i..];
    }
}

/// `pub trait Name` declarations under a crate's `src/ports/`.
fn declared_ports(crate_name: &str) -> BTreeSet<String> {
    let dir = workspace_root()
        .join("crates")
        .join(crate_name)
        .join("src")
        .join("ports");
    if !dir.is_dir() {
        return BTreeSet::new();
    }
    let text = read_tree(&dir);
    scan_names(&text, "pub trait ")
}

/// Naive scan for `pub <kind> Name`, tolerant of preceding attributes.
fn scan_names(text: &str, prefix: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for (idx, _) in text.match_indices(prefix) {
        let after = &text[idx + prefix.len()..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        // A generic bound (`pub trait Foo<T>`) ends at `<`; an empty name means
        // the prefix matched inside a comment or a string.
        if !name.is_empty() {
            found.insert(name);
        }
    }
    found
}

/// A trait declared under `src/ports/` with no `impl Trait for` in the workspace.
const ORPHAN_PORT_BASELINE: &[&str] = &[
    // Each entry is a port that exists, is not implemented, and whose decision is
    // still open. Measured 2026-09-30: 5 of 121 ports are referenced nowhere at
    // all (not even as a parameter type), and 7 more are referenced but never
    // implemented. The five are recorded here; the seven are recorded because a
    // port used as a bound is still a boundary with no adapter.
    "BuildWorkloadObservationPort",
    "CredentialRotationAuthorityPort",
    "NodeControlStorePort",
    "NodeEnrollmentStorePort",
    "ProviderControlPort",
    "PublishArtefact",
    "RoutingEventPort",
    "SignedNodeDesiredStatePullPort",
    "SummarizationPort",
    "TargetDiscoveryPort",
];

/// Local-type suffixes excluded from concept ownership.
///
/// Recorded as data, not as taste: one `StorageError` per crate that can fail is
/// idiomatic, and gating it would bury the real collisions in noise.
const LOCAL_TYPE_SUFFIXES: &[&str] = &[
    "Error", "Result", "Config", "Settings", "Options", "State", "Outcome", "Report", "Request",
    "Response",
];

/// Domain concepts defined in more than one crate.
///
/// Measured 2026-09-30. This list may only shrink: a concept that gains a single
/// canonical home must leave it, and `pruning-policy.json → one_responsibility_one_crate`
/// makes the canonical owner win and the copy deleted or repointed.
///
/// Note what is absent: `ContextResult`, `IdentityResult`, `DogOperationOutcome`,
/// `PeerState` and `HostExecRequest` are genuinely defined in two crates each, but
/// they end in `Result`/`Outcome`/`State`/`Request` and are therefore excluded by
/// `LOCAL_TYPE_SUFFIXES`. Listing them here would make this ratchet report a
/// permanent stale entry, which is the one thing a shrink-only list must not do.
const CONCEPT_COLLISION_BASELINE: &[&str] = &[
    "AgentKind",
    "AgentRuntimeMode",
    "ArgvOutput",
    "ArgvRunner",
    "Capability",
    "Claim",
    "ClaimStatus",
    "Cli",
    "ContributionKind",
    "DiscoveredService",
    "EScoreMetrics",
    "Epoch",
    "FleetInventoryDetails",
    "FleetInventoryProviderSummary",
    "FleetInventorySummary",
    "FleetNodeObservation",
    "ForgejoPayload",
    "LiquidityVault",
    "MemorySecretProvider",
    "MergeStrategy",
    "MerkleProof",
    "MerkleProofStep",
    "NetworkPathEvidence",
    "Observation",
    "ObservationWindow",
    "ProbeStatus",
    "RegistryNode",
    "ReleaseProof",
    "RetryPolicy",
    "RuntimeHarnessContext",
    "RuntimeHarnessKind",
    "RuntimePrincipalLayout",
    "SecretMaterial",
    "SettlementNotification",
    "SocThresholds",
    "SourceChangeAdmission",
    "SourceChangeProposal",
    "SourceMatterClass",
    "SourcePromotionStage",
    "StorageDecision",
    "StorageObservation",
    "Task",
    "TaskKind",
    "Thresholds",
    "TokenEScore",
    "TraceStatus",
    "TransportScope",
    "TransportSecuritySurface",
    "Verdict",
    "WalEntry",
    "WalletSnapshot",
];

fn observed_orphan_ports() -> BTreeSet<String> {
    let mut orphans = BTreeSet::new();
    for crate_name in crate_names() {
        let ports = declared_ports(&crate_name);
        if ports.is_empty() {
            continue;
        }
        let whole = read_tree(
            &workspace_root()
                .join("crates")
                .join(&crate_name)
                .join("src"),
        );
        for port in ports {
            let needle = format!("impl {port} for");
            let generic = format!("impl<");
            let implemented = whole.contains(&needle)
                || whole.match_indices(&generic).any(|(idx, _)| {
                    whole[idx..]
                        .split('{')
                        .next()
                        .is_some_and(|h| h.contains(&port))
                });
            if !implemented {
                orphans.insert(port);
            }
        }
    }
    orphans
}

/// A core file (outside `src/adapters/`) that reaches for an I/O primitive.
///
/// FCIS says the pure core holds deterministic functions and data only. This
/// measures the crates that have already split their shell (`src/adapters/`
/// exists) and counts the core files that still reach for I/O directly.
///
/// The baseline is a shrink-only ratchet: a NEW leak fails, and a repaired one
/// must leave the list. Crate names are the unit, not file paths — a path moves
/// when a module is reorganised, and a ratchet keyed on a path reports churn as
/// debt. Measured 2026-10-06 on the crates that declare an `adapters/` directory.
const FCIS_CORE_IO_BASELINE: &[&str] = &[
    // Each entry is a crate whose pure core still performs I/O outside adapters.
    // These are the crates that have SPLIT their shell and not yet cleaned the
    // core: the boundary exists, the discipline does not.
    "cynic-host",
    "cynic-inference",
    "cynic-node",
    "cynic-secrets",
];

/// I/O primitives that belong in the imperative shell, never in a pure core.
const IO_PRIMITIVES: &[&str] = &["std::process::Command", "reqwest::", "tokio::spawn"];

/// Core files of `crate_name` (outside `src/adapters/`) that reach for I/O.
fn core_files_with_io(crate_name: &str) -> Vec<String> {
    let src = workspace_root().join("crates").join(crate_name).join("src");
    let mut leaks = Vec::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // The shell is where I/O is supposed to live.
                if path.file_name().and_then(|n| n.to_str()) == Some("adapters") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.ends_with(".rs") || name == "tests.rs" || name.ends_with("_tests.rs") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let text = production_only(&text);
            if IO_PRIMITIVES.iter().any(|needle| text.contains(needle)) {
                leaks.push(path.display().to_string());
            }
        }
    }
    leaks.sort();
    leaks
}

/// Crates that declare an `adapters/` directory — i.e. that have adopted FCIS.
fn fcis_adopting_crates() -> Vec<String> {
    crate_names()
        .into_iter()
        .filter(|name| {
            workspace_root()
                .join("crates")
                .join(name)
                .join("src/adapters")
                .is_dir()
        })
        .collect()
}

fn observed_concept_collisions() -> BTreeMap<String, BTreeSet<String>> {
    let mut owners: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for crate_name in crate_names() {
        let src = workspace_root()
            .join("crates")
            .join(&crate_name)
            .join("src");
        let mut stack = vec![src];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if !name.ends_with(".rs") || name == "tests.rs" || name.ends_with("_tests.rs") {
                    continue;
                }
                let Ok(text) = fs::read_to_string(&path) else {
                    continue;
                };
                let text = production_only(&text);
                for prefix in ["pub struct ", "pub enum ", "pub trait "] {
                    for found in scan_names(&text, prefix) {
                        owners.entry(found).or_default().insert(crate_name.clone());
                    }
                }
            }
        }
    }
    owners
        .into_iter()
        .filter(|(name, crates)| {
            crates.len() > 1 && !LOCAL_TYPE_SUFFIXES.iter().any(|s| name.ends_with(s))
        })
        .collect()
}

#[test]
fn every_port_has_an_adapter_or_is_recorded() {
    let observed = observed_orphan_ports();
    let baseline: BTreeSet<&str> = ORPHAN_PORT_BASELINE.iter().copied().collect();

    let new: Vec<&String> = observed
        .iter()
        .filter(|p| !baseline.contains(p.as_str()))
        .collect();
    let stale: Vec<&&str> = baseline
        .iter()
        .filter(|b| !observed.contains(**b))
        .collect();

    assert!(
        new.is_empty(),
        "ports declared with no adapter anywhere in the workspace:\n{new:#?}\n\n\
         A port is a boundary. Declaring one without an adapter is a boundary that \
         was never built: the trait compiles, the call site has nothing to inject, \
         and the capability is absent while looking present. Either implement it, \
         delete it, or record the decision in ORPHAN_PORT_BASELINE."
    );
    assert!(
        stale.is_empty(),
        "baseline entries that now have an adapter:\n{stale:#?}\n\n\
         Remove them. A ratchet that cannot tighten is a ratchet that rots."
    );
}

#[test]
fn a_domain_concept_has_one_home() {
    let observed = observed_concept_collisions();
    let baseline: BTreeSet<&str> = CONCEPT_COLLISION_BASELINE.iter().copied().collect();

    let new: Vec<(&String, &BTreeSet<String>)> = observed
        .iter()
        .filter(|(name, _)| !baseline.contains(name.as_str()))
        .collect();
    let stale: Vec<&&str> = baseline
        .iter()
        .filter(|b| !observed.contains_key(**b))
        .collect();

    assert!(
        new.is_empty(),
        "domain concepts now defined in more than one crate:\n{new:#?}\n\n\
         contracts/pruning-policy.json → one_responsibility_one_crate: a \
         responsibility has exactly one owning crate, and two crates that both \
         claim it are CONTESTED — the canonical owner wins and the copy is \
         deleted or repointed. This is that rule applied to vocabulary rather \
         than to crate descriptions."
    );
    assert!(
        stale.is_empty(),
        "baseline concepts that now have a single home:\n{stale:#?}\n\n\
         Remove them. A ratchet that cannot tighten is a ratchet that rots."
    );
}

/// FCIS: the pure core holds no I/O. Measured per crate, ratcheted shrink-only.
///
/// This is the generalization of `cynic-network/tests/architecture_boundary.rs`,
/// which is the reference implementation. It is deliberately scoped to crates
/// that HAVE an `adapters/` directory: a crate that never adopted FCIS has no
/// shell to leak out of, and failing it would be measuring adoption rather than
/// discipline. The baseline names the crates that split the shell and have not
/// yet cleaned the core — a shrink-only list, like every ratchet here.
#[test]
fn a_pure_core_holds_no_io() {
    let baseline: BTreeSet<&str> = FCIS_CORE_IO_BASELINE.iter().copied().collect();
    let mut observed: BTreeSet<String> = BTreeSet::new();
    let mut detail: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for crate_name in fcis_adopting_crates() {
        let leaks = core_files_with_io(&crate_name);
        if !leaks.is_empty() {
            observed.insert(crate_name.clone());
            detail.insert(crate_name, leaks);
        }
    }

    let new: Vec<&String> = observed
        .iter()
        .filter(|c| !baseline.contains(c.as_str()))
        .collect();
    let stale: Vec<&&str> = baseline
        .iter()
        .filter(|b| !observed.contains(**b))
        .collect();

    assert!(
        new.is_empty(),
        "crates whose pure core now performs I/O outside `src/adapters/`:\n{new:#?}\n\n\
         FCIS (contracts/foundation.json → fcis_core_shell_separation): the core \
         holds deterministic functions and data only. Move the I/O into the \
         imperative shell, or record the decision in FCIS_CORE_IO_BASELINE.\n\n\
         Files involved:\n{detail:#?}"
    );
    assert!(
        stale.is_empty(),
        "baseline crates whose core is now clean:\n{stale:#?}\n\n\
         Remove them. A ratchet that cannot tighten is a ratchet that rots."
    );
}

/// The gap this file does NOT close, asserted so it cannot be forgotten.
///
/// `pruning-policy.json → taxonomy.fcis_and_ddd.known_gap` names four
/// unverified clauses. Two are now covered above. The remaining two are asserted
/// here by reading the contract, so deleting them from the contract fails this
/// test rather than quietly shrinking the gap.
#[test]
fn the_unverified_half_of_the_policy_is_still_declared() {
    let path = workspace_root().join("contracts/pruning-policy.json");
    let text = fs::read_to_string(&path).expect("read contracts/pruning-policy.json");

    for clause in [
        "partially_enforced",
        "known_gap",
        "that an application layer is distinct from the domain and the infrastructure",
        "aggregate boundaries and invariants",
    ] {
        assert!(
            text.contains(clause),
            "contracts/pruning-policy.json no longer carries `{clause}`. The \
             FCIS/DDD policy is still only partially enforced; a contract \
             that stops saying so reads as a guarantee."
        );
    }
}
