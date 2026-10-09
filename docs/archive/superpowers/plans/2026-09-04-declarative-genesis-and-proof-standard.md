# Declarative Genesis Bootstrap & Standard Double-Signed Proof Envelopes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform CynicPeer genesis bootstrap into a purely declarative registry-driven process and standardize observation evidence into a canonical double-signed proof envelope (`NodeObservationAttestation` + `MandateProofEnvelope<T>`).

**Architecture:**
1. Pure declarative genesis models in `cynic-protocol::peer::genesis` deriving deterministic transactions from `infra/registry.json`.
2. Standard double-signed proof envelopes in `cynic-protocol::runtime::proof_envelope` with pure Ed25519 verification.
3. Host signing port and canonical persistence in `cynic-host`.
4. Unified CLI surface (`--emit-proof`, `--proof-out`, `--mandate`) in `cynic-cli::network_audit`.

**Tech Stack:** Rust (edition 2024), Serde, ed25519-dalek 2.2, sha2 0.10, rusqlite 0.34, clap 4.6.

---

### Task 1: Declare `peer_genesis` Contract in `infra/registry.json`

**Files:**
- Modify: `infra/registry.json`

- [ ] **Step 1: Add the `peer_genesis` section to `infra/registry.json`**

Insert `"peer_genesis"` desired state specification into `infra/registry.json` declaring the founder citizen, standard institutional offices, and community treasury:

```json
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
        "allowed_capabilities": [
          "cargo_check",
          "cargo_build",
          "artifact_install"
        ],
        "forbidden_actions": [
          "unbounded_mutation"
        ]
      },
      {
        "office_id": "auditor",
        "title": "Architecture and Security Auditor",
        "allowed_capabilities": [
          "read_only_audit",
          "metrics_evaluation",
          "proof_verification"
        ],
        "forbidden_actions": [
          "unbounded_mutation"
        ]
      },
      {
        "office_id": "scheduler",
        "title": "Workload and Task Scheduler",
        "allowed_capabilities": [
          "workload_dispatch",
          "lease_coordination"
        ],
        "forbidden_actions": [
          "unbounded_mutation"
        ]
      },
      {
        "office_id": "coordinator",
        "title": "Mesh and Network Coordinator",
        "allowed_capabilities": [
          "preauthkey_generation",
          "node_revocation"
        ],
        "forbidden_actions": [
          "unbounded_mutation"
        ]
      }
    ],
    "initial_treasuries": [
      {
        "account_id": "treasury_community",
        "denom": "CYN",
        "initial_balance": 0
      }
    ]
  },
```

- [ ] **Step 2: Validate JSON syntax**

Run: `jq .peer_genesis infra/registry.json`
Expected: Outputs parsed `peer_genesis` object with 0 errors.

- [ ] **Step 3: Commit**

```bash
git add infra/registry.json
git commit -m "feat(registry): declare peer_genesis desired state contract"
```

---

### Task 2: Pure Declarative Genesis Models & Derivation (`cynic-protocol`)

**Files:**
- Create: `crates/cynic-protocol/src/peer/genesis.rs`
- Modify: `crates/cynic-protocol/src/peer/mod.rs`
- Test: unit tests in `crates/cynic-protocol/src/peer/genesis.rs`

- [ ] **Step 1: Write unit tests for declarative genesis derivation**

Create `crates/cynic-protocol/src/peer/genesis.rs` with tests:

