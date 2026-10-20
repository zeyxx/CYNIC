//! Throughput invoice — the treasurer's bill for one source commit.
//!
//! Pure types only: what a build cost (observed wall time per pipeline
//! proof) and what remains unknown (proofs predating `duration_ms`,
//! unmeasured dimensions). Aggregation lives in `cynic-node`; projection
//! lives in the CLI. Unknowns are data, never zero-filled facts.

use serde::{Deserialize, Serialize};

use crate::energy_ledger::EnergyDimensions;

/// The persisted schema identifier of the invoice envelope.
///
/// The VALUE keeps its legacy spelling on purpose: it is written into a
/// live SQLite table and a JSONL invoice log, and it is declared as the
/// envelope requirement in `infra/agent-roles.json`. Renaming the value
/// would orphan every stored invoice, so only the Rust constant name
/// changes. The legacy name is what the data carries.
pub const THROUGHPUT_INVOICE_SCHEMA: &str = "cynic.workload.metabolic_invoice.v1";

/// True when a document's schema is an accepted spelling of this
/// envelope.
///
/// The compatibility surface lives here rather than in each reader: a
/// document is valid when its schema is the current value, and the
/// current value IS the legacy one, so the two coincide today. The
/// helper exists so that a future value rename has exactly one place
/// to widen, instead of one per reader.
pub fn accepts_throughput_invoice_schema(schema: &str) -> bool {
    schema == THROUGHPUT_INVOICE_SCHEMA
}

/// One invoiced pipeline layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThroughputInvoiceLine {
    pub layer: String,
    pub action: String,
    pub status: String,
    /// Wall time in milliseconds attributed to this line.
    pub wall_time_ms: u64,
    /// False when the source proof predates `duration_ms` or carries
    /// multi-layer time that cannot be attributed per layer.
    pub wall_time_observed: bool,
}

/// Aggregated cost account for one source commit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThroughputInvoice {
    pub schema: String,
    pub source_commit: String,
    pub proofs_observed: usize,
    pub proofs_invoiced: usize,
    pub total: EnergyDimensions,
    pub lines: Vec<ThroughputInvoiceLine>,
    pub residual_unknowns: Vec<String>,
}

impl ThroughputInvoice {
    /// Structural validation: schema plus a non-empty commit reference.
    /// Returns blocking codes, empty when valid.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut blockers = Vec::new();
        if self.schema != THROUGHPUT_INVOICE_SCHEMA {
            blockers.push("throughput_invoice_schema_invalid".to_string());
        }
        if self.source_commit.trim().is_empty() {
            blockers.push("throughput_invoice_commit_empty".to_string());
        }
        if blockers.is_empty() {
            Ok(())
        } else {
            Err(blockers)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_commit_is_rejected() {
        let invoice = ThroughputInvoice {
            schema: THROUGHPUT_INVOICE_SCHEMA.to_string(),
            source_commit: String::new(),
            proofs_observed: 0,
            proofs_invoiced: 0,
            total: EnergyDimensions::default(),
            lines: vec![],
            residual_unknowns: vec!["no proofs observed".to_string()],
        };
        assert!(invoice.validate().is_err());
    }
}
