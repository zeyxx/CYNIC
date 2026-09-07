//! Metabolic invoice — the treasurer's bill for one source commit.
//!
//! Pure types only: what a build cost (observed wall time per pipeline
//! proof) and what remains unknown (proofs predating `duration_ms`,
//! unmeasured dimensions). Aggregation lives in `cynic-node`; projection
//! lives in the CLI. Unknowns are data, never zero-filled facts.

use serde::{Deserialize, Serialize};

use crate::energy_ledger::EnergyDimensions;

pub const METABOLIC_INVOICE_SCHEMA: &str = "cynic.workload.metabolic_invoice.v1";

/// One invoiced pipeline layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetabolicInvoiceLine {
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
pub struct MetabolicInvoice {
    pub schema: String,
    pub source_commit: String,
    pub proofs_observed: usize,
    pub proofs_invoiced: usize,
    pub total: EnergyDimensions,
    pub lines: Vec<MetabolicInvoiceLine>,
    pub residual_unknowns: Vec<String>,
}

impl MetabolicInvoice {
    /// Structural validation: schema plus a non-empty commit reference.
    /// Returns blocking codes, empty when valid.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut blockers = Vec::new();
        if self.schema != METABOLIC_INVOICE_SCHEMA {
            blockers.push("metabolic_invoice_schema_invalid".to_string());
        }
        if self.source_commit.trim().is_empty() {
            blockers.push("metabolic_invoice_commit_empty".to_string());
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
        let invoice = MetabolicInvoice {
            schema: METABOLIC_INVOICE_SCHEMA.to_string(),
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