```rust
//! Declarative genesis desired state contract and deterministic transaction derivation.

use serde::{Deserialize, Serialize};

use super::{
    CitizenAccount, LedgerTransaction, LedgerTransactionPayload, NodeAccount,
    NodeMembershipStatus, OfficeAccount, TreasuryAccount,
    CITIZEN_ACCOUNT_SCHEMA, NODE_ACCOUNT_SCHEMA, OFFICE_ACCOUNT_SCHEMA, TREASURY_ACCOUNT_SCHEMA,
};

pub const PEER_GENESIS_SCHEMA: &str = "cynic.peer_genesis.desired_state.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisFounder {
    pub citizen_id: String,
    pub name: String,
    pub public_key_ed25519: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisOffice {
    pub office_id: String,
    pub title: String,
    pub allowed_capabilities: Vec<String>,
    pub forbidden_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisTreasury {
    pub account_id: String,
    pub denom: String,
    pub initial_balance: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisNodeCandidate {
    pub node_id: String,
    pub pubkey_ed25519: String,
    pub role: String,
    pub overlay_ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerGenesisDesiredState {
    pub schema: String,
    pub founder: GenesisFounder,
    pub offices: Vec<GenesisOffice>,
    pub initial_treasuries: Vec<GenesisTreasury>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenesisDerivationError {
    InvalidSchema(String),
    MissingFounder,
    DuplicateOfficeId(String),
}

impl std::fmt::Display for GenesisDerivationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSchema(s) => write!(f, "invalid genesis schema: {s}"),
            Self::MissingFounder => write!(f, "genesis founder cannot be empty"),
            Self::DuplicateOfficeId(id) => write!(f, "duplicate office id in genesis: {id}"),
        }
    }
}

impl std::error::Error for GenesisDerivationError {}

/// Pure derivation of SMR genesis transactions from declared desired state and candidate nodes.
pub fn derive_genesis_transactions(
    genesis: &PeerGenesisDesiredState,
    nodes: &[GenesisNodeCandidate],
    genesis_unix_s: u64,
) -> Result<Vec<LedgerTransaction>, GenesisDerivationError> {
    if genesis.schema != PEER_GENESIS_SCHEMA {
        return Err(GenesisDerivationError::InvalidSchema(genesis.schema.clone()));
    }
    if genesis.founder.citizen_id.trim().is_empty() {
        return Err(GenesisDerivationError::MissingFounder);
    }

    let mut txs = Vec::new();
    let mut seq = 1u64;
    let founder_id = &genesis.founder.citizen_id;

    // 1. Founder Citizen
    let admin = CitizenAccount::new_admin(
        founder_id,
        &genesis.founder.name,
        &genesis.founder.public_key_ed25519,
        genesis_unix_s,
    );
    txs.push(LedgerTransaction::new(
        "genesis_tx_01_founder",
        seq,
        genesis_unix_s,
        founder_id,
        0,
        LedgerTransactionPayload::CitizenRegistered { citizen: admin },
    ));

    // 2. Institutional Offices
    let mut seen_offices = std::collections::BTreeSet::new();
    for off in &genesis.offices {
        if !seen_offices.insert(&off.office_id) {
            return Err(GenesisDerivationError::DuplicateOfficeId(off.office_id.clone()));
        }
        seq += 1;
        let office = OfficeAccount {
            schema: OFFICE_ACCOUNT_SCHEMA.to_string(),
            office_id: off.office_id.clone(),
            title: off.title.clone(),
            allowed_capabilities: off.allowed_capabilities.clone(),
            forbidden_actions: off.forbidden_actions.clone(),
        };
        txs.push(LedgerTransaction::new(
            format!("genesis_office_{}", off.office_id),
            seq,
            genesis_unix_s,
            founder_id,
            seq - 1,
            LedgerTransactionPayload::OfficeDefined { office },
        ));
    }

    // 3. Registered Nodes
    for n in nodes {
        seq += 1;
        let node_acct = NodeAccount {
            schema: NODE_ACCOUNT_SCHEMA.to_string(),
            node_id: n.node_id.clone(),
            pubkey_ed25519: n.pubkey_ed25519.clone(),
            role: n.role.clone(),
            overlay_ip: n.overlay_ip.clone(),
            registered_at_unix_s: genesis_unix_s,
            status: NodeMembershipStatus::Active,
        };
        txs.push(LedgerTransaction::new(
            format!("genesis_node_{}", n.node_id),
            seq,
            genesis_unix_s,
            founder_id,
            seq - 1,
            LedgerTransactionPayload::NodeRegistered { node: node_acct },
        ));
    }

    // 4. Initial Treasuries
    for tr in &genesis.initial_treasuries {
        seq += 1;
        let treasury = TreasuryAccount {
            schema: TREASURY_ACCOUNT_SCHEMA.to_string(),
            account_id: tr.account_id.clone(),
            denom: tr.denom.clone(),
            balance: tr.initial_balance,
            last_allocated_at_unix_s: genesis_unix_s,
        };
        txs.push(LedgerTransaction::new(
            format!("genesis_treasury_{}", tr.account_id),
            seq,
            genesis_unix_s,
            founder_id,
            seq - 1,
            LedgerTransactionPayload::TreasuryMinted { treasury },
        ));
    }

    Ok(txs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_deterministic_transactions_from_desired_state() {
        let genesis = PeerGenesisDesiredState {
            schema: PEER_GENESIS_SCHEMA.to_string(),
            founder: GenesisFounder {
                citizen_id: "cit_admin".to_string(),
                name: "Sovereign Operator".to_string(),
                public_key_ed25519: "aabbcc".to_string(),
            },
            offices: vec![GenesisOffice {
                office_id: "auditor".to_string(),
                title: "Auditor".to_string(),
                allowed_capabilities: vec!["audit".to_string()],
                forbidden_actions: vec![],
            }],
            initial_treasuries: vec![GenesisTreasury {
                account_id: "treasury_cyn".to_string(),
                denom: "CYN".to_string(),
                initial_balance: 1000,
            }],
        };

        let nodes = vec![GenesisNodeCandidate {
            node_id: "cynic-builder".to_string(),
            pubkey_ed25519: "112233".to_string(),
            role: "build_runner".to_string(),
            overlay_ip: Some("10.10.10.119".to_string()),
        }];

        let txs = derive_genesis_transactions(&genesis, &nodes, 1724000000)
            .expect("derivation must succeed");
        assert_eq!(txs.len(), 4);
        assert_eq!(txs[0].sequence, 1);
        assert_eq!(txs[1].sequence, 2);
        assert_eq!(txs[2].sequence, 3);
        assert_eq!(txs[3].sequence, 4);
    }

    #[test]
    fn rejects_invalid_genesis_schema() {
        let genesis = PeerGenesisDesiredState {
            schema: "cynic.invalid.v1".to_string(),
            founder: GenesisFounder {
                citizen_id: "cit_admin".to_string(),
                name: "Sovereign Operator".to_string(),
                public_key_ed25519: "aabbcc".to_string(),
            },
            offices: vec![],
            initial_treasuries: vec![],
        };
        let err = derive_genesis_transactions(&genesis, &[], 1724000000).unwrap_err();
        assert!(matches!(err, GenesisDerivationError::InvalidSchema(_)));
    }
}
```

