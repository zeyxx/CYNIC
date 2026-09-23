//! Pure deterministic state transition engine for CynicPeer.
//!
//! Implements:
//!   f(PeerState, LedgerTransaction) -> Result<(PeerState, TransactionReceipt), TransitionError>
//!
//! Zero I/O, pure in-memory, deterministic, content-addressed.

use serde::{Deserialize, Serialize};

use super::accounts::MandateStatus;
use super::state::{AdmittedWorkload, PeerState, WorkloadExecutionPhase};
use super::transactions::{LedgerTransaction, LedgerTransactionPayload};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionError {
    SequenceNotMonotonic {
        expected: u64,
        got: u64,
    },
    CitizenAlreadyExists(String),
    CitizenNotFound(String),
    NodeAlreadyExists(String),
    NodeNotFound(String),
    AgentAlreadyExists(String),
    AgentNotFound(String),
    OfficeAlreadyExists(String),
    OfficeNotFound(String),
    MandateAlreadyExists(String),
    MandateNotFound(String),
    MandateInvalid {
        mandate_id: String,
        reason: String,
    },
    FencingTokenRegression {
        agent_id: String,
        expected_gt: u64,
        got: u64,
    },
    ResourceNotFound(String),
    ResourceQuotaExceeded {
        node_id: String,
        requested: u64,
        available: u64,
    },
    WorkloadAlreadyExists(String),
    WorkloadNotFound(String),
    WorkloadPhaseMismatch {
        workload_id: String,
        expected: WorkloadExecutionPhase,
        got: WorkloadExecutionPhase,
    },
    DisputeAlreadyExists(String),
    TreasuryNotFound(String),
    InsufficientCredits {
        treasury_id: String,
        requested: u64,
        available: u64,
    },
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SequenceNotMonotonic { expected, got } => {
                write!(f, "sequence not monotonic: expected {expected}, got {got}")
            }
            Self::CitizenAlreadyExists(id) => write!(f, "citizen already exists: {id}"),
            Self::CitizenNotFound(id) => write!(f, "citizen not found: {id}"),
            Self::NodeAlreadyExists(id) => write!(f, "node already exists: {id}"),
            Self::NodeNotFound(id) => write!(f, "node not found: {id}"),
            Self::AgentAlreadyExists(id) => write!(f, "agent already exists: {id}"),
            Self::AgentNotFound(id) => write!(f, "agent not found: {id}"),
            Self::OfficeAlreadyExists(id) => write!(f, "office already exists: {id}"),
            Self::OfficeNotFound(id) => write!(f, "office not found: {id}"),
            Self::MandateAlreadyExists(id) => write!(f, "mandate already exists: {id}"),
            Self::MandateNotFound(id) => write!(f, "mandate not found: {id}"),
            Self::MandateInvalid { mandate_id, reason } => {
                write!(f, "mandate {mandate_id} invalid: {reason}")
            }
            Self::FencingTokenRegression {
                agent_id,
                expected_gt,
                got,
            } => {
                write!(
                    f,
                    "fencing token regression for agent {agent_id}: expected > {expected_gt}, got {got}"
                )
            }
            Self::ResourceNotFound(id) => write!(f, "resource not found: {id}"),
            Self::ResourceQuotaExceeded {
                node_id,
                requested,
                available,
            } => {
                write!(
                    f,
                    "resource quota exceeded on node {node_id}: requested {requested}, available {available}"
                )
            }
            Self::WorkloadAlreadyExists(id) => write!(f, "workload already exists: {id}"),
            Self::WorkloadNotFound(id) => write!(f, "workload not found: {id}"),
            Self::WorkloadPhaseMismatch {
                workload_id,
                expected,
                got,
            } => {
                write!(
                    f,
                    "workload {workload_id} phase mismatch: expected {expected:?}, got {got:?}"
                )
            }
            Self::DisputeAlreadyExists(id) => write!(f, "dispute already exists: {id}"),
            Self::TreasuryNotFound(id) => write!(f, "treasury not found: {id}"),
            Self::InsufficientCredits {
                treasury_id,
                requested,
                available,
            } => {
                write!(
                    f,
                    "insufficient credits in treasury {treasury_id}: requested {requested}, available {available}"
                )
            }
        }
    }
}

