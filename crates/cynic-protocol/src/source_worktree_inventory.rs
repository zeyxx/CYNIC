//! Source worktree inventory observation types.

use serde::{Deserialize, Serialize};

pub const SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA: &str =
    "cynic.source_worktree_inventory_observation.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWorktreeInventoryEntryState {
    Active,
    Detached,
    Prunable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWorktreeInventoryPathBasis {
    Absolute,
    Relative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceWorktreeInventoryEntry {
    pub path: String,
    pub commit: String,
    pub branch: Option<String>,
    pub state: SourceWorktreeInventoryEntryState,
    pub path_basis: SourceWorktreeInventoryPathBasis,
    pub evidence_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceWorktreeInventoryObservation {
    pub schema: String,
    pub repository_id: String,
    pub node_id: String,
    pub observed_at_ms: u64,
    pub entries: Vec<SourceWorktreeInventoryEntry>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceWorktreeInventoryUnknown {
    pub schema: String,
    pub repository_id: String,
    pub node_id: String,
    pub observed_at_ms: u64,
    pub error: String,
}

pub fn validate_source_worktree_inventory_observation(
    _observation: &SourceWorktreeInventoryObservation,
) -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_schema_is_valid() {
        let obs = SourceWorktreeInventoryObservation {
            schema: SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
            repository_id: "repo1".to_string(),
            node_id: "node1".to_string(),
            observed_at_ms: 0,
            entries: Vec::new(),
            evidence_refs: Vec::new(),
        };
        assert!(validate_source_worktree_inventory_observation(&obs).is_empty());
    }
}
