# Design: Declarative Genesis Bootstrap & Standard Double-Signed Proof Envelopes

**Date**: 2026-09-04  
**Status**: Approved (Brainstorming)  
**Context**: Branch `agent/gemini/constitutional-peer-foundation`  

---

## 1. Context & Motivation

1. **Declarative Genesis**: `bootstrap_genesis_from_registry` currently hardcodes the founder citizen (`cit_admin`) and the 4 institutional offices (`builder`, `auditor`, `scheduler`, `coordinator`) inside Rust source code (`genesis.rs`). This violates CYNIC's invariant that desired state resides in declarative contracts/registries, with code acting strictly as a pure deterministic executor.
2. **Proof Standardization & Double Signature**: Previous ad-hoc proposals introduced non-standard French flags (`--produit-preuve`) and isolated schemas (`cynic.network_audit_proof.v1`), deepening architectural fragmentation. A universal standard is required where any observation (network, storage, build) can be sealed as a double-signed evidence artifact:
   - **Layer 1 (Node Attestation)**: The physical probing machine signs the observation digest using its machine key (`/etc/cynic/identity/node.key` or Ed25519 environment).
   - **Layer 2 (Mandate Proof Envelope)**: The auditing authority signs under an active `MandateAccount`, anchoring the proof into the SMR causal dependency chain and preserving verification continuity (`previous_proof_hash`).

---

## 2. Architecture & Components

### A. Declarative Genesis Contract (`infra/registry.json`)

Add the `"peer_genesis"` section to `infra/registry.json`:

```jsonc
"peer_genesis": {
  "schema": "cynic.peer_genesis.desired_state.v1",
  "founder": {
    "citizen_id": "cit_admin",
    "name": "Sovereign Operator",
    "public_key_ed25519": "0000000000000000000000000000000000000000000000000000000000000000"
  },
  "offices": [
    {
      "office_id": "builder",
      "title": "Build Runner",
      "allowed_capabilities": ["cargo_check", "cargo_build", "artifact_install"],
      "forbidden_actions": ["unbounded_mutation"]
    },
    {
      "office_id": "auditor",
      "title": "Architecture and Security Auditor",
      "allowed_capabilities": ["read_only_audit", "metrics_evaluation", "proof_verification"],
      "forbidden_actions": ["unbounded_mutation"]
    },
    {
      "office_id": "scheduler",
      "title": "Workload and Task Scheduler",
      "allowed_capabilities": ["workload_dispatch", "lease_coordination"],
      "forbidden_actions": ["unbounded_mutation"]
    },
    {
      "office_id": "coordinator",
      "title": "Mesh and Network Coordinator",
      "allowed_capabilities": ["preauthkey_generation", "node_revocation"],
      "forbidden_actions": ["unbounded_mutation"]
    }
  ],
  "initial_treasuries": [
    {
      "account_id": "treasury_community",
      "denom": "CYN",
      "initial_balance": 0
    }
  ]
}
```

### B. Pure Protocol Models (`crates/cynic-protocol/src/peer/genesis.rs`)

1. Types:
   - `PeerGenesisDesiredState`
   - `GenesisFounder`
   - `GenesisOffice`
   - `GenesisTreasury`
   - `GenesisNodeCandidate`
2. Pure function:
   ```rust
   pub fn derive_genesis_transactions(
       genesis: &PeerGenesisDesiredState,
       nodes: &[GenesisNodeCandidate],
       genesis_unix_s: u64,
   ) -> Result<Vec<LedgerTransaction>, GenesisDerivationError>
   ```
   - Emits Tx 1: `CitizenRegistered` for founder citizen.
   - Emits Tx 2..N: `OfficeDefined` for all declared offices.
   - Emits Tx N+1..M: `NodeRegistered` for declared candidate nodes.
   - Emits Tx M+1..K: `TreasuryMinted` for initial treasuries.
   - Fails closed if the `peer_genesis` block is absent or structurally invalid.

### C. Standard Proof Envelopes (`crates/cynic-protocol/src/runtime/proof_envelope.rs`)

1. Schemas:
   - `cynic.node.observation_attestation.v1`
   - `cynic.runtime.mandate_proof_envelope.v1`
2. Types:
   - `NodeObservationAttestation`:
     * `schema: String`
     * `node_id: String`
     * `subject_digest_sha256: String`
     * `observed_at_unix_ms: u64`
     * `signature_hex: String`
   - `MandateProofEnvelope<T>`:
     * `schema: String`
     * `mandate_id: String`
     * `previous_proof_hash: Option<String>`
     * `node_attestation: NodeObservationAttestation`
     * `payload: T` (e.g. `NetworkAuditReport`)
     * `auditor_signature_hex: String`
3. Pure verification utilities:
   - `hash_canonical_json<T: Serialize>(val: &T) -> String`
   - `verify_node_attestation(attestation: &NodeObservationAttestation, public_key_ed25519: &str) -> bool`
   - `verify_mandate_proof<T: Serialize>(envelope: &MandateProofEnvelope<T>, auditor_public_key: &str) -> bool`

### D. CLI Surface & Persistence

1. CLI parameters:
   - Standard flags: `--emit-proof`, `--proof-out <path>`, `--mandate <id>`
   - Replaces non-standard `--produit-preuve`.
2. Default storage path:
   - `/var/lib/cynic/proofs/<gate>/<timestamp>.json` (root) or `$XDG_DATA_HOME/cynic/proofs/<gate>/<timestamp>.json` (user).

---

## 3. Invariants & Error Handling

- **Zero Hardcoding**: Founder citizen, offices, capabilities, and node initializations are strictly sourced from declarative registries.
- **Fail-Closed Verification**: An invalid or absent signature results in `VerificationError` and immediate command failure.
- **Library-First Invariant**: All domain logic and serialization routines reside in `cynic-protocol` and `cynic-node` libraries; `cynic-cli` remains a thin projection facade.
