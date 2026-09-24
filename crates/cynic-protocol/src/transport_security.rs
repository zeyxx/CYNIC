use serde::{Deserialize, Serialize};

/// Shared proof-evidence vocabulary used across build, cli, host, node and vcs
/// surfaces. Kept in the protocol residue because it is cross-family vocabulary,
/// not transport-specific (40+ consumer sites across 5 crates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEvidenceState {
    Proven,
    Refuted,
    Declared,
    Absent,
    Unknown,
    NotApplicable,
}
