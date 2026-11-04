//! Pure idea-ledger domain logic (FCIS: no I/O).
//!
//! An idea ledger tracks ideas, their lifecycle status, freshness, and
//! consumption/compounding links. This module owns types, validation, and
//! in-memory transformations only. File I/O lives in the adapter surface.

use serde::{Deserialize, Serialize};

pub const IDEA_LEDGER_SCHEMA: &str = "cynic.idea_ledger.v1";
pub const DEFAULT_LEDGER_STATUS: &str = "seed";

pub const VALID_STATUSES: &[&str] = &["seed", "growing", "mature", "abandoned"];
pub const VALID_FRESHNESSES: &[&str] = &["fresh", "stale", "orphan"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdeaLedger {
    pub schema: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub ideas: Vec<IdeaEntry>,
}

impl IdeaLedger {
    pub fn new_empty() -> Self {
        Self {
            schema: IDEA_LEDGER_SCHEMA.to_string(),
            status: DEFAULT_LEDGER_STATUS.to_string(),
            ideas: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdeaEntry {
    pub idea_id: String,
    pub title: String,
    pub description: String,
    pub domain: String,
    pub status: String,
    pub freshness: String,
    #[serde(default)]
    pub consumed_by: Vec<String>,
    #[serde(default)]
    pub compounds: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    EmptyTitle,
    EmptyIdeaId,
    InvalidStatus(String),
    InvalidFreshness(String),
    DuplicateIdeaId(String),
    IdeaNotFound(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyTitle => write!(f, "idea title must not be empty"),
            Self::EmptyIdeaId => write!(f, "idea id must not be empty"),
            Self::InvalidStatus(s) => write!(
                f,
                "invalid idea status '{s}': expected one of seed/growing/mature/abandoned"
            ),
            Self::InvalidFreshness(fr) => write!(
                f,
                "invalid freshness '{fr}': expected one of fresh/stale/orphan"
            ),
            Self::DuplicateIdeaId(id) => write!(f, "idea id already exists: {id}"),
            Self::IdeaNotFound(id) => write!(f, "idea not found: {id}"),
        }
    }
}

pub fn is_valid_status(status: &str) -> bool {
    VALID_STATUSES.contains(&status)
}

pub fn is_valid_freshness(freshness: &str) -> bool {
    VALID_FRESHNESSES.contains(&freshness)
}

pub fn add_idea(ledger: &mut IdeaLedger, entry: IdeaEntry) -> Result<(), LedgerError> {
    if entry.title.trim().is_empty() {
        return Err(LedgerError::EmptyTitle);
    }
    if entry.idea_id.trim().is_empty() {
        return Err(LedgerError::EmptyIdeaId);
    }
    if !is_valid_status(&entry.status) {
        return Err(LedgerError::InvalidStatus(entry.status.clone()));
    }
    if !is_valid_freshness(&entry.freshness) {
        return Err(LedgerError::InvalidFreshness(entry.freshness.clone()));
    }
    if ledger.ideas.iter().any(|i| i.idea_id == entry.idea_id) {
        return Err(LedgerError::DuplicateIdeaId(entry.idea_id.clone()));
    }
    ledger.ideas.push(entry);
    Ok(())
}

pub fn update_status(
    ledger: &mut IdeaLedger,
    idea_id: &str,
    status: &str,
    now: &str,
) -> Result<IdeaEntry, LedgerError> {
    if idea_id.trim().is_empty() {
        return Err(LedgerError::EmptyIdeaId);
    }
    if !is_valid_status(status) {
        return Err(LedgerError::InvalidStatus(status.to_string()));
    }
    let entry = ledger
        .ideas
        .iter_mut()
        .find(|i| i.idea_id == idea_id)
        .ok_or_else(|| LedgerError::IdeaNotFound(idea_id.to_string()))?;
    entry.status = status.to_string();
    entry.updated_at = now.to_string();
    Ok(entry.clone())
}

pub fn update_freshness(
    ledger: &mut IdeaLedger,
    idea_id: &str,
    freshness: &str,
    now: &str,
) -> Result<IdeaEntry, LedgerError> {
    if idea_id.trim().is_empty() {
        return Err(LedgerError::EmptyIdeaId);
    }
    if !is_valid_freshness(freshness) {
        return Err(LedgerError::InvalidFreshness(freshness.to_string()));
    }
    let entry = ledger
        .ideas
        .iter_mut()
        .find(|i| i.idea_id == idea_id)
        .ok_or_else(|| LedgerError::IdeaNotFound(idea_id.to_string()))?;
    entry.freshness = freshness.to_string();
    entry.updated_at = now.to_string();
    Ok(entry.clone())
}

pub fn consume(
    ledger: &mut IdeaLedger,
    idea_id: &str,
    consumer: &str,
    now: &str,
) -> Result<IdeaEntry, LedgerError> {
    if idea_id.trim().is_empty() {
        return Err(LedgerError::EmptyIdeaId);
    }
    if consumer.trim().is_empty() {
        return Err(LedgerError::EmptyTitle);
    }
    let entry = ledger
        .ideas
        .iter_mut()
        .find(|i| i.idea_id == idea_id)
        .ok_or_else(|| LedgerError::IdeaNotFound(idea_id.to_string()))?;
    if !entry.consumed_by.iter().any(|c| c == consumer) {
        entry.consumed_by.push(consumer.to_string());
    }
    entry.updated_at = now.to_string();
    Ok(entry.clone())
}

pub fn filter_ideas<'a>(
    ledger: &'a IdeaLedger,
    domain: Option<&str>,
    status: Option<&str>,
    freshness: Option<&str>,
) -> Vec<&'a IdeaEntry> {
    ledger
        .ideas
        .iter()
        .filter(|i| domain.is_none_or(|d| i.domain == d))
        .filter(|i| status.is_none_or(|s| i.status == s))
        .filter(|i| freshness.is_none_or(|f| i.freshness == f))
        .collect()
}

pub fn generate_idea_id(existing: &[IdeaEntry], now_unix_s: u64) -> String {
    let base = format!("idea-{now_unix_s}");
    if !existing.iter().any(|i| i.idea_id == base) {
        return base;
    }
    let mut i = 1u64;
    loop {
        let candidate = format!("{base}-{i}");
        if !existing.iter().any(|e| e.idea_id == candidate) {
            return candidate;
        }
        i += 1;
    }
}
