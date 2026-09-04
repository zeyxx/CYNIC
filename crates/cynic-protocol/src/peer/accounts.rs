//! Canonical 7 Constitutional Accounts of a CynicPeer.
//!
//! Specifications: `protocols/cynic-government-architecture.md` and
//! `protocols/cynic-stage2-peer-and-ledger.md`.
//! Invariants: `contracts/foundation.json` §481 (one_boundary_one_reason)

use serde::{Deserialize, Serialize};

pub const CITIZEN_ACCOUNT_SCHEMA: &str = "cynic.peer.account.citizen.v1";
pub const NODE_ACCOUNT_SCHEMA: &str = "cynic.peer.account.node.v1";
pub const AGENT_ACCOUNT_SCHEMA: &str = "cynic.peer.account.agent.v1";
pub const OFFICE_ACCOUNT_SCHEMA: &str = "cynic.peer.account.office.v1";
pub const MANDATE_ACCOUNT_SCHEMA: &str = "cynic.peer.account.mandate.v1";
pub const RESOURCE_ACCOUNT_SCHEMA: &str = "cynic.peer.account.resource.v1";
pub const TREASURY_ACCOUNT_SCHEMA: &str = "cynic.peer.account.treasury.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Active,
    Suspended,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentControlMode {
    Hitl,
    Supervised,
    AutonomousAudit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MandateStatus {
    Active,
    Exhausted,
    Expired,
    Revoked,
    Disputed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeMembershipStatus {
    Candidate,
    Enrolled,
    Drained,
    Decommissioned,
}

/// 1. CitizenAccount: Constituent authority and decisions reserved for human operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitizenAccount {
    pub schema: &'static str,
    pub citizen_id: String,
    pub display_name: String,
    pub public_key_hex: String,
    pub is_sovereign_admin: bool,
    pub created_at_unix_s: u64,
    pub status: AccountStatus,
}

impl CitizenAccount {
    pub fn new_admin(
        citizen_id: impl Into<String>,
        display_name: impl Into<String>,
        public_key_hex: impl Into<String>,
        now: u64,
    ) -> Self {
        Self {
            schema: CITIZEN_ACCOUNT_SCHEMA,
            citizen_id: citizen_id.into(),
            display_name: display_name.into(),
            public_key_hex: public_key_hex.into(),
            is_sovereign_admin: true,
            created_at_unix_s: now,
            status: AccountStatus::Active,
        }
    }
}

/// 2. NodeAccount: Machine or compute environment hosting capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeAccount {
    pub schema: &'static str,
    pub node_id: String,
    pub owner_citizen_id: String,
    pub hostname: String,
    pub host_family: String,
    pub capabilities: Vec<String>,
    pub status: NodeMembershipStatus,
    pub last_liveness_proof_unix_s: Option<u64>,
}

/// 3. AgentAccount: Computational incarnation executing workloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentAccount {
    pub schema: &'static str,
    pub agent_id: String,
    pub parent_citizen_id: String,
    pub office_id: String,
    pub model_name: String,
    pub harness_kind: String,
    pub mode: AgentControlMode,
    pub created_at_unix_s: u64,
    pub status: AccountStatus,
}

/// 4. OfficeAccount: Institutional stable profession and its capability constraints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeAccount {
    pub schema: &'static str,
    pub office_id: String,
    pub title: String,
    pub allowed_capabilities: Vec<String>,
    pub forbidden_actions: Vec<String>,
}

/// 5. MandateAccount: Temporary, time-bounded, scoped authorization with budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MandateAccount {
    pub schema: &'static str,
    pub mandate_id: String,
    pub agent_id: String,
    pub office_id: String,
    pub scope: String,
    pub fencing_token: u64,
    pub started_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub max_actions: u64,
    pub consumed_actions: u64,
    pub max_wall_seconds: u64,
    pub consumed_wall_seconds: u64,
    pub status: MandateStatus,
}

impl MandateAccount {
    pub fn is_valid_at(&self, now: u64) -> bool {
        self.status == MandateStatus::Active
            && now >= self.started_at_unix_s
            && now <= self.expires_at_unix_s
            && self.consumed_actions < self.max_actions
            && self.consumed_wall_seconds < self.max_wall_seconds
    }
}

/// 6. ResourceAccount: Physically observed compute capacity (never minted by policy).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceAccount {
    pub schema: &'static str,
    pub resource_id: String,
    pub node_id: String,
    pub cpu_cores_available: u32,
    pub ram_bytes_available: u64,
    pub disk_bytes_available: u64,
    pub gpu_vram_bytes_available: u64,
    pub observed_at_unix_s: u64,
    pub observation_proof_digest: Option<String>,
}

/// 7. TreasuryAccount: Accounting balances of internal CYD calibration credits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreasuryAccount {
    pub schema: &'static str,
    pub treasury_id: String,
    pub cyd_calibration_credits: u64,
    pub escrow_credits: u64,
    pub disputed_credits: u64,
    pub total_spent: u64,
}
