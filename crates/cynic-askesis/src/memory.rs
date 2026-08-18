//! Askesis memory primitives.

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::log::jsonl::JsonlLog;
use crate::log::{LogStore, ObservationEvent};

/// Default location for operator memory used by MCP surface writes.
pub const DEFAULT_HUMAN_KERNEL_LOG: &str = "/var/lib/cynic/memory/logs/human-kernel.jsonl";

/// Resolve the active human log path for MCP-invoked operations.
pub fn resolve_human_log_path() -> PathBuf {
    if let Ok(path_str) = std::env::var("CYNIC_ASKESIS_LOG_FILE") {
        if !path_str.trim().is_empty() {
            return PathBuf::from(path_str);
        }
    }

    let default_path = PathBuf::from(DEFAULT_HUMAN_KERNEL_LOG);
    if let Some(parent) = default_path.parent() {
        if parent.exists() || std::fs::create_dir_all(parent).is_ok() {
            return default_path;
        }
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".cynic/memory/logs/human-kernel.jsonl")
}

/// Append one observation to Askesis log storage.
pub fn append_human_log_entry(
    logfile: impl AsRef<Path>,
    content: &str,
    domain: &str,
) -> Result<()> {
    let content = content.trim();
    let domain = domain.trim();

    if content.is_empty() {
        return Err(crate::error::AskesisError::InvalidObservationEvent(
            "content must not be empty".to_string(),
        ));
    }
    if domain.is_empty() {
        return Err(crate::error::AskesisError::InvalidObservationEvent(
            "domain must not be empty".to_string(),
        ));
    }

    let mut store = JsonlLog::new(logfile.as_ref())?;
    let entry = ObservationEvent::new(content).with_domain(domain);
    store.append(entry)?;
    Ok(())
}
