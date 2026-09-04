//! The 18 Canonical Ledger Transactions of a CynicPeer.
//!
//! Specifications: `protocols/cynic-stage2-peer-and-ledger.md` §5.
//! Invariants: Append-only, deterministic, content-addressed, idempotent.

use serde::{Deserialize, Serialize};

use super::accounts::{
    CitizenAccount, MandateAccount, NodeAccount, OfficeAccount, ResourceAccount,
};

pub const LEDGER_TRANSACTION_SCHEMA: &str = "cynic.peer.ledger_transaction.v1";

/// The 18 canonical constitutional transitions of the CynicPeer SMR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LedgerTransactionPayload {
    /// 1. Register a new human citizen or sovereign authority.
    CitizenRegistered { citizen: CitizenAccount },
    /// 2. Bind a physical/virtual machine node to a citizen custodian.
    NodeBound { node: NodeAccount },
    /// 3. Record physically observed hardware resources for a node.
    ResourceObserved { resource: ResourceAccount },
    /// 4. Define an institutional office with capabilities and bounds.
    OfficeDefined { office: OfficeAccount },
    /// 5. Issue a time-bounded, scoped mandate to an agent.
    MandateGranted { mandate: MandateAccount },
    /// 6. Formally admit a workload under an active mandate.
    WorkloadAdmitted {
        workload_id: String,
        mandate_id: String,
        target_node_id: String,
        action: String,
    },
    /// 7. Mark admitted workload execution started.
    WorkloadStarted {
        workload_id: String,
        started_at_unix_s: u64,
    },
    /// 8. Reserve compute/memory capacity for a workload.
    ResourceReserved {
        workload_id: String,
        node_id: String,
        cpu_cores: u32,
        ram_bytes: u64,
    },
    /// 9. Record actual physical resource consumption.
    ResourceConsumed {
        workload_id: String,
        node_id: String,
        actions_used: u64,
        wall_seconds_used: u64,
    },
    /// 10. Mark workload execution completed with receipt.
    WorkloadCompleted {
        workload_id: String,
        success: bool,
        completed_at_unix_s: u64,
        proof_digest: String,
    },
    /// 11. Record observed outcome metrics.
    OutcomeObserved {
        workload_id: String,
        outcome_kind: String,
        score_percent: u8,
    },
    /// 12. Attest cryptographic proof bundle reference.
    EvidenceAttested {
        evidence_id: String,
        digest: String,
        source: String,
    },
    /// 13. Record CYD calibration credit allocation.
    CalibrationCreditRecorded {
        treasury_id: String,
        amount: u64,
        reason: String,
    },
    /// 14. Open a formal dispute against a mandate or outcome.
    DisputeOpened {
        dispute_id: String,
        target_id: String,
        reason: String,
        opened_by_citizen_id: String,
    },
    /// 15. Temporarily suspend a credit or capability pending dispute.
    CreditSuspended {
        treasury_id: String,
        amount: u64,
        dispute_id: String,
    },
    /// 16. Revoke an active mandate prematurely.
    MandateRevoked {
        mandate_id: String,
        revocation_reason: String,
        revoked_at_unix_s: u64,
    },
    /// 17. Register a new federated CynicPeer in the mesh.
    PeerJoined {
        peer_id: String,
        public_key_hex: String,
        joined_at_unix_s: u64,
    },
    /// 18. Seal a deterministic checkpoint across all local accounts.
    PeerCheckpointed {
        epoch: u64,
        state_merkle_root_hex: String,
        checkpoint_unix_s: u64,
    },
}

/// A signed, versioned, append-only transaction envelope submitted to the Peer SMR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerTransaction {
    pub schema: String,
    pub tx_id: String,
    pub sequence: u64,
    pub timestamp_unix_s: u64,
    pub issuer_id: String,
    pub nonce: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_evidence_refs: Vec<String>,
    pub payload: LedgerTransactionPayload,
}

impl LedgerTransaction {
    pub fn new(
        tx_id: impl Into<String>,
        sequence: u64,
        timestamp_unix_s: u64,
        issuer_id: impl Into<String>,
        nonce: u64,
        payload: LedgerTransactionPayload,
    ) -> Self {
        Self {
            schema: LEDGER_TRANSACTION_SCHEMA.to_string(),
            tx_id: tx_id.into(),
            sequence,
            timestamp_unix_s,
            issuer_id: issuer_id.into(),
            nonce,
            signature_hex: None,
            input_evidence_refs: Vec::new(),
            payload,
        }
    }
}
