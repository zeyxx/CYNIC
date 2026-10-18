//! Core types for cynic-finance.
//!
//! All types are pure data — no I/O, no side effects.
//! Schemas follow the `cynic.finance.*` naming convention.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Schema identifier for resource transactions.
pub const LEDGER_TRANSACTION_SCHEMA: &str = "cynic.finance.resource_transaction.v1";

/// Schema identifier for exchange rates.
pub const EXCHANGE_RATE_SCHEMA: &str = "cynic.finance.exchange_rate.v1";

/// Schema identifier for exchange rate tables.
pub const EXCHANGE_RATE_TABLE_SCHEMA: &str = "cynic.finance.exchange_rate_table.v1";

/// Resource categories for ledger grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Compute,
    Gpu,
    Storage,
    Network,
    Energy,
    Token,
    Context,
    Time,
    Action,
}

/// A measured resource unit. Each variant is measured independently.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResourceUnit {
    ComputeCpuMs(u64),
    ComputeGpuMs(u64),
    ComputeVramBytes(u64),
    StorageBytes(u64),
    StorageIops(u64),
    NetworkBytes(u64),
    NetworkPackets(u64),
    EnergyWatts(f64),
    EnergyKwh(f64),
    TokenCyd(u64),
    ContextTokens(u64),
    TimeSeconds(u64),
    Actions(u64),
}

impl ResourceUnit {
    /// Returns the resource type for this unit.
    pub fn resource_type(&self) -> ResourceType {
        match self {
            ResourceUnit::ComputeCpuMs(_) => ResourceType::Compute,
            ResourceUnit::ComputeGpuMs(_) => ResourceType::Gpu,
            ResourceUnit::ComputeVramBytes(_) => ResourceType::Gpu,
            ResourceUnit::StorageBytes(_) => ResourceType::Storage,
            ResourceUnit::StorageIops(_) => ResourceType::Storage,
            ResourceUnit::NetworkBytes(_) => ResourceType::Network,
            ResourceUnit::NetworkPackets(_) => ResourceType::Network,
            ResourceUnit::EnergyWatts(_) => ResourceType::Energy,
            ResourceUnit::EnergyKwh(_) => ResourceType::Energy,
            ResourceUnit::TokenCyd(_) => ResourceType::Token,
            ResourceUnit::ContextTokens(_) => ResourceType::Context,
            ResourceUnit::TimeSeconds(_) => ResourceType::Time,
            ResourceUnit::Actions(_) => ResourceType::Action,
        }
    }

    /// Returns the numeric value as f64 for exchange rate computation.
    pub fn as_f64(&self) -> f64 {
        match self {
            ResourceUnit::ComputeCpuMs(v) => *v as f64,
            ResourceUnit::ComputeGpuMs(v) => *v as f64,
            ResourceUnit::ComputeVramBytes(v) => *v as f64,
            ResourceUnit::StorageBytes(v) => *v as f64,
            ResourceUnit::StorageIops(v) => *v as f64,
            ResourceUnit::NetworkBytes(v) => *v as f64,
            ResourceUnit::NetworkPackets(v) => *v as f64,
            ResourceUnit::EnergyWatts(v) => *v,
            ResourceUnit::EnergyKwh(v) => *v,
            ResourceUnit::TokenCyd(v) => *v as f64,
            ResourceUnit::ContextTokens(v) => *v as f64,
            ResourceUnit::TimeSeconds(v) => *v as f64,
            ResourceUnit::Actions(v) => *v as f64,
        }
    }
}

/// Direction of a resource transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionDirection {
    /// Consumption — decreases balance.
    Debit,
    /// Allocation or reward — increases balance.
    Credit,
}

/// Metadata attached to a resource transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResourceMetadata {
    /// Optional workload or mandate identifier.
    pub workload_id: Option<String>,
    /// Optional note or reason.
    pub reason: Option<String>,
    /// Optional proof reference (hash of evidence).
    pub proof_ref: Option<String>,
}

/// A single resource transaction — append-only, idempotent, double-entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceTransaction {
    pub schema: String,
    pub transaction_id: String,
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
    /// Subject: agent_id or node_id.
    pub subject: String,
    pub resource: ResourceUnit,
    pub direction: TransactionDirection,
    /// Counterparty: destination account for debits, source account for credits.
    pub counterparty: String,
    /// SHA-256 hash of (transaction_id + timestamp + subject + resource + direction + counterparty).
    pub proof: String,
    pub metadata: ResourceMetadata,
}

impl ResourceTransaction {
    /// Creates a new resource transaction with computed proof hash.
    pub fn new(
        transaction_id: impl Into<String>,
        timestamp: u64,
        subject: impl Into<String>,
        resource: ResourceUnit,
        direction: TransactionDirection,
        counterparty: impl Into<String>,
        metadata: ResourceMetadata,
    ) -> Self {
        let transaction_id = transaction_id.into();
        let subject = subject.into();
        let counterparty = counterparty.into();

        let proof = Self::compute_proof(
            &transaction_id,
            timestamp,
            &subject,
            &resource,
            &direction,
            &counterparty,
        );

        Self {
            schema: LEDGER_TRANSACTION_SCHEMA.to_string(),
            transaction_id,
            timestamp,
            subject,
            resource,
            direction,
            counterparty,
            proof,
            metadata,
        }
    }

