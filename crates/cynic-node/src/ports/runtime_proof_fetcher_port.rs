//! 2N+2 fetcher port — abstract peer proof-bundle transport so the
//! federation decision layer stays adapter-free (AGENTS.md §7: "Adapters are
//! disposable; ports and proofs are durable").
//!
//! The transport itself (Phase 3.2 websocket, or an MVP HTTP GET) is injected
//! here. The proof-consumption policy (fleet_reconcile) only depends on the port.

use cynic_protocol::RuntimeProofBundle;

pub trait RuntimeProofFetcherPort: Send + Sync {
    /// Fetch a peer node's `RuntimeProofBundle` by network address.
    /// Returns the bundle or an error. `None`-proof is a read-back failure.
    fn fetch_peer_proof_bundle(
        &self,
        peer_url: &str,
    ) -> Result<RuntimeProofBundle, Box<dyn std::error::Error>>;
}
