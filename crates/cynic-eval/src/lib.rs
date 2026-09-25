//! cynic-eval: evaluation domain (consolidated per R7 — one crate per
//! cohesive functional domain).
//!
//! Modules (internal structure, not separate crates):
//! - `eval` — dog scoring, dog health, wisdom, hygiene.
//! - `judgment` — CynicJudgment, Cynefin domains, DIKW layers, sage phases.
//! - `kyon` — decision engine over judgments.
//! - `emergence_gallery` — emergence observation gallery.
//! - `convergence` — causal convergence, coherence, epoch index.
//! - `evidence_journal` — cold-evidence journal contract (JSONL, hash-chained).
//! - `evidence_projection` — journal → storage projection replay.
//! - `agent_execution_provenance` — execution provenance seals.
//!
//! Shared evidence vocabulary (EvidenceEnvelope, AgentEvidenceRef,
//! AgentBudget, digest_json) stays in the protocol residue — it is
//! cross-family vocabulary consumed by 19 residue modules.
//!
//! Extracted from `cynic-protocol` (Session 1 Vague B).

pub mod agent_execution_provenance;
pub mod convergence;
pub mod emergence_gallery;
pub mod evidence_journal;
pub mod evidence_projection;
pub mod eval;
pub mod judge_patch;
pub mod judgment;
pub mod kyon;

pub use emergence_gallery::*;
pub use eval::*;
pub use judge_patch::*;
pub use judgment::*;
pub use kyon::*;
