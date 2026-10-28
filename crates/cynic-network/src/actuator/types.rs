//! Network Actuation Types & Profile Models
//! Clean-room extracted from cynic-node to cynic-network per Foundation §481 (SoC).
//! Pure data structures for actuation plans, bundles, and status reporting.
//!
//! Pure network actuation domain (FCIS): types and predicates with zero I/O.

use std::collections::BTreeMap;

use crate::command_surface::HostCommandSurface;
use cynic_protocol::{
    ExecutionAuthorization, MutationContract, PolicyAdmissionReport, PolicyAuthority,
    PolicyDecision, PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof,
};
use serde::{Deserialize, Serialize};

pub const NETWORK_ACTUATOR_MANIFEST_SCHEMA: &str = "cynic.network_actuator.bundle_manifest.v1";
pub const NETWORK_ACTUATOR_ACTIVATION_PLAN_SCHEMA: &str =
    "cynic.network_actuator.activation_plan.v1";
pub const NETWORK_ACTUATOR_APPLY_REPORT_SCHEMA: &str = "cynic.network_actuator.apply_report.v1";
pub const NETWORK_ACTUATOR_STATUS_SCHEMA: &str = "cynic.network_actuator.activation_status.v1";

pub const NETWORK_ACTUATION_REQUEST_SCHEMA: &str = "cynic.host.network_actuation_request.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkActuatorBundleManifest {
    pub schema: String,
    pub files: Vec<String>,
    pub binary_sha256: String,
    pub generated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkActuatorActivationPlan {
    pub schema: String,
    pub profile: String,
    pub commands: Vec<Vec<String>>,
    pub target_network: String,
    pub generated_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkActuatorApplyReport {
    pub schema: String,
    pub status: String,
    pub profile: String,
    pub applied_commands_count: usize,
    pub executed_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkActuatorActivationStatus {
    pub schema: String,
    pub active_profile: Option<String>,
    pub is_active: bool,
    pub last_checked_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkActuationPolicyDocument {
    pub schema: &'static str,
    pub status: &'static str,
    pub executor_path: String,
    pub sudoers_subject: &'static str,
    pub max_timeout_seconds: u64,
    pub allowed_profiles: BTreeMap<String, NetworkActuationAllowedProfile>,
    pub forbidden_tokens: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkActuationAllowedProfile {
    pub description: &'static str,
    pub command_prefixes: Vec<HostCommandSurface>,
}

#[derive(Debug, Clone)]
pub struct NetworkActuationRuntime {
    pub executor_path: String,
    pub sudoers_subject: &'static str,
    pub max_timeout_seconds: u64,
    pub service_manager_bin: String,
    pub wireguard_unit: String,
    pub route_bin: String,
    pub wireguard_bin: String,
    pub network_manager_bin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkActuationTarget {
    pub provider_fallback_cidr: String,
    pub provider_fallback_gateway: Option<String>,
    pub service_cidr: String,
    pub mesh_interface: String,
    #[serde(default)]
    pub management_interface: Option<String>,
    #[serde(default)]
    pub management_connection: Option<String>,
    #[serde(default)]
    pub management_address: Option<String>,
    #[serde(default)]
    pub management_gateway: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkActuationObservation {
    pub provider_fallback_active: bool,
    pub synapse_route_active: bool,
    pub wireguard_active: bool,
    #[serde(default)]
    pub management_link_active: bool,
    pub target: NetworkActuationTarget,
}

impl NetworkActuationObservation {
    pub fn allows_profile(&self, profile: &str) -> bool {
        match profile {
            "provider-fallback-route" => {
                !self.provider_fallback_active && self.target.provider_fallback_gateway.is_some()
            }
            "synapse-route" => !self.synapse_route_active,
            "wireguard-mesh" => !self.wireguard_active,
            "direct-management-link" => {
                !self.management_link_active
                    && self.target.management_interface.is_some()
                    && self.target.management_connection.is_some()
                    && self.target.management_address.is_some()
                    && self.target.management_gateway.is_some()
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkActuationPlan {
    pub schema: &'static str,
    pub id: String,
    pub profile: String,
    pub target: String,
    pub commands: Vec<HostCommandSurface>,
    pub preconditions: Vec<String>,
    pub rollback_intent: String,
    pub expected_proofs: Vec<RequiredProof>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkActuationRequest {
    pub schema: String,
    pub plan: NetworkActuationPlan,
    pub admission: PolicyAdmissionReport,
    pub authorization: ExecutionAuthorization,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkActuationWirePlan {
    pub schema: String,
    pub id: String,
    pub profile: String,
    pub target: String,
    pub commands: Vec<HostCommandSurface>,
    pub preconditions: Vec<String>,
    pub rollback_intent: String,
    pub expected_proofs: Vec<RequiredProof>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkActuationWireRequest {
    pub schema: String,
    pub plan: NetworkActuationWirePlan,
    pub admission: PolicyAdmissionReport,
    pub authorization: ExecutionAuthorization,
}

impl NetworkActuationWireRequest {
    pub fn into_request(self) -> Result<NetworkActuationRequest, String> {
        if self.schema != NETWORK_ACTUATION_REQUEST_SCHEMA {
            return Err("request_schema_invalid".to_string());
        }
        if self.plan.schema != "cynic.host.network_actuation_plan.v1" {
            return Err("plan_schema_invalid".to_string());
        }
        Ok(NetworkActuationRequest {
            schema: self.schema,
            plan: NetworkActuationPlan {
                schema: "cynic.host.network_actuation_plan.v1",
                id: self.plan.id,
                profile: self.plan.profile,
                target: self.plan.target,
                commands: self.plan.commands,
                preconditions: self.plan.preconditions,
                rollback_intent: self.plan.rollback_intent,
                expected_proofs: self.plan.expected_proofs,
            },
            admission: self.admission,
            authorization: self.authorization,
        })
    }
}