- [ ] **Step 2: Wire `pub mod genesis;` into `crates/cynic-protocol/src/peer/mod.rs`**

Add `pub mod genesis;` and `pub use genesis::*;` to `crates/cynic-protocol/src/peer/mod.rs`.

- [ ] **Step 3: Run unit tests on builder**

Sync and run: `ssh root@10.10.10.119 'cd /var/lib/cynic/source/repositories/CYNIC && sudo -u cynic cargo test -p cynic-protocol --lib peer::genesis'`
Expected: `test result: ok. 2 passed; 0 failed`.

- [ ] **Step 4: Commit**

```bash
git add crates/cynic-protocol/src/peer/genesis.rs crates/cynic-protocol/src/peer/mod.rs
git commit -m "feat(protocol): pure declarative genesis models and transaction derivation"
```

---

### Task 3: Refactor Node Genesis Bootstrap to Consume Declarative Registry (`cynic-node`)

**Files:**
- Modify: `crates/cynic-node/src/adapters/peer_storage/genesis.rs`
- Test: integration test in `crates/cynic-node/src/adapters/peer_storage/mod.rs`

- [ ] **Step 1: Refactor `bootstrap_genesis_from_registry`**

Update `crates/cynic-node/src/adapters/peer_storage/genesis.rs`:
1. Parse `"peer_genesis"` from `registry.json` as `PeerGenesisDesiredState`.
2. Extract nodes as `Vec<GenesisNodeCandidate>`.
3. Call pure `derive_genesis_transactions(...)`.
4. Commit each transaction to `storage`.
5. Remove all hardcoded citizen and office string literals.

