//! Pure coordination-ledger domain logic (FCIS: no I/O).
//!
//! A coordination ledger is the unified registry over the two legacy
//! coordination surfaces: `.agent-claims.json` (scopes) and the
//! fire-and-forget `coordination_log` (JSONL entries). This module owns
//! types, validation, and in-memory construction only. File I/O lives in
//! the adapter surface (`coordination_surface`).

use serde::{Deserialize, Serialize};

pub const COORDINATION_LEDGER_SCHEMA: &str = "cynic.coordination_ledger.v1";
pub const DEFAULT_LEDGER_STATUS: &str = "observed";

/// One referenced coordination source (a legacy log).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinationLedgerSource {
    pub name: String,
    pub kind: String,
    pub path: String,
    #[serde(default)]
    pub schema: Option<String>,
}

/// Unified registry referencing both legacy coordination systems.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoordinationLedger {
    pub schema: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub claims_path: Option<String>,
    #[serde(default)]
    pub coordination_log_path: Option<String>,
    #[serde(default)]
    pub sources: Vec<CoordinationLedgerSource>,
    #[serde(default)]
    pub merge_queue: Vec<serde_json::Value>,
}

impl CoordinationLedger {
    pub fn new_empty() -> Self {
        Self {
            schema: COORDINATION_LEDGER_SCHEMA.to_string(),
            status: DEFAULT_LEDGER_STATUS.to_string(),
            claims_path: None,
            coordination_log_path: None,
            sources: Vec::new(),
            merge_queue: Vec::new(),
        }
    }

    pub fn new_unified(claims_path: &str, coordination_log_path: &str) -> Self {
        Self {
            schema: COORDINATION_LEDGER_SCHEMA.to_string(),
            status: DEFAULT_LEDGER_STATUS.to_string(),
            claims_path: Some(claims_path.to_string()),
            coordination_log_path: Some(coordination_log_path.to_string()),
            sources: vec![
                CoordinationLedgerSource {
                    name: "agent_claims".to_string(),
                    kind: "scopes".to_string(),
                    path: claims_path.to_string(),
                    schema: Some("cynic.agent_claims.v2".to_string()),
                },
                CoordinationLedgerSource {
                    name: "coordination_log".to_string(),
                    kind: "fire_and_forget".to_string(),
                    path: coordination_log_path.to_string(),
                    schema: Some("cynic.coordination_entry.v1".to_string()),
                },
            ],
            merge_queue: Vec::new(),
        }
    }
}

/// Pure builder: construct a unified ledger from the two source paths.
pub fn build_ledger(claims_path: &str, coordination_log_path: &str) -> CoordinationLedger {
    CoordinationLedger::new_unified(claims_path, coordination_log_path)
}

/// Pure validation: a ledger must reference both legacy systems.
pub fn is_unified(ledger: &CoordinationLedger) -> bool {
    ledger
        .claims_path
        .as_ref()
        .is_some_and(|p| !p.trim().is_empty())
        && ledger
            .coordination_log_path
            .as_ref()
            .is_some_and(|p| !p.trim().is_empty())
        && ledger.sources.len() >= 2
}
