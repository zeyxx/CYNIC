//! StateBlock — hash-chained organism state log.
//!
//! Append-only time-series of CYNIC's own state. Each block hashes the previous,
//! creating a tamper-evident chain. Not a blockchain (no distributed consensus),
//! but a transparency log — like Certificate Transparency for the organism's health.
//!
//! K15 consumers: introspection (trend detection), crystal pipeline (self-awareness),
//! /state-history endpoint (human audit trail).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A single Dog's state at snapshot time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DogSnapshot {
    pub id: String,
    pub circuit: String,
    pub success_rate: f64,
    pub mean_latency_ms: f64,
    pub failures: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_failure_reason: Option<String>,
}

/// System-level state at snapshot time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub status: String,
    pub healthy_dogs: usize,
    pub total_dogs: usize,
    pub verdict_count: u64,
    pub total_tokens: u64,
    pub crystals_forming: usize,
    pub crystals_crystallized: usize,
}

/// Resource usage at snapshot time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    pub cpu_pct: f64,
    pub memory_used_gb: f64,
    pub disk_avail_gb: f64,
    pub uptime_secs: u64,
}

/// Organ/agent liveness at snapshot time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganSnapshot {
    pub source: String,
    pub last_observation: String,
    pub total_observations: u64,
    /// Seconds since last observation. >3600 = likely dead.
    pub silence_secs: u64,
}

/// Full organism state snapshot — one block in the chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateBlock {
    pub seq: u64,
    pub timestamp: String,
    pub prev_hash: String,
    pub dogs: Vec<DogSnapshot>,
    pub system: SystemSnapshot,
    pub resource: ResourceSnapshot,
    #[serde(default)]
    pub organs: Vec<OrganSnapshot>,
    pub hash: String,
}

/// Genesis hash — the "block 0" prev_hash.
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

impl StateBlock {
    /// Build a new block. Computes hash from prev_hash + canonical snapshot.
    pub fn new(
        seq: u64,
        prev_hash: String,
        dogs: Vec<DogSnapshot>,
        system: SystemSnapshot,
        resource: ResourceSnapshot,
        organs: Vec<OrganSnapshot>,
    ) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut block = Self {
            seq,
            timestamp,
            prev_hash,
            dogs,
            system,
            resource,
            organs,
            hash: String::new(),
        };
        block.hash = block.compute_hash();
        block
    }

    /// SHA-256 of prev_hash + canonical JSON of snapshot fields.
    fn compute_hash(&self) -> String {
        let canonical = serde_json::json!({
            "seq": self.seq,
            "timestamp": self.timestamp,
            "prev_hash": self.prev_hash,
            "dogs": self.dogs,
            "system": self.system,
            "resource": self.resource,
            "organs": self.organs,
        });
        let mut hasher = Sha256::new();
        hasher.update(self.prev_hash.as_bytes());
        hasher.update(canonical.to_string().as_bytes());
        let result = hasher.finalize();
        result.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Verify this block's hash is consistent with its data.
    pub fn verify(&self) -> bool {
        self.hash == self.compute_hash()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_block(seq: u64, prev_hash: &str) -> StateBlock {
        StateBlock::new(
            seq,
            prev_hash.to_string(),
            vec![DogSnapshot {
                id: "test-dog".into(),
                circuit: "closed".into(),
                success_rate: 0.95,
                mean_latency_ms: 1500.0,
                failures: 2,
                last_failure_reason: None,
            }],
            SystemSnapshot {
                status: "sovereign".into(),
                healthy_dogs: 5,
                total_dogs: 5,
                verdict_count: 100,
                total_tokens: 50000,
                crystals_forming: 10,
                crystals_crystallized: 3,
            },
            ResourceSnapshot {
                cpu_pct: 12.0,
                memory_used_gb: 14.0,
                disk_avail_gb: 70.0,
                uptime_secs: 3600,
            },
            vec![],
        )
    }

    #[test]
    fn genesis_block_verifies() {
        let block = sample_block(0, GENESIS_HASH);
        assert!(block.verify());
        assert_eq!(block.seq, 0);
        assert_eq!(block.prev_hash, GENESIS_HASH);
    }

    #[test]
    fn chain_links() {
        let b0 = sample_block(0, GENESIS_HASH);
        let b1 = sample_block(1, &b0.hash);
        assert!(b0.verify());
        assert!(b1.verify());
        assert_eq!(b1.prev_hash, b0.hash);
    }

    #[test]
    fn tamper_detection() {
        let mut block = sample_block(0, GENESIS_HASH);
        let original_hash = block.hash.clone();
        block.system.healthy_dogs = 0; // tamper
        assert_ne!(block.compute_hash(), original_hash);
        assert!(!block.verify());
    }

    #[test]
    fn different_data_different_hash() {
        let b1 = sample_block(0, GENESIS_HASH);
        let b2 = StateBlock::new(
            0,
            GENESIS_HASH.to_string(),
            vec![DogSnapshot {
                id: "other-dog".into(),
                circuit: "open".into(),
                success_rate: 0.15,
                mean_latency_ms: 50000.0,
                failures: 500,
                last_failure_reason: Some("timeout".into()),
            }],
            b1.system.clone(),
            b1.resource.clone(),
            vec![],
        );
        assert_ne!(b1.hash, b2.hash);
    }
}
