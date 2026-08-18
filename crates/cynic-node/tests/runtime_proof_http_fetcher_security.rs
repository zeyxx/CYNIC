//! Phase 3.2-security TDD — HttpRuntimeProofFetcher transport guard.
//!
//! Validates the federation transport adapter rejects plain http:// peers
//! (MITM surface) BEFORE attempting a network call. The RuntimeProofBundle is
//! kernel/node-signed for integrity, but peer admission evidence is sensitive
//! federation topology data — transport MUST use https://.
//!
//! AGENS.md §4: privileged/mutable boundaries bounded, auditable. Read-only
//! proof fetch is not a mutation, but leaking peer topology is a residual risk.

use cynic_node::adapters::runtime_proof_http_fetcher::HttpRuntimeProofFetcher;
use cynic_node::ports::runtime_proof_fetcher_port::RuntimeProofFetcherPort;

#[test]
fn http_scheme_rejected_before_network() {
    let fetcher = HttpRuntimeProofFetcher::new();
    let err = fetcher
        .fetch_peer_proof_bundle("http://peer.insecure:3000")
        .expect_err("plain http:// must be rejected for federation transport");
    let msg = err.to_string();
    assert!(
        msg.contains("must use https://") && msg.contains("federation transport"),
        "expected TLS rejection message, got: {msg}"
    );
}

#[test]
fn https_scheme_accepted_into_transport_phase() {
    // Does not assert a successful fetch (no live peer); it asserts the guard
    // lets https:// past the scheme check (error is network-level, not TLS-policy).
    let fetcher = HttpRuntimeProofFetcher::new();
    let result = fetcher.fetch_peer_proof_bundle("https://peer.secure:9443");
    // Either network error (peer up) or ok (peer serves bundle). The guard
    // must NOT early-return a TLS-policy Err here.
    assert!(
        !result.map(|_| ()).map_err(|e| e.to_string()).is_err_and(|m| m.contains("must use https://")),
        "https:// must pass the TLS guard (network error expected instead)"
    );
}
