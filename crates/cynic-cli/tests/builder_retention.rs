//! Executable invariants for `contracts/builder-retention.json`.
//!
//! The builder (10.10.10.119) fills its disk with reproducible per-SHA and
//! session checkouts. The contract owns the retention thresholds; this test
//! proves the contract is complete and self-consistent. It does not run any
//! purge — enforcement reports an inventory before every delete.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate is two levels below the workspace root")
        .to_path_buf()
}

fn contract() -> Value {
    let raw = fs::read_to_string(workspace_root().join("contracts/builder-retention.json"))
        .expect("read contracts/builder-retention.json");
    serde_json::from_str(&raw).expect("contracts/builder-retention.json is valid JSON")
}

#[test]
fn retention_contract_schema_is_complete() {
    let doc = contract();
    assert_eq!(
        doc["schema"].as_str(),
        Some("cynic.builder_retention.v1"),
        "schema marker must match"
    );
    assert!(
        doc["owner"].as_str().is_some_and(|owner| !owner.is_empty()),
        "one owner per dimension: a retention contract without owner is unenforceable"
    );
    let rules = doc["rules"].as_array().expect("rules is an array");
    assert!(
        !rules.is_empty(),
        "a retention contract without rules retains nothing"
    );
    for rule in rules {
        for key in ["id", "path", "retain_days", "condition", "action"] {
            assert!(
                rule.get(key).is_some(),
                "rule {} is missing {key}",
                rule["id"].as_str().unwrap_or("?")
            );
        }
        let days = rule["retain_days"].as_u64().unwrap_or(0);
        assert!(
            (1..=90).contains(&days),
            "rule {} retain_days={days} outside sane bounds 1..=90",
            rule["id"].as_str().unwrap_or("?")
        );
    }
    let protected = doc["protected"].as_array().expect("protected is an array");
    assert!(
        !protected.is_empty(),
        "no protected set: the purge would have no fence"
    );
    assert!(
        doc["read_back"].as_str().is_some_and(|rb| !rb.is_empty()),
        "every purge needs its read-back proof declared"
    );
}