```rust
pub fn bootstrap_genesis_from_registry(
    storage: &mut PeerStorageManager,
    registry_path: impl AsRef<Path>,
) -> Result<usize, PeerStorageError> {
    if storage.state().last_sequence > 0 {
        return Ok(0);
    }

    let registry_path = registry_path.as_ref();
    let raw = fs::read_to_string(registry_path)
        .map_err(|e| PeerStorageError::Wal(format!("failed to read registry for genesis: {e}")))?;
    let val: Value = serde_json::from_str(&raw)
        .map_err(|e| PeerStorageError::Wal(format!("failed to parse registry json: {e}")))?;

    let genesis_raw = val.get("peer_genesis")
        .ok_or_else(|| PeerStorageError::Wal("missing 'peer_genesis' in registry.json".to_string()))?;
    let genesis: cynic_protocol::peer::PeerGenesisDesiredState = serde_json::from_value(genesis_raw.clone())
        .map_err(|e| PeerStorageError::Wal(format!("invalid 'peer_genesis' contract: {e}")))?;

    let mut candidates = Vec::new();
    if let Some(nodes) = val.get("nodes").and_then(|n| n.as_array()) {
        for node in nodes {
            let Some(node_id) = node.get("id").or_else(|| node.get("name")).and_then(|v| v.as_str()) else { continue; };
            let overlay_ip = node.get("network").and_then(|n| n.get("synapse")).and_then(|v| v.as_str()).map(String::from);
            let role = node.get("role").and_then(|v| v.as_str()).unwrap_or("worker").to_string();
            let pubkey = format!("ed25519_node_{node_id}");
            candidates.push(cynic_protocol::peer::GenesisNodeCandidate {
                node_id: node_id.to_string(),
                pubkey_ed25519: pubkey,
                role,
                overlay_ip,
            });
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1724000000);

    let txs = cynic_protocol::peer::derive_genesis_transactions(&genesis, &candidates, now)
        .map_err(|e| PeerStorageError::Wal(e.to_string()))?;

    let tx_count = txs.len();
    for tx in txs {
        storage.commit(tx)?;
    }

    Ok(tx_count)
}
```

- [ ] **Step 2: Add integration test testing pure registry-driven bootstrap**

In `crates/cynic-node/src/adapters/peer_storage/genesis.rs`:
Add test verifying that bootstrap succeeds with valid declarative JSON and fails closed if `"peer_genesis"` is missing.

- [ ] **Step 3: Run node peer tests on builder**

Run: `ssh root@10.10.10.119 'cd /var/lib/cynic/source/repositories/CYNIC && sudo -u cynic cargo test -p cynic-node --lib peer_storage'`
Expected: All peer storage tests PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/cynic-node/src/adapters/peer_storage/genesis.rs
git commit -m "feat(node): make peer genesis bootstrap 100% declarative from registry"
```

---

### Task 4: Standard Proof Envelopes (`cynic-protocol::runtime::proof_envelope`)

**Files:**
- Create: `crates/cynic-protocol/src/runtime/proof_envelope.rs`
- Modify: `crates/cynic-protocol/src/runtime.rs`
- Test: unit tests in `crates/cynic-protocol/src/runtime/proof_envelope.rs`

- [ ] **Step 1: Implement `NodeObservationAttestation` and `MandateProofEnvelope<T>`**

Create `crates/cynic-protocol/src/runtime/proof_envelope.rs`:
- Structures:
  * `NodeObservationAttestation` (`schema`, `node_id`, `subject_digest_sha256`, `observed_at_unix_ms`, `signature_hex`)
  * `MandateProofEnvelope<T>` (`schema`, `mandate_id`, `previous_proof_hash`, `node_attestation`, `payload: T`, `auditor_signature_hex`)
- Crypto helpers using `ed25519_dalek` and `sha2`:
  * `sign_observation_digest(private_key_bytes: &[u8; 32], digest_sha256: &str) -> String`
  * `verify_observation_attestation(attestation: &NodeObservationAttestation, public_key_bytes: &[u8; 32]) -> Result<(), String>`
  * `sign_mandate_envelope<T: Serialize>(private_key_bytes: &[u8; 32], mandate_id: &str, prev_hash: Option<&str>, attestation: &NodeObservationAttestation, payload: &T) -> String`
  * `verify_mandate_envelope<T: Serialize>(envelope: &MandateProofEnvelope<T>, auditor_public_key_bytes: &[u8; 32]) -> Result<(), String>`

- [ ] **Step 2: Add TDD unit tests for signature roundtrip and tampering detection**

Test:
1. Valid key signs observation $\to$ attestation verifies.
2. Tampered observation digest $\to$ verification fails.
3. Valid auditor key signs envelope $\to$ mandate proof verifies.
4. Tampered previous hash $\to$ verification fails.

- [ ] **Step 3: Run unit tests on builder**

Run: `ssh root@10.10.10.119 'cd /var/lib/cynic/source/repositories/CYNIC && sudo -u cynic cargo test -p cynic-protocol --lib runtime::proof_envelope'`
Expected: `test result: ok. 4 passed; 0 failed`.

- [ ] **Step 4: Commit**

```bash
git add crates/cynic-protocol/src/runtime/proof_envelope.rs crates/cynic-protocol/src/runtime.rs
git commit -m "feat(protocol): canonical double-signed proof envelopes with Ed25519 verification"
```

---

### Task 5: Proof Signing Port & Storage Persistence (`cynic-host`)

**Files:**
- Create: `crates/cynic-host/src/proof_signer.rs`
- Modify: `crates/cynic-host/src/lib.rs`
- Test: unit tests in `crates/cynic-host/src/proof_signer.rs`

- [ ] **Step 1: Implement `ProofSignerPort` and `LocalKeyProofSigner`**

In `crates/cynic-host/src/proof_signer.rs`:
- Port trait:
  ```rust
  pub trait ProofSignerPort: Send + Sync {
      fn sign_node_observation(&self, digest_sha256: &str) -> Result<String, String>;
      fn sign_mandate_envelope(&self, digest_sha256: &str) -> Result<String, String>;
  }
  ```
- Local adapter resolving keys from environment or file:
  * Node key: `CYNIC_NODE_SIGNING_KEY` / `/etc/cynic/identity/node.key` (or ephemeral fallback in lab/test).
  * Auditor key: `CYNIC_AUDIT_SIGNING_KEY` / active mandate session key.
- Helper `persist_proof_envelope<T: Serialize>(envelope: &MandateProofEnvelope<T>, output_path: Option<&Path>) -> Result<PathBuf, String>` saving pretty JSON atomically.

- [ ] **Step 2: Run tests on builder**

Run: `ssh root@10.10.10.119 'cd /var/lib/cynic/source/repositories/CYNIC && sudo -u cynic cargo test -p cynic-host --lib proof_signer'`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/cynic-host/src/proof_signer.rs crates/cynic-host/src/lib.rs
git commit -m "feat(host): implement ProofSignerPort and canonical proof persistence"
```

