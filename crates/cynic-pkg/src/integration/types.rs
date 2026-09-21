//! Provider-agnostic integration observation types.
//!
//! These shapes describe presence, execution identity, and connection presence.
//! They never contain credential values.

use serde::{Deserialize, Serialize};

use super::lifecycle::{BoundaryKind, LifecycleDisposition, LifecycleStatus};
use crate::types::{ToolingAuthorityFamily, ToolingInstallProvenance};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationKind {
    ManagedService,
    ExternalCli,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationPrincipal {
    RuntimeUser,
    OperatorLocal,
    SystemManaged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationCapability {
    ManagedService,
    RepositoryAuthority,
    PublicRelay,
    ModelHub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegrationDependencySpec {
    pub id: &'static str,
    pub install_principal: IntegrationPrincipal,
    pub tooling_authority_family: Option<ToolingAuthorityFamily>,
    pub expected_tooling_provenance: Option<ToolingInstallProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationDependencyView {
    pub id: String,
    pub install_principal: IntegrationPrincipal,
    pub tooling_authority: Option<IntegrationToolingAuthorityView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationToolingAuthorityView {
    pub integration_id: Option<String>,
    pub dependency_id: Option<String>,
    pub family: ToolingAuthorityFamily,
    pub observed_provenance: Option<ToolingInstallProvenance>,
    pub expected_provenance: Option<ToolingInstallProvenance>,
    pub status: String,
    pub observed_payload_provenance: Option<String>,
    pub expected_payload_provenance: Option<String>,
    pub canonical_update_command: Option<String>,
    pub update_authority_note: Option<String>,
    pub normalizable_now: bool,
    pub requires_uv: bool,
    pub requires_runtime_principal: bool,
    pub can_auto_apply: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegrationSpec {
    pub id: &'static str,
    pub kind: IntegrationKind,
    pub desired_principal: IntegrationPrincipal,
    pub dependency: Option<IntegrationDependencySpec>,
    pub connection_required: bool,
    pub capabilities: &'static [IntegrationCapability],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationObservationContext {
    pub runtime_user: String,
    pub runtime_home: String,
    pub operator_user: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawIntegrationObservation {
    pub present: bool,
    pub path_or_endpoint: Option<String>,
    pub version: Option<String>,
    pub observed_principal: Option<IntegrationPrincipal>,
    pub principal_name: Option<String>,
    pub connection: ConnectionState,
    pub account: Option<String>,
    pub credential_source: Option<String>,
    pub notes: Vec<String>,
}

impl RawIntegrationObservation {
    pub fn missing(note: impl Into<String>) -> Self {
        Self {
            present: false,
            path_or_endpoint: None,
            version: None,
            observed_principal: None,
            principal_name: None,
            connection: ConnectionState::Unknown,
            account: None,
            credential_source: None,
            notes: vec![note.into()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationConnectionObservation {
    pub state: ConnectionState,
    pub account: Option<String>,
    pub credential_source: Option<String>,
    pub secret_exposed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationObservation {
    pub id: String,
    pub kind: IntegrationKind,
    pub capabilities: Vec<IntegrationCapability>,
    pub status: String,
    pub lifecycle_status: LifecycleStatus,
    pub present: bool,
    pub path_or_endpoint: Option<String>,
    pub version: Option<String>,
    pub desired_principal: IntegrationPrincipal,
    pub dependency: Option<IntegrationDependencyView>,
    pub observed_principal: Option<IntegrationPrincipal>,
    pub principal_name: Option<String>,
    pub principal_matches: bool,
    pub connection: IntegrationConnectionObservation,
    pub adoption_status: String,
    pub lifecycle_disposition: LifecycleDisposition,
    pub next_boundary: String,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    pub next_command: Option<String>,
    pub notes: Vec<String>,
    pub proofs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationStatusSummary {
    pub total: usize,
    pub ready: usize,
    pub missing: usize,
    pub principal_mismatch: usize,
    pub connection_required: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationsStatusReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub runtime_user: String,
    pub operator_user: String,
    pub integrations: Vec<IntegrationObservation>,
    pub dependency_tooling_authorities: Vec<IntegrationToolingAuthorityView>,
    pub summary: IntegrationStatusSummary,
    pub rules: Vec<&'static str>,
}
