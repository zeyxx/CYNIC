//! Every declaration of the authority ladder must equal the contract's.
//!
//! WHY THIS FILE EXISTS. The ladder lived in five places at once:
//! `contracts/foundation.json`, `infra/runtime-authority.json`,
//! `infra/directory-tree.json`, `infra/crate-topology.json` and a prose list in
//! `AGENTS.md`. Four of them agreed; `AGENTS.md` was seven entries long and
//! omitted `infra/directory-tree.json`, so a disagreement between
//! `directory-tree.json` and `registry.json` had no ranked answer — the two
//! files the ladder exists to separate. And `crate-topology.json` ranked
//! itself eighth, which is a contract amplifying its own authority: the ladder
//! exists to settle what outranks a document, so a document cannot be the
//! source of its own rank.
//!
//! WHAT IT ASSERTS. That the contract's `authority_order` is the ladder: the two
//! other contracts that declare one agree with it exactly, the prose list in
//! `AGENTS.md` is the same eight entries in the same order, and a derived
//! document does not declare one at all.
//!
//! HOW TO READ A FAILURE. A mismatch is not a formatting problem. It means two
//! documents now disagree about which of them wins, and the disagreement is
//! invisible until the two are the pair that actually conflict.

#![allow(clippy::expect_used)]

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return dir;
        }
        assert!(dir.pop(), "workspace root not found");
    }
}

fn read_json(relative: &str) -> serde_json::Value {
    let path = workspace_root().join(relative);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

/// The ladder the contract declares, which is the only one that needs to exist.
fn contract_ladder() -> Vec<String> {
    let foundation = read_json("contracts/foundation.json");
    foundation["constitutional"]["authority_order"]
        .as_array()
        .expect("foundation.json declares constitutional.authority_order")
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .expect("authority_order holds strings")
                .to_string()
        })
        .collect()
}

/// The prose ladder in `AGENTS.md`, parsed out of the numbered list.
///
/// Parsed rather than restated because a second copy of the list in a test is
/// the same defect one level down: the test would agree with itself and the
/// prose could drift from the contract without anything noticing.
fn agents_ladder() -> Vec<String> {
    let path = workspace_root().join("AGENTS.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let Some(start) = text.find("When files disagree:") else {
        panic!("AGENTS.md has no 'When files disagree:' section");
    };
    let section = &text[start..];
    let mut ladder = Vec::new();
    for line in section.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !ladder.is_empty() {
                break;
            }
            continue;
        }
        let Some(dot) = trimmed.find(". ") else {
            if !ladder.is_empty() {
                break;
            }
            continue;
        };
        let number = &trimmed[..dot];
        if number.parse::<u32>().is_err() {
            if !ladder.is_empty() {
                break;
            }
            continue;
        }
        // Backticks are prose formatting around a path, not part of the path, and
        // they can appear mid-entry (`Cargo.toml` workspace), so they are removed
        // rather than trimmed.
        ladder.push(trimmed[dot + 2..].replace('`', "").to_string());
    }
    assert!(!ladder.is_empty(), "AGENTS.md declares no authority ladder");
    ladder
}

#[test]
fn the_contract_ladder_is_the_one_that_exists() {
    let ladder = contract_ladder();
    assert_eq!(
        ladder.len(),
        8,
        "the ladder changed length. A new authority was added or removed: {ladder:?}"
    );
    assert_eq!(
        ladder[0], "observed runtime state",
        "observed state is the ground truth and must stay first"
    );
    assert_eq!(
        ladder[1], "contracts/foundation.json",
        "the constitutional contract must stay second"
    );
}

#[test]
fn the_other_contracts_that_declare_a_ladder_agree_with_the_contract() {
    let ladder = contract_ladder();
    for relative in ["infra/runtime-authority.json", "infra/directory-tree.json"] {
        let declared = read_json(relative)["authority_order"]
            .as_array()
            .unwrap_or_else(|| panic!("{relative} declares no authority_order"))
            .iter()
            .filter_map(|entry| entry.as_str().map(str::to_string))
            .collect::<Vec<_>>();
        assert_eq!(
            declared, ladder,
            "{relative} and contracts/foundation.json disagree about the ladder.\n\
             contract: {ladder:?}\n\
             {relative}: {declared:?}",
        );
    }
}

#[test]
fn the_prose_ladder_in_agents_is_the_contract_ladder() {
    let ladder = contract_ladder();
    let prose = agents_ladder();
    assert_eq!(
        prose, ladder,
        "AGENTS.md and contracts/foundation.json disagree about the ladder.\n\
         contract: {ladder:?}\n\
         AGENTS.md: {prose:?}",
    );
}

/// A contract that ranks itself is not a contract about authority.
#[test]
fn no_contract_ranks_itself() {
    let topology = read_json("infra/crate-topology.json");
    assert!(
        topology.get("authority_order").is_none(),
        "infra/crate-topology.json declares an authority_order. The ladder lives in \
         contracts/foundation.json; a document that restates it can only drift, and \
         this one ranked itself."
    );
}