---

### Task 6: CLI Proof Standard in `network-audit` (`cynic-cli`)

**Files:**
- Modify: `crates/cynic-cli/src/network_audit/mod.rs`
- Modify: `crates/cynic-cli/src/network_audit/scan.rs`
- Modify: `crates/cynic-cli/src/lib.rs`

- [ ] **Step 1: Add `--emit-proof`, `--proof-out`, and `--mandate` to `NetworkAuditCommand`**

In `crates/cynic-cli/src/lib.rs` and `network_audit/mod.rs`:
```rust
    #[command(name = "network-audit")]
    NetworkAudit {
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        emit_proof: bool,
        #[arg(long)]
        proof_out: Option<PathBuf>,
        #[arg(long)]
        mandate: Option<String>,
    },
```

- [ ] **Step 2: Wire execution in `execute_network_audit`**

When `emit_proof == true`:
1. Execute scan producing `NetworkAuditReport`.
2. Compute SHA-256 of canonical report JSON.
3. Sign node attestation via `ProofSignerPort`.
4. Wrap in `MandateProofEnvelope`.
5. Persist to standard path or `--proof-out`.
6. Output JSON or human confirmation.

- [ ] **Step 3: Test execution end-to-end**

Run on builder:
`ssh root@10.10.10.119 'cd /var/lib/cynic/source/repositories/CYNIC && sudo -u cynic cargo check --locked -p cynic-cli'`
Expected: Compilation clean, exit 0.

- [ ] **Step 4: Commit**

```bash
git add crates/cynic-cli/src/network_audit/ crates/cynic-cli/src/lib.rs
git commit -m "feat(cli): expose standard --emit-proof, --proof-out, --mandate on network-audit"
```

---

### Task 7: Full Fleet Verification on Builder Node (`10.10.10.119`)

**Files:**
- All touched crates

- [ ] **Step 1: Fetch and checkout latest branch commit on builder**
- [ ] **Step 2: Run all peer and proof tests**
  `sudo -u cynic cargo test -p cynic-protocol --lib peer`
  `sudo -u cynic cargo test -p cynic-protocol --lib runtime::proof_envelope`
  `sudo -u cynic cargo test -p cynic-node --lib peer_storage`
  `sudo -u cynic cargo test -p cynic-host --lib proof_signer`
  `sudo -u cynic cargo test -p cynic-cli --lib`
- [ ] **Step 3: Run `--emit-proof` smoke test on live registry**
  Execute `cynic network-audit --emit-proof --json` and verify emitted `MandateProofEnvelope` structure and valid signatures.
