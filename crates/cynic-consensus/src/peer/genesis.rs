//! Declarative genesis desired state contract and deterministic transaction derivation.

use serde::{Deserialize, Serialize};

use super::{
    CitizenAccount, LedgerTransaction, LedgerTransactionPayload, NodeAccount, NodeMembershipStatus,
    OFFICE_ACCOUNT_SCHEMA, OfficeAccount,
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
    pub treasury_id: String,
    pub initial_credits: u64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisNodeCandidate {
    pub node_id: String,
    pub hostname: String,
    pub host_family: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerGenesisDesiredState {
    pub schema: String,
    pub founder: GenesisFounder,
    pub offices: Vec<GenesisOffice>,
    #[serde(default)]
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
        return Err(GenesisDerivationError::InvalidSchema(
            genesis.schema.clone(),
        ));
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
            return Err(GenesisDerivationError::DuplicateOfficeId(
                off.office_id.clone(),
            ));
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
            schema: super::NODE_ACCOUNT_SCHEMA.to_string(),
            node_id: n.node_id.clone(),
            owner_citizen_id: founder_id.clone(),
            hostname: n.hostname.clone(),
            host_family: n.host_family.clone(),
            capabilities: n.capabilities.clone(),
            status: NodeMembershipStatus::Enrolled,
            last_liveness_proof_unix_s: Some(genesis_unix_s),
        };
        txs.push(LedgerTransaction::new(
            format!("genesis_node_{}", n.node_id),
            seq,
            genesis_unix_s,
            founder_id,
            seq - 1,
            LedgerTransactionPayload::NodeBound { node: node_acct },
        ));
    }

    // 4. Initial Treasury Allocations
    for tr in &genesis.initial_treasuries {
        seq += 1;
        txs.push(LedgerTransaction::new(
            format!("genesis_treasury_{}", tr.treasury_id),
            seq,
            genesis_unix_s,
            founder_id,
            seq - 1,
            LedgerTransactionPayload::CalibrationCreditRecorded {
                treasury_id: tr.treasury_id.clone(),
                amount: tr.initial_credits,
                reason: tr.reason.clone(),
            },
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
                treasury_id: "treasury_cyn".to_string(),
                initial_credits: 1000,
                reason: "initial genesis endowment".to_string(),
            }],
        };

        let nodes = vec![GenesisNodeCandidate {
            node_id: "cynic-builder".to_string(),
            hostname: "cynic-builder.synapse.cynic".to_string(),
            host_family: "linux.ubuntu.systemd".to_string(),
            capabilities: vec!["build_runner".to_string()],
        }];

        let txs = derive_genesis_transactions(&genesis, &nodes, 1724000000)
            .expect("derivation must succeed");
        assert_eq!(txs.len(), 4);
        assert_eq!(txs[0].sequence, 1);
        assert_eq!(txs[1].sequence, 2);
        assert_eq!(txs[2].sequence, 3);
        assert_eq!(txs[3].sequence, 4);
        assert_eq!(txs[0].issuer_id, "cit_admin");
        assert_eq!(txs[1].issuer_id, "cit_admin");
        assert_eq!(txs[2].issuer_id, "cit_admin");
        assert_eq!(txs[3].issuer_id, "cit_admin");
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

    #[test]
    fn rejects_duplicate_office_id() {
        let genesis = PeerGenesisDesiredState {
            schema: PEER_GENESIS_SCHEMA.to_string(),
            founder: GenesisFounder {
                citizen_id: "cit_admin".to_string(),
                name: "Sovereign Operator".to_string(),
                public_key_ed25519: "aabbcc".to_string(),
            },
            offices: vec![
                GenesisOffice {
                    office_id: "auditor".to_string(),
                    title: "Auditor 1".to_string(),
                    allowed_capabilities: vec![],
                    forbidden_actions: vec![],
                },
                GenesisOffice {
                    office_id: "auditor".to_string(),
                    title: "Auditor 2".to_string(),
                    allowed_capabilities: vec![],
                    forbidden_actions: vec![],
                },
            ],
            initial_treasuries: vec![],
        };
        let err = derive_genesis_transactions(&genesis, &[], 1724000000).unwrap_err();
        assert!(matches!(err, GenesisDerivationError::DuplicateOfficeId(_)));
    }
}
