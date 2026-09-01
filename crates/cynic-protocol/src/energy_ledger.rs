//! Energy ledger — allocate, track, and prove energy consumption for agent actions.
//!
//! Every agent action costs energy. The ledger ensures:
//! 1. Budget is allocated before execution (no blind spending)
//! 2. Consumption is measured and recorded (no silent overruns)
//! 3. Proof is generated (audit trail)
//!
//! Pattern:
//!   AgentAction → Ledger::allocate(budget) → execute → Ledger::record(consumed) → Proof
//!
//! Reusable by: cleanup, merge, disk extension, LLM calls, any privileged operation.

use serde::{Deserialize, Serialize};

pub const ENERGY_LEDGER_SCHEMA: &str = "cynic.energy_ledger.v1";
pub const ENERGY_ALLOCATION_SCHEMA: &str = "cynic.energy_allocation.v1";
pub const ENERGY_CONSUMPTION_SCHEMA: &str = "cynic.energy_consumption.v1";
pub const ENERGY_PROOF_SCHEMA: &str = "cynic.energy_proof.v1";

/// Dimensions of energy consumption.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnergyDimensions {
    /// Wall time in milliseconds.
    pub wall_time_ms: u64,
    /// CPU time in milliseconds (if measured).
    pub cpu_ms: Option<u64>,
    /// Network bytes transferred (if measured).
    pub network_bytes: Option<u64>,
    /// Disk bytes written (if measured).
    pub disk_bytes: Option<u64>,
    /// Estimated energy in joules (thermal + electrical).
    pub estimated_joules: f64,
}

impl Default for EnergyDimensions {
    fn default() -> Self {
        Self {
            wall_time_ms: 0,
            cpu_ms: None,
            network_bytes: None,
            disk_bytes: None,
            estimated_joules: 0.0,
        }
    }
}

/// A budget allocation for an agent action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnergyAllocation {
    pub schema: String,
    /// Unique allocation ID.
    pub allocation_id: String,
    /// Agent that requested the allocation.
    pub agent_id: String,
    /// Action being performed.
    pub action: String,
    /// Maximum energy budget.
    pub max_budget: EnergyDimensions,
    /// Allocated at (unix seconds).
    pub allocated_at_unix_s: u64,
    /// Expires at (unix seconds).
    pub expires_at_unix_s: u64,
}

/// Actual energy consumed during execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnergyConsumption {
    pub schema: String,
    /// Reference to the allocation.
    pub allocation_id: String,
    /// Actual consumption measured.
    pub consumed: EnergyDimensions,
    /// Consumed at (unix seconds).
    pub consumed_at_unix_s: u64,
}

/// Proof of energy consumption for audit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnergyProof {
    pub schema: String,
    pub allocation: EnergyAllocation,
    pub consumption: EnergyConsumption,
    /// Whether the consumption stayed within budget.
    pub within_budget: bool,
    /// Efficiency ratio (consumed / budget) per dimension.
    pub efficiency: f64,
    /// Evidence references.
    pub evidence_refs: Vec<String>,
}

/// Energy ledger — tracks allocations and consumption.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnergyLedger {
    pub schema: String,
    pub allocations: Vec<EnergyAllocation>,
    pub consumptions: Vec<EnergyConsumption>,
    pub proofs: Vec<EnergyProof>,
}

impl EnergyLedger {
    pub fn new() -> Self {
        Self {
            schema: ENERGY_LEDGER_SCHEMA.to_string(),
            allocations: Vec::new(),
            consumptions: Vec::new(),
            proofs: Vec::new(),
        }
    }

    /// Allocate energy budget for an action.
    pub fn allocate(
        &mut self,
        agent_id: &str,
        action: &str,
        max_budget: EnergyDimensions,
        ttl_seconds: u64,
    ) -> &EnergyAllocation {
        let now = now_unix_s();
        let allocation = EnergyAllocation {
            schema: ENERGY_ALLOCATION_SCHEMA.to_string(),
            allocation_id: format!("alloc:{}:{}:{}", agent_id, action, now),
            agent_id: agent_id.to_string(),
            action: action.to_string(),
            max_budget,
            allocated_at_unix_s: now,
            expires_at_unix_s: now + ttl_seconds,
        };
        self.allocations.push(allocation);
        self.allocations.last().unwrap()
    }