impl std::error::Error for TransitionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionOutcome {
    Applied,
    Checkpointed { epoch: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub tx_id: String,
    pub sequence: u64,
    pub timestamp_unix_s: u64,
    pub outcome: TransitionOutcome,
    pub state_root_digest: String,
}

/// Apply a signed transaction to the PeerState, producing a new state and receipt.
pub fn apply_transaction(
    state: &PeerState,
    tx: &LedgerTransaction,
) -> Result<(PeerState, TransactionReceipt), TransitionError> {
    // 1. Invariant: Monotonic sequence
    if tx.sequence != state.last_sequence + 1 {
        return Err(TransitionError::SequenceNotMonotonic {
            expected: state.last_sequence + 1,
            got: tx.sequence,
        });
    }

    let mut next = state.clone();
    let mut outcome = TransitionOutcome::Applied;

    // 2. State transition by transaction payload
    match &tx.payload {
        LedgerTransactionPayload::CitizenRegistered { citizen } => {
            if next.citizens.contains_key(&citizen.citizen_id) {
                return Err(TransitionError::CitizenAlreadyExists(
                    citizen.citizen_id.clone(),
                ));
            }
            next.citizens
                .insert(citizen.citizen_id.clone(), citizen.clone());
        }
        LedgerTransactionPayload::NodeBound { node } => {
            if !next.citizens.contains_key(&node.owner_citizen_id) {
                return Err(TransitionError::CitizenNotFound(
                    node.owner_citizen_id.clone(),
                ));
            }
            if next.nodes.contains_key(&node.node_id) {
                return Err(TransitionError::NodeAlreadyExists(node.node_id.clone()));
            }
            next.nodes.insert(node.node_id.clone(), node.clone());
        }
        LedgerTransactionPayload::ResourceObserved { resource } => {
            if !next.nodes.contains_key(&resource.node_id) {
                return Err(TransitionError::NodeNotFound(resource.node_id.clone()));
            }
            next.resources
                .insert(resource.resource_id.clone(), resource.clone());
        }
        LedgerTransactionPayload::OfficeDefined { office } => {
            if next.offices.contains_key(&office.office_id) {
                return Err(TransitionError::OfficeAlreadyExists(
                    office.office_id.clone(),
                ));
            }
            next.offices
                .insert(office.office_id.clone(), office.clone());
        }
        LedgerTransactionPayload::MandateGranted { mandate } => {
            // Mandate requires valid office
            if !next.offices.contains_key(&mandate.office_id) {
                return Err(TransitionError::OfficeNotFound(mandate.office_id.clone()));
            }
            if next.mandates.contains_key(&mandate.mandate_id) {
                return Err(TransitionError::MandateAlreadyExists(
                    mandate.mandate_id.clone(),
                ));
            }

            // Invariant: Fencing token must strictly increase for this agent
            let max_existing_token = next
                .mandates
                .values()
                .filter(|m| m.agent_id == mandate.agent_id)
                .map(|m| m.fencing_token)
                .max()
                .unwrap_or(0);

            if mandate.fencing_token <= max_existing_token {
                return Err(TransitionError::FencingTokenRegression {
                    agent_id: mandate.agent_id.clone(),
                    expected_gt: max_existing_token,
                    got: mandate.fencing_token,
                });
            }

            next.mandates
                .insert(mandate.mandate_id.clone(), mandate.clone());
        }
        LedgerTransactionPayload::WorkloadAdmitted {
            workload_id,
            mandate_id,
            target_node_id,
            action,
        } => {
            if next.workloads.contains_key(workload_id) {
                return Err(TransitionError::WorkloadAlreadyExists(workload_id.clone()));
            }
            let mandate = next
                .mandates
                .get(mandate_id)
                .ok_or_else(|| TransitionError::MandateNotFound(mandate_id.clone()))?;

            if !mandate.is_valid_at(tx.timestamp_unix_s) {
                return Err(TransitionError::MandateInvalid {
                    mandate_id: mandate_id.clone(),
                    reason: "mandate expired, exhausted, or not active at transaction timestamp"
                        .to_string(),
                });
            }

            if !next.nodes.contains_key(target_node_id) {
                return Err(TransitionError::NodeNotFound(target_node_id.clone()));
            }

            next.workloads.insert(
                workload_id.clone(),
                AdmittedWorkload {
                    workload_id: workload_id.clone(),
                    mandate_id: mandate_id.clone(),
                    target_node_id: target_node_id.clone(),
                    action: action.clone(),
                    phase: WorkloadExecutionPhase::Admitted,
                    admitted_at_unix_s: tx.timestamp_unix_s,
                    started_at_unix_s: None,
                    completed_at_unix_s: None,
                    cpu_cores_reserved: 0,
                    ram_bytes_reserved: 0,
                    actions_used: 0,
                    wall_seconds_used: 0,
                    success: None,
                    proof_digest: None,
                },
            );
        }
        LedgerTransactionPayload::WorkloadStarted {
            workload_id,
            started_at_unix_s,
        } => {
            let workload = next
                .workloads
                .get_mut(workload_id)
                .ok_or_else(|| TransitionError::WorkloadNotFound(workload_id.clone()))?;

            if workload.phase != WorkloadExecutionPhase::Admitted {
                return Err(TransitionError::WorkloadPhaseMismatch {
                    workload_id: workload_id.clone(),
                    expected: WorkloadExecutionPhase::Admitted,
                    got: workload.phase,
                });
            }

            workload.phase = WorkloadExecutionPhase::Started;
            workload.started_at_unix_s = Some(*started_at_unix_s);
        }
        LedgerTransactionPayload::ResourceReserved {
            workload_id,
            node_id,
            cpu_cores,
            ram_bytes,
        } => {
            let workload = next
                .workloads
                .get_mut(workload_id)
                .ok_or_else(|| TransitionError::WorkloadNotFound(workload_id.clone()))?;

            let resource = next
                .resources
                .values_mut()
                .find(|r| r.node_id == *node_id)
                .ok_or_else(|| TransitionError::ResourceNotFound(node_id.clone()))?;

            if resource.cpu_cores_available < *cpu_cores {
                return Err(TransitionError::ResourceQuotaExceeded {
                    node_id: node_id.clone(),
                    requested: u64::from(*cpu_cores),
                    available: u64::from(resource.cpu_cores_available),
                });
            }
            if resource.ram_bytes_available < *ram_bytes {
                return Err(TransitionError::ResourceQuotaExceeded {
                    node_id: node_id.clone(),
                    requested: *ram_bytes,
                    available: resource.ram_bytes_available,
                });
            }

            resource.cpu_cores_available -= *cpu_cores;
            resource.ram_bytes_available -= *ram_bytes;
            workload.cpu_cores_reserved += *cpu_cores;
            workload.ram_bytes_reserved += *ram_bytes;
        }
        LedgerTransactionPayload::ResourceConsumed {
            workload_id,
            node_id: _,
            actions_used,
            wall_seconds_used,
        } => {
            let workload = next
                .workloads
                .get_mut(workload_id)
                .ok_or_else(|| TransitionError::WorkloadNotFound(workload_id.clone()))?;

            workload.actions_used += *actions_used;
            workload.wall_seconds_used += *wall_seconds_used;

            if let Some(mandate) = next.mandates.get_mut(&workload.mandate_id) {
                mandate.consumed_actions += *actions_used;
                mandate.consumed_wall_seconds += *wall_seconds_used;
                if mandate.consumed_actions >= mandate.max_actions
                    || mandate.consumed_wall_seconds >= mandate.max_wall_seconds
                {
                    mandate.status = MandateStatus::Exhausted;
                }
            }
        }
        LedgerTransactionPayload::WorkloadCompleted {
            workload_id,
            success,
            completed_at_unix_s,
            proof_digest,
        } => {
            let workload = next
                .workloads
                .get_mut(workload_id)
                .ok_or_else(|| TransitionError::WorkloadNotFound(workload_id.clone()))?;

            workload.phase = WorkloadExecutionPhase::Completed;
            workload.completed_at_unix_s = Some(*completed_at_unix_s);
            workload.success = Some(*success);
            workload.proof_digest = Some(proof_digest.clone());
        }
        LedgerTransactionPayload::MandateRevoked {
            mandate_id,
            revocation_reason: _,
            revoked_at_unix_s: _,
        } => {
            let mandate = next
                .mandates
                .get_mut(mandate_id)
                .ok_or_else(|| TransitionError::MandateNotFound(mandate_id.clone()))?;
            mandate.status = MandateStatus::Revoked;
        }
        LedgerTransactionPayload::CalibrationCreditRecorded {
            treasury_id,
            amount,
            reason: _,
        } => {
            let treasury = next.treasury.entry(treasury_id.clone()).or_insert_with(|| {
                super::accounts::TreasuryAccount {
                    schema: super::accounts::TREASURY_ACCOUNT_SCHEMA.to_string(),
                    treasury_id: treasury_id.clone(),
                    cyd_calibration_credits: 0,
                    escrow_credits: 0,
                    disputed_credits: 0,
                    total_spent: 0,
                }
            });
            treasury.cyd_calibration_credits += *amount;
        }
        LedgerTransactionPayload::CreditSuspended {
            treasury_id,
            amount,
            dispute_id: _,
        } => {
            let treasury = next
                .treasury
                .get_mut(treasury_id)
                .ok_or_else(|| TransitionError::TreasuryNotFound(treasury_id.clone()))?;

            if treasury.cyd_calibration_credits < *amount {
                return Err(TransitionError::InsufficientCredits {
                    treasury_id: treasury_id.clone(),
                    requested: *amount,
                    available: treasury.cyd_calibration_credits,
                });
            }
            treasury.cyd_calibration_credits -= *amount;
            treasury.disputed_credits += *amount;
        }
        LedgerTransactionPayload::PeerCheckpointed {
            epoch,
            state_merkle_root_hex: _,
            checkpoint_unix_s: _,
        } => {
            next.epoch = *epoch;
            outcome = TransitionOutcome::Checkpointed { epoch: *epoch };
        }
        _ => {
            // Other events are recorded into transaction history without mutating accounts
        }
    }

    // 3. Update sequence and transaction hash
    next.last_sequence = tx.sequence;
    next.last_tx_hash = tx.tx_id.clone();

    // 4. Compute state root digest
    let state_root_digest = next.state_root_digest();

    let receipt = TransactionReceipt {
        tx_id: tx.tx_id.clone(),
        sequence: tx.sequence,
        timestamp_unix_s: tx.timestamp_unix_s,
        outcome,
        state_root_digest,
    };

    Ok((next, receipt))
}
