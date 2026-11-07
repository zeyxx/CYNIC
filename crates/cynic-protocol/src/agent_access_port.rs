//! Port L0 `agent_access` — opération fermée + traits Observation/Execution.
//!
//! WHY HERE AND NOT IN `cynic-host`. `cynic-host/src/ports/mod.rs` declares
//! `HostOperation` with 13 variants, while `agent_access` (decision +
//! authority projection) consumes only 4 of them. Naming the whole
//! `HostOperation` from an L0 decision would leak the host substrate into
//! the protocol layer. This module is the bounded, closed port instead:
//! exactly those 4 variants, with the same field shape, so the host
//! adapter can translate exhaustively (no wildcard) to `HostOperation`.
//!
//! The 4 consumed variants, measured against `HostOperation`:
//! - `EnsureSystemGroup` — `agent_access/decision.rs`
//! - `EnsureSupplementaryGroupMember` — `agent_access/decision.rs`
//! - `EnsureOwnedDir` — `agent_access/decision.rs`
//! - `InstallOwnedFile` — `agent_access/authority_projection.rs`
//!
//! Pure contract only: enum + traits, zero I/O. The decision/plan DTOs now
//! live in L2 (`cynic-agent/src/agent_access`) and build
//! `AgentAccessOperation`; the host adapter translates them to
//! `HostOperation` exhaustively.

use serde::Serialize;

/// Closed agent-access operation: the 4 `HostOperation` variants that
/// `agent_access` actually consumes. Adding a variant requires updating
/// every exhaustive match over this enum (L0 tests + host adapter).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentAccessOperation {
    EnsureSystemGroup {
        group: String,
    },
    EnsureSupplementaryGroupMember {
        user: String,
        group: String,
    },
    EnsureOwnedDir {
        path: String,
        user: String,
        group: String,
        mode: u32,
    },
    InstallOwnedFile {
        source: String,
        target: String,
        user: String,
        group: String,
        mode: u32,
    },
}

/// Read-only observation boundary for agent access.
///
/// Generic over the observation type `O` because the concrete observation
/// DTO (`AgentAccessObservation`) still lives in `cynic-host` and cannot
/// be named from L0. Implementations must only read the declared group,
/// runtime directory and socket; unknown state is reported through `O`
/// (or `Err`), never promoted to convergence.
pub trait AgentAccessObservationPort<O>: Send + Sync {
    fn observe_agent_access(
        &self,
        access_group: &str,
        runtime_directory: &str,
        socket_path: &str,
    ) -> Result<O, String>;
}

/// Execution boundary for one bounded agent-access operation.
///
/// Implementations must accept only the closed `AgentAccessOperation`
/// variants already admitted by the pure decision; anything else is
/// unrepresentable by construction.
pub trait AgentAccessExecutionPort: Send + Sync {
    fn execute_agent_access_operation(
        &self,
        operation: &AgentAccessOperation,
    ) -> Result<(), String>;
}
