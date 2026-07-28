use serde::{Deserialize, Serialize};

pub const KYON_MIRROR_SIGNAL_SCHEMA: &str = "cynic.kyon.mirror_signal.v1";
pub const KYON_PROTECTIVE_PLAN_SCHEMA: &str = "cynic.kyon.protective_plan.v1";
pub const KYON_FEDERATED_DISCLOSURE_SCHEMA: &str = "cynic.kyon.federated_disclosure.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataClass {
    RawPrivate,
    DerivedPrivate,
    Commitment,
    ZeroKnowledgeAttestation,
    ConsentedEvidence,
    PublicAggregate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationRef {
    pub evidence_ref: String,
    pub data_class: DataClass,
    pub observed_at_unix_ms: u64,
    pub source_kind: String,
    pub raw_content_retained: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalAssessment {
    Benign,
    Concerning,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorSignal {
    pub schema: String,
    pub signal_id: String,
    pub subject_node_id: String,
    pub kind: String,
    pub assessment: SignalAssessment,
    pub severity_basis_points: u16,
    pub uncertainty_basis_points: u16,
    pub observation_refs: Vec<ObservationRef>,
    pub contains_raw_content: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KyonAction {
    Allow,
    Warn,
    Blur,
    Quarantine,
    Hide,
    Silence,
    Block,
    PreserveEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShieldPolicy {
    pub policy_id: String,
    pub policy_version: String,
    pub concerning_threshold_basis_points: u16,
    pub concerning_action: KyonAction,
    pub unknown_action: KyonAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectivePlan {
    pub schema: String,
    pub signal_id: String,
    pub policy_id: String,
    pub policy_version: String,
    pub action: KyonAction,
    pub mutates_local_state: bool,
    pub requires_user_confirmation: bool,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentGrant {
    pub grant_id: String,
    pub subject_node_id: String,
    pub recipient_id: String,
    pub purposes: Vec<String>,
    pub allowed_data_classes: Vec<DataClass>,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub revoked_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisclosureRequest {
    pub schema: String,
    pub request_id: String,
    pub subject_node_id: String,
    pub recipient_id: String,
    pub purpose: String,
    pub data_class: DataClass,
    pub evidence_refs: Vec<String>,
    pub requested_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisclosureAdmission {
    pub schema: String,
    pub request_id: String,
    pub status: String,
    pub grant_id: Option<String>,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}
