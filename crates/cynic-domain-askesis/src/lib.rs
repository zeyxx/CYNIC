#![forbid(unsafe_code)]

//! cynic-domain-askesis — append-only zone backup archival.
//!
//! Provides the `archive_zone_backup` function: serializes a registrar/Cloudflare
//! zone export into a `cynic_askesis::AskesisLog` entry as tamper-evident proof
//! that the pre-mutation state was captured. Called by the runner BEFORE any
//! zone write to satisfy the "backup before write" invariant.
//!
//! Authority: foundation.json §35 (append_only_history), AGENTS.md backup policy.

pub mod archive;

pub use archive::archive_zone_backup;
