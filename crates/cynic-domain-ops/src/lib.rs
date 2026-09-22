//! cynic-domain-ops: hex-dominated domain for external domain operations.
//!
//! Pure domain logic for registrar/DNS provider lifecycle management:
//! domain availability, registration, nameserver delegation, zone migration,
//! and authoritative verification.
//!
//! Hexagonal layout:
//! - `domain/` — pure domain logic (record transform, reconciliation, idempotency).
//!   Zero I/O: no external process execution, no HTTP client, no async spawning.
//! - `port.rs` — driven ports (trait definitions for registrar, DNS, edge probe).
//! - `runner.rs` — orchestration over injected ports (Phase 4).
//! - `adapters/` — provider HTTP clients (Phase 3).
//!
//! Types are imported from `cynic_protocol::domain_ops` (pure protocol types).
//! All decision logic is deterministic and testable without network access.
//!
//! Authority: foundation.json §32 (provider_agnostic_domain), §37 (portable_host_contract).

pub mod domain;
pub mod port;
pub mod runner;

pub mod contracts;

pub use crate::domain::{
    reconcile_zone, transform_record_for_cloudflare, generate_idempotency_key,
    policy_for_record_type, classify_record,
};
pub use crate::port::{
    CloudflareDnsPort, EdgeProbePort, RegistrarPort, SecretProvider,
};
pub use crate::runner::run_domain_ops;

// Contract deserialization and proof construction.
pub use crate::contracts::{DomainOpsError, build_proof, parse_desired_state, proof_to_json};

// Re-export protocol types for convenience.
pub use cynic_protocol::domain_ops::{
    DnsRecordShape, DnsRecordType, ProxyMode, ProxyPolicy, ZoneRecordDiff,
    DomainOperation, DomainOperationProof, ProofStatus,
    DomainOpsDesiredState, ProviderConfig, EdgeProbeSpec,
    DomainAvailability, RegistrationReceipt, NameserverUpdateReceipt,
    ZoneMigrationResult, DelegationProof, EdgeProbeResult, SkippedRecord,
};
