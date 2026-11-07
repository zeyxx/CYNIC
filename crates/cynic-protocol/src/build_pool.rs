//! Pure build-pool observation DTOs (L0).
//!
//! Moved from `cynic-build` (L2) so host-side adapters (Forgejo run
//! observer, local toolchain probe) can name the observations they
//! produce without an upward L1 -> L2 dependency. `cynic-build`
//! re-exports these types at their historical paths.

use serde::{Deserialize, Serialize};

/// One observed job execution on a pool runner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct JobObservation {
    /// Workflow run id (Forgejo task id).
    pub run_id: i64,
    /// Runner that executed the job.
    pub runner: String,
    /// Job label the runner was matched on (e.g. `cynic/build`).
    pub label: String,
    /// Terminal state: `success`, `failure`, `cancelled`.
    pub status: String,
    /// Commit the job executed.
    #[serde(default)]
    pub head_sha: String,
    /// Execution identifier: the workflow execution the job belongs to.
    ///
    /// Forgejo exposes one run per job; the jobs of a single gate execution
    /// share this id. It is the unit of concurrency, not the commit.
    #[serde(default)]
    pub execution_id: String,
    /// Job name inside the workflow (e.g. `Gate 3 Core Types`).
    #[serde(default)]
    pub job_name: String,
    /// Seconds between the job being queued and the runner picking it up.
    #[serde(default)]
    pub queue_seconds: Option<u64>,
    /// Seconds the job actually ran.
    #[serde(default)]
    pub run_seconds: Option<u64>,
    /// Wall-clock seconds of the whole run (queue + execution).
    #[serde(default)]
    pub wall_seconds: Option<u64>,
    /// Epoch seconds when the job started (unknown stays unknown).
    #[serde(default)]
    pub started_epoch: Option<i64>,
    /// Epoch seconds when the job finished (unknown stays unknown).
    #[serde(default)]
    pub ended_epoch: Option<i64>,
}

/// How a required tool is proven present on a host.
///
/// A build toolchain is not only executables: `libssl-dev` is a set of
/// headers, visible only through `pkg-config`. Probing only `PATH` would
/// report a present system library as missing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// An executable resolvable in `PATH` (e.g. `cargo`).
    #[default]
    Binary,
    /// A pkg-config module (e.g. `openssl` for the `libssl-dev` headers).
    PkgConfig,
}

/// What was actually observed about a tool on a host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ToolObservation {
    pub name: String,
    pub present: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    /// How the observation was made.
    #[serde(default)]
    pub kind: ToolKind,
}
