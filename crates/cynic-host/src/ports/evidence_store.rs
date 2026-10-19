// ContentAddressedEvidenceStorePort is declared in `cynic-protocol` (L0),
// beside the `EvidenceEnvelope` type it speaks about.
//
// WHY. It used to live in `cynic-kernel` (L3) and be re-exported here, which
// made L1 depend on L3 — an ascending edge. The layer audit tolerates such an
// edge only as recorded debt, and that debt was paid once (6cf827fc) then
// reintroduced. Re-exporting the L0 declaration keeps every existing
// `cynic_host::ports::evidence_store::ContentAddressedEvidenceStorePort`
// path working while removing the ascending dependency.
//
// Zero-I/O core: a re-export carries no logic and performs no I/O.
pub use cynic_protocol::evidence_store_port::ContentAddressedEvidenceStorePort;