/// A document may CITE the authority; it may not CLAIM to BE the authority.
///
/// WHY THIS EXISTS. `infra/CANONICAL_CRATE_MAP.md` declared
/// `> **Authority** : canonical taxonomy source for workspace crates. Referenced by
/// AGENTS.md, README.md, and contracts/foundation.json`. Measured 2026-09-30:
/// `contracts/foundation.json` does not mention that file at all, and neither does
/// the ladder in `AGENTS.md`. The claim was self-issued, and because it was never
/// executed it stood for days — long enough that three documents came to describe
/// three different crate policies, and every agent arriving read a different one.
///
/// THE DISTINCTION THIS ENFORCES. Two shapes are legal:
///   `> **Authority**: contracts/foundation.json`            — a citation, resolving upward
///   `> **Status**: census, NOT the taxonomy authority`       — an explicit disclaim
/// One shape is not:
///   `> **Authority**: canonical taxonomy source ...`         — a self-issued rank
///
/// The test cannot read intent, so it reads structure: a document may name another
/// document as its authority, but a phrase that ranks the document ITSELF
/// ("canonical source", "single source of truth", "the authority for X") is a
/// violation. A citation resolves to a path; a self-rank resolves to nothing.
///
/// Fails in both directions, like every ratchet here: a new self-rank is rejected,
/// and a repaired document must leave `SELF_RANKING_BASELINE`.
#[test]
fn no_document_ranks_itself() {
    // Phrases that rank the document itself rather than citing another.
    const SELF_RANK_PATTERNS: &[&str] = &[
        "canonical taxonomy source",
        "canonical authority source",
        "is the single source of truth",
        "is the canonical source",
        "the authority for whether",
    ];

    // Documents permitted to name an authority, with the shape they must keep.
    // A citation (`Authority: contracts/foundation.json`) is not a self-rank and
    // is not listed here. Every entry below is a measured, shrink-only exception.
    const SELF_RANKING_BASELINE: &[&str] = &[
        // `contracts/pruning-policy.json` is ratified by the operator and its
        // `ratification.applies_to_the_agent` states it is "the authority for
        // whether a crate may be created, fused, or removed". That is a self-rank
        // in prose, and it is TRUE — the operator ratified it. It is recorded
        // rather than removed because the rank is real; removing the sentence
        // would delete a fact, not a violation.
        "contracts/pruning-policy.json",
    ];

    let mut found: Vec<String> = Vec::new();

    for relative in document_paths() {
        let Ok(text) = std::fs::read_to_string(workspace_root().join(&relative)) else {
            continue;
        };
        let lowered = text.to_lowercase();
        for pattern in SELF_RANK_PATTERNS {
            if lowered.contains(&pattern.to_lowercase()) {
                found.push(relative.clone());
                break;
            }
        }
    }
    found.sort();
    found.dedup();

    let baseline: Vec<String> = SELF_RANKING_BASELINE
        .iter()
        .map(|s| s.to_string())
        .collect();
    let new: Vec<&String> = found.iter().filter(|f| !baseline.contains(f)).collect();
    let stale: Vec<&String> = baseline.iter().filter(|b| !found.contains(b)).collect();

    assert!(
        new.is_empty(),
        "documents that rank themselves as authority:\n{new:#?}\n\n\
         A document may cite the authority (a path that resolves upward) or disclaim \
         it (`**Status**: NOT the authority`). It may not declare itself canonical. \
         Three documents once claimed this at once and every agent arriving read a \
         different policy. If the rank is real, record it in the baseline AND in \
         contracts/foundation.json; otherwise reword it as a citation."
    );
    assert!(
        stale.is_empty(),
        "baseline entries that no longer rank themselves:\n{stale:#?}\n\n\
         Remove them. A ratchet that cannot tighten is a ratchet that rots."
    );
}

/// Every `.md` and `.json` document under `infra/` and `contracts/`, relative to
/// the workspace root, excluding archived and quarantined trees.
///
/// Archived trees are excluded on purpose: a superseded document that still says
/// "canonical" is history, not a competing authority, and its quarantine note is
/// what tells a reader so.
fn document_paths() -> Vec<String> {
    const ROOTS: &[&str] = &["infra", "contracts"];
    const EXCLUDED: &[&str] = &["archive", "quarantine", "runtime-checkpoints"];

    let mut out = Vec::new();
    let mut stack: Vec<std::path::PathBuf> = ROOTS
        .iter()
        .map(|root| workspace_root().join(root))
        .collect();

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if !EXCLUDED.contains(&name.as_str()) {
                    stack.push(path);
                }
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "md" || e == "json")
            {
                if let Ok(relative) = path.strip_prefix(workspace_root()) {
                    out.push(relative.display().to_string());
                }
            }
        }
    }
    out.sort();
    out
}