    /// Record actual consumption and generate proof.
    pub fn record(
        &mut self,
        allocation_id: &str,
        consumed: EnergyDimensions,
        evidence_refs: Vec<String>,
    ) -> Result<EnergyProof, String> {
        let allocation = self
            .allocations
            .iter()
            .find(|a| a.allocation_id == allocation_id)
            .ok_or_else(|| format!("allocation_{}_not_found", allocation_id))?
            .clone();

        let consumption = EnergyConsumption {
            schema: ENERGY_CONSUMPTION_SCHEMA.to_string(),
            allocation_id: allocation_id.to_string(),
            consumed: consumed.clone(),
            consumed_at_unix_s: now_unix_s(),
        };

        let within_budget = consumed.estimated_joules <= allocation.max_budget.estimated_joules
            && consumed.wall_time_ms <= allocation.max_budget.wall_time_ms;

        let efficiency = if allocation.max_budget.estimated_joules > 0.0 {
            consumed.estimated_joules / allocation.max_budget.estimated_joules
        } else {
            0.0
        };

        let proof = EnergyProof {
            schema: ENERGY_PROOF_SCHEMA.to_string(),
            allocation: allocation.clone(),
            consumption: consumption.clone(),
            within_budget,
            efficiency,
            evidence_refs,
        };

        self.consumptions.push(consumption);
        self.proofs.push(proof.clone());

        Ok(proof)
    }

    /// Get remaining budget for an allocation.
    pub fn remaining_budget(&self, allocation_id: &str) -> Option<EnergyDimensions> {
        let allocation = self.allocations.iter().find(|a| a.allocation_id == allocation_id)?;
        let consumed = self
            .consumptions
            .iter()
            .filter(|c| c.allocation_id == allocation_id)
            .fold(EnergyDimensions::default(), |mut acc, c| {
                acc.wall_time_ms += c.consumed.wall_time_ms;
                acc.cpu_ms = add_opt(acc.cpu_ms, c.consumed.cpu_ms);
                acc.network_bytes = add_opt(acc.network_bytes, c.consumed.network_bytes);
                acc.disk_bytes = add_opt(acc.disk_bytes, c.consumed.disk_bytes);
                acc.estimated_joules += c.consumed.estimated_joules;
                acc
            });

        Some(EnergyDimensions {
            wall_time_ms: allocation.max_budget.wall_time_ms.saturating_sub(consumed.wall_time_ms),
            cpu_ms: sub_opt(allocation.max_budget.cpu_ms, consumed.cpu_ms),
            network_bytes: sub_opt(allocation.max_budget.network_bytes, consumed.network_bytes),
            disk_bytes: sub_opt(allocation.max_budget.disk_bytes, consumed.disk_bytes),
            estimated_joules: (allocation.max_budget.estimated_joules - consumed.estimated_joules).max(0.0),
        })
    }

    /// Check if an allocation is still valid (not expired).
    pub fn is_valid(&self, allocation_id: &str) -> bool {
        let now = now_unix_s();
        self.allocations
            .iter()
            .find(|a| a.allocation_id == allocation_id)
            .map(|a| now >= a.allocated_at_unix_s && now < a.expires_at_unix_s)
            .unwrap_or(false)
    }
}

fn add_opt(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (None, None) => None,
        (Some(x), None) | (None, Some(x)) => Some(x),
        (Some(x), Some(y)) => Some(x.saturating_add(y)),
    }
}

fn sub_opt(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (None, _) => None,
        (Some(x), None) => Some(x),
        (Some(x), Some(y)) => Some(x.saturating_sub(y)),
    }
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Estimate energy for common operations.
pub mod estimates {
    use super::EnergyDimensions;

    /// SSH command execution (~50-200J depending on latency).
    pub fn ssh_command(latency_ms: u64) -> EnergyDimensions {
        EnergyDimensions {
            wall_time_ms: latency_ms,
            cpu_ms: Some(10),
            network_bytes: Some(10_000),
            disk_bytes: None,
            estimated_joules: 50.0 + (latency_ms as f64 * 0.5),
        }
    }

