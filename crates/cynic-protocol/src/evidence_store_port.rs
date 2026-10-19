//! Content-addressed evidence store port.
//!
//! WHY HERE AND NOT IN A HIGHER LAYER. The trait is pure and zero-I/O: it
//! names a contract, it performs no work. It was originally declared in
//! `cynic-kernel` (L3) and re-exported by `cynic-host` (L1), which made L1
//! depend on L3 — an ascending edge that the layer audit tolerates only as
//! recorded debt. The debt was paid once (6cf827fc, "fix(debt): pay
//! host-kernel edge") and reintroduced by the external-port adapter work.
//!
//! `cynic-protocol` is L0 and already owns `EvidenceEnvelope`, the only
//! argument type in the signature. Declaring the trait beside the type it
//! speaks about removes the L1 -> L3 edge instead of recording it again.
//!
//! Zero-I/O core: this module contains no adapter and performs no I/O.

use crate::evidence::EvidenceEnvelope;

/// Persists and reads content-addressed evidence envelopes.
pub trait ContentAddressedEvidenceStorePort: Send + Sync {
    fn persist(&self, envelope: &EvidenceEnvelope) -> Result<String, String>;

    fn read(&self, _digest: &str) -> Result<Option<EvidenceEnvelope>, String> {
        Err("evidence_read_not_supported".to_string())
    }
}
