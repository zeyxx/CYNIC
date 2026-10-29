//! Adapter bridging cynic-knowledge epistemic facts to cynic-surreal records.

use crate::domain::epistemic_facts::VerifiableClaim;
use cynic_surreal::domain::memory::EpistemicFactRecord;

/// Maps a VerifiableClaim to a SurrealDB epistemic fact record.
pub fn to_surreal_fact_record(claim: &VerifiableClaim) -> EpistemicFactRecord {
    claim.to_surreal_fact_record()
}