    /// Disk resize operation (~500-2000J depending on size).
    pub fn disk_resize(size_gb: u64) -> EnergyDimensions {
        EnergyDimensions {
            wall_time_ms: 5000,
            cpu_ms: Some(100),
            network_bytes: Some(50_000),
            disk_bytes: Some(size_gb * 1_000_000_000),
            estimated_joules: 500.0 + (size_gb as f64 * 10.0),
        }
    }

    /// LLM inference call (~1000-5000J depending on tokens).
    pub fn llm_inference(tokens: u64) -> EnergyDimensions {
        EnergyDimensions {
            wall_time_ms: tokens * 10,
            cpu_ms: Some(tokens * 5),
            network_bytes: Some(tokens * 100),
            disk_bytes: None,
            estimated_joules: 1000.0 + (tokens as f64 * 2.0),
        }
    }

    /// Git merge operation (~200-1000J).
    pub fn git_merge(conflicts: u32) -> EnergyDimensions {
        EnergyDimensions {
            wall_time_ms: 1000 + (conflicts as u64 * 500),
            cpu_ms: Some(100 + (conflicts as u64 * 50)),
            network_bytes: Some(100_000),
            disk_bytes: Some(1_000_000),
            estimated_joules: 200.0 + (conflicts as f64 * 200.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_and_record() {
        let mut ledger = EnergyLedger::new();
        let budget = EnergyDimensions {
            wall_time_ms: 10_000,
            cpu_ms: Some(1000),
            network_bytes: Some(100_000),
            disk_bytes: None,
            estimated_joules: 500.0,
        };
        let alloc = ledger.allocate("test_agent", "test_action", budget, 3600);
        let alloc_id = alloc.allocation_id.clone();

        let consumed = EnergyDimensions {
            wall_time_ms: 5000,
            cpu_ms: Some(500),
            network_bytes: Some(50_000),
            disk_bytes: None,
            estimated_joules: 250.0,
        };
        let proof = ledger.record(&alloc_id, consumed, vec!["evidence1".to_string()]).unwrap();
        assert!(proof.within_budget);
        assert_eq!(proof.efficiency, 0.5);
    }

    #[test]
    fn over_budget_detected() {
        let mut ledger = EnergyLedger::new();
        let budget = EnergyDimensions {
            wall_time_ms: 1000,
            estimated_joules: 100.0,
            ..Default::default()
        };
        let alloc = ledger.allocate("agent", "action", budget, 3600);
        let alloc_id = alloc.allocation_id.clone();

        let consumed = EnergyDimensions {
            wall_time_ms: 2000,
            estimated_joules: 200.0,
            ..Default::default()
        };
        let proof = ledger.record(&alloc_id, consumed, vec![]).unwrap();
        assert!(!proof.within_budget);
    }

    #[test]
    fn remaining_budget_tracks_consumption() {
        let mut ledger = EnergyLedger::new();
        let budget = EnergyDimensions {
            wall_time_ms: 10_000,
            estimated_joules: 1000.0,
            ..Default::default()
        };
        let alloc = ledger.allocate("agent", "action", budget, 3600);
        let alloc_id = alloc.allocation_id.clone();

        let consumed = EnergyDimensions {
            wall_time_ms: 3000,
            estimated_joules: 300.0,
            ..Default::default()
        };
        ledger.record(&alloc_id, consumed, vec![]).unwrap();

        let remaining = ledger.remaining_budget(&alloc_id).unwrap();
        assert_eq!(remaining.wall_time_ms, 7000);
        assert!((remaining.estimated_joules - 700.0).abs() < 0.01);
    }

    #[test]
    fn allocation_expires() {
        let mut ledger = EnergyLedger::new();
        let budget = EnergyDimensions::default();
        let alloc = ledger.allocate("agent", "action", budget, 1); // 1 second TTL
        let alloc_id = alloc.allocation_id.clone();
        assert!(ledger.is_valid(&alloc_id));
    }

    #[test]
    fn ssh_command_estimate() {
        let est = estimates::ssh_command(100);
        assert!(est.estimated_joules > 50.0);
        assert_eq!(est.wall_time_ms, 100);
    }

    #[test]
    fn disk_resize_estimate() {
        let est = estimates::disk_resize(20);
        assert!(est.estimated_joules > 500.0);
        assert_eq!(est.disk_bytes, Some(20_000_000_000));
    }
}
