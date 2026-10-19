// ContentAddressedEvidenceStorePort is declared in `cynic-protocol` (L0).
//
// WHY. The trait is pure and zero-I/O, and its only argument type
// (`EvidenceEnvelope`) is L0 too. Declaring it here made L3 own a contract
// that L1 needed, producing the L1 -> L3 ascending edge that the layer
// audit records as debt. The L0 declaration removes the edge instead of
// recording it again.
//
// Zero-I/O core: a re-export carries no logic and performs no I/O.
pub use cynic_protocol::evidence_store_port::ContentAddressedEvidenceStorePort;