    /// Computes the proof hash for a transaction.
    pub fn compute_proof(
        transaction_id: &str,
        timestamp: u64,
        subject: &str,
        resource: &ResourceUnit,
        direction: &TransactionDirection,
        counterparty: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(transaction_id.as_bytes());
        hasher.update(timestamp.to_be_bytes());
        hasher.update(subject.as_bytes());
        hasher.update(format!("{:?}", resource).as_bytes());
        hasher.update(format!("{:?}", direction).as_bytes());
        hasher.update(counterparty.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Verifies the proof hash matches the transaction data.
    pub fn verify_proof(&self) -> bool {
        let computed = Self::compute_proof(
            &self.transaction_id,
            self.timestamp,
            &self.subject,
            &self.resource,
            &self.direction,
            &self.counterparty,
        );
        computed == self.proof
    }
}

/// Build metrics for a single crate — records build, sync, and test timings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildMetrics {
    /// Name of the crate measured.
    pub crate_name: String,
    /// Build time in milliseconds.
    pub build_time_ms: u64,
    /// Sync time in milliseconds.
    pub sync_time_ms: u64,
    /// Test time in milliseconds.
    pub test_time_ms: u64,
    /// Whether the build cache was hit.
    pub cache_hit: bool,
}

impl BuildMetrics {
    /// Creates a new `BuildMetrics` instance.
    pub fn new(
        crate_name: impl Into<String>,
        build_time_ms: u64,
        sync_time_ms: u64,
        test_time_ms: u64,
        cache_hit: bool,
    ) -> Self {
        Self {
            crate_name: crate_name.into(),
            build_time_ms,
            sync_time_ms,
            test_time_ms,
            cache_hit,
        }
    }

    /// Returns the total time (build + sync + test) in milliseconds.
    pub fn total_time_ms(&self) -> u64 {
        self.build_time_ms + self.sync_time_ms + self.test_time_ms
    }
}

impl From<&BuildMetrics> for ResourceUnit {
    fn from(metrics: &BuildMetrics) -> Self {
        ResourceUnit::ComputeCpuMs(metrics.build_time_ms)
    }
}

/// A resource ledger — append-only, monotone, idempotent, double-entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceLedger {
    pub ledger_id: String,
    pub resource_type: ResourceType,
    /// Append-only entries. Never modified or deleted.
    pub entries: Vec<ResourceTransaction>,
    /// Derived balance — computed from entries, never stored independently.
    pub balance: ResourceUnit,
}

impl ResourceLedger {
    /// Creates a new empty ledger for a given resource type.
    pub fn new(ledger_id: impl Into<String>, resource_type: ResourceType) -> Self {
        let balance = match resource_type {
            ResourceType::Compute => ResourceUnit::ComputeCpuMs(0),
            ResourceType::Gpu => ResourceUnit::ComputeGpuMs(0),
            ResourceType::Storage => ResourceUnit::StorageBytes(0),
            ResourceType::Network => ResourceUnit::NetworkBytes(0),
            ResourceType::Energy => ResourceUnit::EnergyWatts(0.0),
            ResourceType::Token => ResourceUnit::TokenCyd(0),
            ResourceType::Context => ResourceUnit::ContextTokens(0),
            ResourceType::Time => ResourceUnit::TimeSeconds(0),
            ResourceType::Action => ResourceUnit::Actions(0),
        };

        Self {
            ledger_id: ledger_id.into(),
            resource_type,
            entries: Vec::new(),
            balance,
        }
    }

    /// Returns the number of entries in the ledger.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the ledger has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the current balance.
    pub fn balance(&self) -> &ResourceUnit {
        &self.balance
    }

    /// Returns all entries.
    pub fn entries(&self) -> &[ResourceTransaction] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_metrics_new() {
        let m = BuildMetrics::new("cynic-finance", 1500, 200, 800, false);
        assert_eq!(m.crate_name, "cynic-finance");
        assert_eq!(m.build_time_ms, 1500);
        assert_eq!(m.sync_time_ms, 200);
        assert_eq!(m.test_time_ms, 800);
        assert!(!m.cache_hit);
    }

    #[test]
    fn build_metrics_total_time_ms() {
        let m = BuildMetrics::new("cynic-core", 1000, 500, 300, true);
        assert_eq!(m.total_time_ms(), 1800);
    }

    #[test]
    fn build_metrics_to_resource_unit() {
        let m = BuildMetrics::new("cynic-build", 2500, 100, 400, false);
        let ru: ResourceUnit = (&m).into();
        assert_eq!(ru, ResourceUnit::ComputeCpuMs(2500));
        assert_eq!(ru.resource_type(), ResourceType::Compute);
    }

    #[test]
    fn build_metrics_cache_hit_flag() {
        let m = BuildMetrics::new("cynic-proto", 0, 50, 0, true);
        assert!(m.cache_hit);
        assert_eq!(m.total_time_ms(), 50);
    }

    #[test]
    fn build_metrics_serialization_roundtrip() {
        let m = BuildMetrics::new("cynic-test", 100, 20, 30, true);
        let json = serde_json::to_string(&m).expect("serialize");
        let deserialized: BuildMetrics = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(m, deserialized);
    }
}
