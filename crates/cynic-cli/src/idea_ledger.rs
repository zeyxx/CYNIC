//! Thin adapter over `cynic_knowledge::idea_ledger`.
//!
//! The idea-ledger domain model and pure logic (FCIS: no I/O) live in
//! `cynic_knowledge`; this module only re-exports the public API so existing
//! consumers keep using `cynic_cli::idea_ledger`.

pub use cynic_knowledge::idea_ledger::{
    DEFAULT_LEDGER_STATUS, IDEA_LEDGER_SCHEMA, VALID_FRESHNESSES, VALID_STATUSES,
    IdeaEntry, IdeaLedger, LedgerError, consume, filter_ideas, generate_idea_id,
    is_valid_freshness, is_valid_status, update_freshness, update_status,
};
