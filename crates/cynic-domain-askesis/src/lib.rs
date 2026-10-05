#![forbid(unsafe_code)]

//! cynic-domain-askesis — append-only zone backup archival.
//!
//! Provides the `archive_zone_backup` function: serializes a registrar/Cloudflare
//! zone export into a `cynic_askesis::AskesisLog` entry as tamper-evident proof
//! that the pre-mutation state was captured. Called by the runner BEFORE any
//! zone write to satisfy the "backup before write" invariant.
//!
//! Authority: `contracts/foundation.json` → `explicit_over_implicit` and
//! `runtime_proof_over_prose`. Append-only history is enforced by the archive
//! module's own read-back; AGENTS.md has no "backup policy" section.

pub mod archive;

pub use archive::archive_zone_backup;
