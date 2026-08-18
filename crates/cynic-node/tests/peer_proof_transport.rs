//! 2N+2b TDD — standalone (no cynic-node full compile) — verify the port-bound
//! federation policy `verify_peer_runtime_publisher_admission` routes the peer
//! RuntimeProofBundle's `runtime_publisher_admission` gate WITHOUT depending on
//! cynic-inference transitive debt. Tests are pure: mock fetcher injects bundles.

use cynic_node::network_runtime_proof::{
    fetch_peer_runtime_proof_bundle, verify_peer_runtime_publisher_admission,
};
use cynic_protocol::{
    RUNTIME_PROOF_BUNDLE_SCHEMA, RUNTIME_PROOF_SCHEMA, RuntimeProof, RuntimeProofBundle,
    RuntimeProofEvidence,
};

/// Mock fetcher — injects a canned bundle (AGENS.md: adapters disposable).
struct MockFetcher {
    bundle: RuntimeProofBundle,
}

impl cynic_node::ports::runtime_proof_fetcher_port::RuntimeProofFetcherPort for MockFetcher {
    fn fetch_peer_proof_bundle(
        &self,
        _: &str,
    ) -> Result<RuntimeProofBundle, Box<dyn std::error::Error>> {
        Ok(self.bundle.clone())
    }
}

fn granted_bundle() -> RuntimeProofBundle {
    // Mirrors infra/runtime_proof.v1 — granted runtime_publisher (2N+1).
    RuntimeProofBundle {
        schema: RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
        status: "admitted".to_string(),
        proofs: vec![RuntimeProof {
            schema: RUNTIME_PROOF_SCHEMA.to_string(),
            gate: "runtime_publisher_admission".to_string(),
            status: "admitted".to_string(),
            producer: "cynic-node".to_string(),
            subject: "RuntimeMachineIdentity".to_string(),
            observed_at_unix_ms: 1,
            evidence: vec![RuntimeProofEvidence {
                id: "runtime_publisher_granted".to_string(),
                status: "true".to_string(),
                detail: "granted=true via=federated".to_string(),
            }],
        }],
    }
}

fn denied_bundle() -> RuntimeProofBundle {
    RuntimeProofBundle {
        schema: RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
        status: "blocked".to_string(),
        proofs: vec![RuntimeProof {
            schema: RUNTIME_PROOF_SCHEMA.to_string(),
            gate: "runtime_publisher_admission".to_string(),
            status: "denied_by_2N_seed".to_string(),
            producer: "cynic-node".to_string(),
            subject: "RuntimeMachineIdentity".to_string(),
            observed_at_unix_ms: 1,
            evidence: vec![RuntimeProofEvidence {
                id: "runtime_publisher_granted".to_string(),
                status: "false".to_string(),
                detail: "granted=false".to_string(),
            }],
        }],
    }
}

fn empty_bundle() -> RuntimeProofBundle {
    RuntimeProofBundle {
        schema: RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
        status: "unknown".to_string(),
        proofs: vec![RuntimeProof {
            schema: RUNTIME_PROOF_SCHEMA.to_string(),
            gate: "network_runtime_observed".to_string(),
            status: "satisfied".to_string(),
            producer: "cynic-node".to_string(),
            subject: "peer".to_string(),
            observed_at_unix_ms: 1,
            evidence: vec![RuntimeProofEvidence {
                id: "network_status".to_string(),
                status: "mesh_active".to_string(),
                detail: "no runtime_publisher proof".to_string(),
            }],
        }],
    }
}

#[test]
fn granted_peer_admitted_to_federation() {
    let fetcher = MockFetcher {
        bundle: granted_bundle(),
    };
    assert!(verify_peer_runtime_publisher_admission(
        &fetcher,
        "http://peer:3000"
    ));
}

#[test]
fn denied_peer_blocked_from_federation() {
    let fetcher = MockFetcher {
        bundle: denied_bundle(),
    };
    assert!(!verify_peer_runtime_publisher_admission(
        &fetcher,
        "http://peer:3000"
    ));
}

#[test]
fn peer_without_runtime_publisher_proof_blocked() {
    // § "Unknown is data" — missing gate = residual unknown, never silent allow.
    let fetcher = MockFetcher {
        bundle: empty_bundle(),
    };
    assert!(!verify_peer_runtime_publisher_admission(
        &fetcher,
        "http://peer:3000"
    ));
}

#[test]
fn fetcher_port_routes_through_injected_adapter() {
    let fetcher = MockFetcher {
        bundle: granted_bundle(),
    };
    let bundle = fetch_peer_runtime_proof_bundle(&fetcher, "http://peer:3000");
    let bundle = bundle.expect("bundle fetched through injected port");
    assert_eq!(bundle.proofs.len(), 1);
    assert_eq!(bundle.proofs[0].gate, "runtime_publisher_admission");
}
