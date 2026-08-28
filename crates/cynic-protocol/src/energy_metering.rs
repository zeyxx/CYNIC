use serde::{Deserialize, Serialize};

pub const ENERGY_BUDGET_SCHEMA: &str = "cynic.energy_budget.v1";
pub const ENERGY_CONSUMPTION_RECEIPT_SCHEMA: &str = "cynic.energy_consumption_receipt.v1";
pub const SUSTAINABLE_COMPUTE_PROOF_SCHEMA: &str = "cynic.sustainable_compute_proof.v1";

/// Energy budget per agent mandate (anti-context-bloat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnergyBudget {
    pub schema: String,
    pub max_energy_joules: u64,
    pub max_dram_gigabytes_transferred: u64,
    pub max_watts_peak: u32,
}

impl EnergyBudget {
    pub fn inference_decode(tokens: u64, joules_per_token: f64) -> Self {
        let total = (tokens as f64 * joules_per_token) as u64;
        Self {
            schema: ENERGY_BUDGET_SCHEMA.to_string(),
            max_energy_joules: total,
            max_dram_gigabytes_transferred: 0,
            max_watts_peak: 65,
        }
    }
}

/// Receipt of actual energy consumption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnergyConsumptionReceipt {
    pub schema: String,
    pub session_id: String,
    pub energy_joules: u64,
    pub dram_gigabytes_transferred: u64,
    pub duration_ms: u64,
    pub observed_at_unix_ms: u64,
}

/// Proof-of-Sustainable-Compute for on-chain attestation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SustainableComputeProof {
    pub schema: String,
    pub node_id: String,
    pub epoch: u64,
    pub tokens_generated: u64,
    pub energy_joules: u64,
    pub tokens_per_joule: f64,
    /// SHA3-512 of the PoH checkpoint batch.
    pub poh_checkpoint_root: String,
}

impl SustainableComputeProof {
    pub fn efficiency(&self) -> f64 {
        if self.energy_joules == 0 {
            0.0
        } else {
            self.tokens_generated as f64 / self.energy_joules as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_budget_for_decode() {
        let b = EnergyBudget::inference_decode(1000, 0.05);
        assert_eq!(b.max_energy_joules, 50);
    }

    #[test]
    fn efficiency_calculation() {
        let p = SustainableComputeProof {
            schema: SUSTAINABLE_COMPUTE_PROOF_SCHEMA.to_string(),
            node_id: "node1".to_string(),
            epoch: 1,
            tokens_generated: 1000,
            energy_joules: 50,
            tokens_per_joule: 20.0,
            poh_checkpoint_root: "abc".to_string(),
        };
        assert!((p.efficiency() - 20.0).abs() < 0.01);
    }
}
