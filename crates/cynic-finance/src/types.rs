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

/// Origin of a resource value — authority ordering: Observed > Derived > Projected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceOrigin {
    /// Read back from runtime (authoritative).
    Observed,
    /// Computed from observed values.
    Derived,
    /// Estimated from historical patterns (lowest authority).
    Projected,
}

/// A business-agnostic resource class. Fixed variants cover the canonical
/// verticals; `Custom` allows runtime registration of new classes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceClass {
    Compute,
    Gpu,
    Memory,
    Storage,
    Bandwidth,
    Latency,
    Energy,
    Token,
    Context,
    Time,
    Action,
    Custom(String),
}

impl ResourceClass {
    /// Returns the canonical string key for registry lookups.
    pub fn as_str(&self) -> &str {
        match self {
            ResourceClass::Compute => "compute",
            ResourceClass::Gpu => "gpu",
            ResourceClass::Memory => "memory",
            ResourceClass::Storage => "storage",
            ResourceClass::Bandwidth => "bandwidth",
            ResourceClass::Latency => "latency",
            ResourceClass::Energy => "energy",
            ResourceClass::Token => "token",
            ResourceClass::Context => "context",
            ResourceClass::Time => "time",
            ResourceClass::Action => "action",
            ResourceClass::Custom(value) => value.as_str(),
        }
    }
}

impl std::fmt::Display for ResourceClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for ResourceClass {
    fn from(value: String) -> Self {
        match value.as_str() {
            "compute" => ResourceClass::Compute,
            "gpu" => ResourceClass::Gpu,
            "memory" => ResourceClass::Memory,
            "storage" => ResourceClass::Storage,
            "bandwidth" => ResourceClass::Bandwidth,
            "latency" => ResourceClass::Latency,
            "energy" => ResourceClass::Energy,
            "token" => ResourceClass::Token,
            "context" => ResourceClass::Context,
            "time" => ResourceClass::Time,
            "action" => ResourceClass::Action,
            other => ResourceClass::Custom(other.to_string()),
        }
    }
}

impl From<&str> for ResourceClass {
    fn from(value: &str) -> Self {
        ResourceClass::from(value.to_string())
    }
}

/// Type aliases for finance-local identifiers and quantities.
pub type ResourceId = String;
pub type Quantity = f64;
pub type Timestamp = u64;
pub type Provenance = String;
pub type ConstraintId = String;

/// A typed, measurable, depletable resource instance (MMORPG "item").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    /// Content-addressed identifier.
    pub id: ResourceId,
    /// Resource class (item type).
    pub class: ResourceClass,
    /// Measured amount.
    pub quantity: Quantity,
    /// Canonical unit.
    pub unit: ResourceUnit,
    /// Authority origin of the value.
    pub origin: ResourceOrigin,
    /// `None` means durable.
    pub expiry: Option<Timestamp>,
    /// Node + measurement provenance.
    pub provenance: Provenance,
}

/// Constraint kinds that gate resource consumption.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintType {
    SlaDeadline,
    ThroughputFloor,
    QualityFloor,
    Availability,
    BudgetCap,
    LatencyCeiling,
    Concurrency,
    Custom(String),
}

impl ConstraintType {
    pub fn as_str(&self) -> &str {
        match self {
            ConstraintType::SlaDeadline => "sla_deadline",
            ConstraintType::ThroughputFloor => "throughput_floor",
            ConstraintType::QualityFloor => "quality_floor",
            ConstraintType::Availability => "availability",
            ConstraintType::BudgetCap => "budget_cap",
            ConstraintType::LatencyCeiling => "latency_ceiling",
            ConstraintType::Concurrency => "concurrency",
            ConstraintType::Custom(value) => value.as_str(),
        }
    }
}

impl std::fmt::Display for ConstraintType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for ConstraintType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "sla_deadline" => ConstraintType::SlaDeadline,
            "throughput_floor" => ConstraintType::ThroughputFloor,
            "quality_floor" => ConstraintType::QualityFloor,
            "availability" => ConstraintType::Availability,
            "budget_cap" => ConstraintType::BudgetCap,
            "latency_ceiling" => ConstraintType::LatencyCeiling,
            "concurrency" => ConstraintType::Concurrency,
            other => ConstraintType::Custom(other.to_string()),
        }
    }
}

impl From<&str> for ConstraintType {
    fn from(value: &str) -> Self {
        ConstraintType::from(value.to_string())
    }
}

/// A typed requirement that gates resource consumption.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    pub id: ConstraintId,
    pub constraint_type: ConstraintType,
    pub value: f64,
    pub source: String,
}

/// Schema describing a registered resource class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceSchema {
    pub class: ResourceClass,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl From<ResourceClass> for ResourceSchema {
    fn from(class: ResourceClass) -> Self {
        Self {
            name: class.as_str().to_string(),
            class,
            unit: None,
            description: None,
        }
    }
}

impl From<String> for ResourceSchema {
    fn from(value: String) -> Self {
        ResourceSchema::from(ResourceClass::from(value))
    }
}

impl From<&str> for ResourceSchema {
    fn from(value: &str) -> Self {
        ResourceSchema::from(ResourceClass::from(value))
    }
}

