//! Pure task-ledger domain logic (FCIS: no I/O).
//!
//! A task ledger is a small, append/update-friendly list of kairotic and
//! operational opportunities. This module owns types, validation, and
//! in-memory transformations only. File I/O lives in the adapter surface.

use serde::{Deserialize, Serialize};

pub const TASK_LEDGER_SCHEMA: &str = "cynic.task_ledger.v1";
pub const DEFAULT_LEDGER_STATUS: &str = "seed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLedger {
    pub schema: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub tasks: Vec<TaskEntry>,
}

impl TaskLedger {
    pub fn new_empty() -> Self {
        Self {
            schema: TASK_LEDGER_SCHEMA.to_string(),
            status: DEFAULT_LEDGER_STATUS.to_string(),
            tasks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEntry {
    pub task_id: String,
    pub title: String,
    pub priority: String,
    pub status: String,
    pub kind: String,
    #[serde(default)]
    pub link: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    EmptyTitle,
    EmptyTaskId,
    InvalidKind(String),
    DuplicateTaskId(String),
    TaskNotFound(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyTitle => write!(f, "task title must not be empty"),
            Self::EmptyTaskId => write!(f, "task id must not be empty"),
            Self::InvalidKind(kind) => write!(
                f,
                "invalid task kind '{kind}': expected 'kairotic' or 'operational'"
            ),
            Self::DuplicateTaskId(id) => write!(f, "task id already exists: {id}"),
            Self::TaskNotFound(id) => write!(f, "task not found: {id}"),
        }
    }
}

pub fn is_valid_kind(kind: &str) -> bool {
    matches!(kind, "kairotic" | "operational")
}

pub fn add_task(ledger: &mut TaskLedger, entry: TaskEntry) -> Result<(), LedgerError> {
    if entry.title.trim().is_empty() {
        return Err(LedgerError::EmptyTitle);
    }
    if entry.task_id.trim().is_empty() {
        return Err(LedgerError::EmptyTaskId);
    }
    if !is_valid_kind(&entry.kind) {
        return Err(LedgerError::InvalidKind(entry.kind.clone()));
    }
    if ledger.tasks.iter().any(|t| t.task_id == entry.task_id) {
        return Err(LedgerError::DuplicateTaskId(entry.task_id.clone()));
    }
    ledger.tasks.push(entry);
    Ok(())
}

pub fn update_status(
    ledger: &mut TaskLedger,
    task_id: &str,
    status: &str,
    now: &str,
) -> Result<TaskEntry, LedgerError> {
    if task_id.trim().is_empty() {
        return Err(LedgerError::EmptyTaskId);
    }
    if status.trim().is_empty() {
        return Err(LedgerError::EmptyTitle);
    }
    let entry = ledger
        .tasks
        .iter_mut()
        .find(|t| t.task_id == task_id)
        .ok_or_else(|| LedgerError::TaskNotFound(task_id.to_string()))?;
    entry.status = status.to_string();
    entry.updated_at = now.to_string();
    Ok(entry.clone())
}

pub fn filter_tasks<'a>(
    ledger: &'a TaskLedger,
    kind: Option<&str>,
    status: Option<&str>,
    priority: Option<&str>,
) -> Vec<&'a TaskEntry> {
    ledger
        .tasks
        .iter()
        .filter(|t| kind.is_none_or(|k| t.kind == k))
        .filter(|t| status.is_none_or(|s| t.status == s))
        .filter(|t| priority.is_none_or(|p| t.priority == p))
        .collect()
}

pub fn generate_task_id(existing: &[TaskEntry], now_unix_s: u64) -> String {
    let base = format!("task-{now_unix_s}");
    if !existing.iter().any(|t| t.task_id == base) {
        return base;
    }
    let mut i = 1u64;
    loop {
        let candidate = format!("{base}-{i}");
        if !existing.iter().any(|t| t.task_id == candidate) {
            return candidate;
        }
        i += 1;
    }
}
