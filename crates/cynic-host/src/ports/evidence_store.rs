// ContentAddressedEvidenceStorePort moved to cynic-kernel (pure trait, zero I/O).
// Re-exported here for existing cynin-host and cynin-node consumers.
pub use cynic_kernel::ports::evidence_store::ContentAddressedEvidenceStorePort;
