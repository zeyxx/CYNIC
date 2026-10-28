//! Read-only SSH target inspection types shared between host transport
//! and SSH trust policy. Pure data, no I/O.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExecutableObservation {
    pub binary: String,
    pub status: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SshHostKeyObservation {
    pub algorithm: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SshTargetInspectionReport {
    pub schema: &'static str,
    pub target: String,
    pub status: String,
    /// Timestamp from the observer that produced the host-key evidence.
    /// A key match without this temporal boundary cannot authorize a route.
    pub observed_at_unix_ms: u64,
    pub ssh: ExecutableObservation,
    pub ssh_keyscan: ExecutableObservation,
    pub host_keys: Vec<SshHostKeyObservation>,
    pub trust_state: String,
    pub mutation: bool,
    pub next_action: String,
    pub detail: Option<String>,
}