/// Dynamic registry mapping `ResourceClass -> ResourceSchema`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceRegistry {
    #[serde(default)]
    schemas: std::collections::HashMap<ResourceClass, ResourceSchema>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a schema (or a class name) in the registry.
    pub fn register(&mut self, schema: impl Into<ResourceSchema>) -> Result<(), String> {
        let schema = schema.into();
        if self.schemas.contains_key(&schema.class) {
            return Err(format!(
                "resource class {} already registered",
                schema.class
            ));
        }
        self.schemas.insert(schema.class.clone(), schema);
        Ok(())
    }

    /// Register or replace a schema unconditionally.
    pub fn register_or_replace(&mut self, schema: impl Into<ResourceSchema>) {
        let schema = schema.into();
        self.schemas.insert(schema.class.clone(), schema);
    }

    /// Alias for `register`.
    pub fn add(&mut self, schema: impl Into<ResourceSchema>) -> Result<(), String> {
        self.register(schema)
    }

    /// Alias for `register`.
    pub fn insert(&mut self, schema: impl Into<ResourceSchema>) -> Result<(), String> {
        self.register(schema)
    }

    /// Look up a schema by class.
    pub fn get(&self, class: &ResourceClass) -> Option<&ResourceSchema> {
        self.schemas.get(class)
    }

    /// Look up a schema by string key.
    pub fn get_by_str(&self, key: &str) -> Option<&ResourceSchema> {
        self.schemas.get(&ResourceClass::from(key))
    }

    /// Returns true if the class is registered.
    pub fn contains(&self, class: &ResourceClass) -> bool {
        self.schemas.contains_key(class)
    }

    /// Alias for `contains`.
    pub fn has(&self, class: &ResourceClass) -> bool {
        self.contains(class)
    }

    /// Remove a registered class.
    pub fn remove(&mut self, class: &ResourceClass) -> Option<ResourceSchema> {
        self.schemas.remove(class)
    }

    /// Alias for `remove`.
    pub fn unregister(&mut self, class: &ResourceClass) -> Option<ResourceSchema> {
        self.remove(class)
    }

    /// List all registered schemas.
    pub fn list(&self) -> Vec<&ResourceSchema> {
        self.schemas.values().collect()
    }

    /// Alias for `list`.
    pub fn all(&self) -> Vec<&ResourceSchema> {
        self.list()
    }

    /// Returns all registered classes.
    pub fn classes(&self) -> Vec<ResourceClass> {
        self.schemas.keys().cloned().collect()
    }

    /// Number of registered classes.
    pub fn len(&self) -> usize {
        self.schemas.len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_new_and_fields() {
        let r = Resource {
            id: "res-1".into(),
            class: ResourceClass::Compute,
            quantity: 42.0,
            unit: ResourceUnit::ComputeCpuMs(42),
            origin: ResourceOrigin::Observed,
            expiry: None,
            provenance: "node-1".into(),
        };
        assert_eq!(r.id, "res-1");
        assert_eq!(r.class, ResourceClass::Compute);
        assert_eq!(r.quantity, 42.0);
        assert!(r.expiry.is_none());
    }

    #[test]
    fn resource_serialization_roundtrip() {
        let r = Resource {
            id: "res-2".into(),
            class: ResourceClass::Custom("foo".into()),
            quantity: 7.5,
            unit: ResourceUnit::EnergyWatts(7.5),
            origin: ResourceOrigin::Projected,
            expiry: Some(100),
            provenance: "node-2".into(),
        };
        let json = serde_json::to_string(&r).expect("serialize");
        let deserialized: Resource = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, deserialized);
    }

    #[test]
    fn constraint_new_and_fields() {
        let c = Constraint {
            id: "con-1".into(),
            constraint_type: ConstraintType::BudgetCap,
            value: 100.0,
            source: "ledger".into(),
        };
        assert_eq!(c.id, "con-1");
        assert_eq!(c.constraint_type, ConstraintType::BudgetCap);
        assert_eq!(c.value, 100.0);
    }

    #[test]
    fn constraint_serialization_roundtrip() {
        let c = Constraint {
            id: "con-2".into(),
            constraint_type: ConstraintType::from("custom_x"),
            value: 3.0,
            source: "test".into(),
        };
        let json = serde_json::to_string(&c).expect("serialize");
        let deserialized: Constraint = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c, deserialized);
    }

    #[test]
    fn resource_registry_register_and_lookup() {
        let mut reg = ResourceRegistry::new();
        assert!(reg.is_empty());
        reg.register("compute").expect("register");
        assert!(reg.contains(&ResourceClass::Compute));
        assert_eq!(reg.len(), 1);
        let schema = reg.get(&ResourceClass::Compute).expect("schema");
        assert_eq!(schema.class, ResourceClass::Compute);
        assert!(reg.get_by_str("compute").is_some());
    }

    #[test]
    fn resource_registry_duplicate_rejected() {
        let mut reg = ResourceRegistry::new();
        reg.register(ResourceClass::Gpu).expect("first");
        let result = reg.register(ResourceClass::Gpu);
        assert!(result.is_err());
    }

    #[test]
    fn resource_registry_remove_and_list() {
        let mut reg = ResourceRegistry::default();
        reg.register_or_replace(ResourceSchema {
            class: ResourceClass::Memory,
            name: "memory".into(),
            unit: Some("mb".into()),
            description: None,
        });
        reg.register(ResourceClass::Storage).expect("register");
        assert_eq!(reg.list().len(), 2);
        assert_eq!(reg.classes().len(), 2);
        reg.remove(&ResourceClass::Memory);
        assert_eq!(reg.len(), 1);
        assert!(!reg.contains(&ResourceClass::Memory));
    }

    #[test]
    fn resource_origin_ordering_variants() {
        assert_ne!(ResourceOrigin::Observed, ResourceOrigin::Projected);
    }
}
