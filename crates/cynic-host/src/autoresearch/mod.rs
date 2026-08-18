//! CYNIC Autoresearcher — host-side observation adapter (Phase 2).
//!
//! `TraceFsAdapter` implements `cynic_protocol::AutoresearchPort` by reading
//! local agent session traces (Claude Code JSONL, Codex sessions, Gemini chats)
//! in a STREAMING fashion — never loading the whole corpus (the Windows corpus
//! is ~4 GiB of JSONL) into memory. This is the neutral `prepare.py` of the
//! autoresearch loop: it observes, it does not decide. The metrics, dedup, and
//! keep/discard live in `cynic-protocol::autoresearch` (PURE). The reflection LLM
//! adapter is Phase 3.
//!
//! Observation contract: for each session file we emit `TraceUnit`s tagged by
//! `domain` (derived from the error bucket / MCP server the trace exhibits) and
//! `is_block` (true only when the unit is a *real* failure/block, not a passing
//! mention). This is exactly what `measure_coverage` in the protocol needs: a
//! real block, not a frequency count.

pub mod reflection;
pub mod trace_fs;

pub use trace_fs::{TraceFsAdapter, TraceSource, TraceUnitEmitter};
