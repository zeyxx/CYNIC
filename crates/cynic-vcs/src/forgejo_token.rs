//! Forgejo personal access token lifecycle — pure domain types and planning.

use serde::{Deserialize, Serialize};

use crate::ForgejoRepositoryPermission;

pub const FORGEJO_TOKEN_OBSERVATION_SCHEMA: &str = "cynic.forgejo_token.observation.v1";
pub const FORGEJO_TOKEN_SPEC_SCHEMA: &str = "cynic.forgejo_token.spec.v1";
pub const FORGEJO_TOKEN_STATUS_SCHEMA: &str = "cynic.forgejo_token.status.v1";
pub const FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA: &str = "cynic.forgejo_token.provision_report.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForgejoTokenAuthState {
    Unknown,
    Proven,
    Refuted,
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForgejoTokenActionKind {
    Observe,
    Create,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenSpec {
    pub schema: String,
    pub principal: String,
    pub api_url: String,
    pub target_path: String,
    pub mode: u32,
    pub scopes: Vec<String>,
    pub expiry_seconds: Option<u64>,
}

impl Default for ForgejoTokenSpec {
    fn default() -> Self {
        Self {
            schema: FORGEJO_TOKEN_SPEC_SCHEMA.to_string(),
            principal: String::new(),
            api_url: String::new(),
            target_path: "/etc/cynic/secrets/forgejo-token".to_string(),
            mode: 0o640,
            scopes: vec![
                "write:repository".to_string(),
                "write:organization".to_string(),
                "write:admin".to_string(),
            ],
            expiry_seconds: Some(7_776_000),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenObservation {
    pub schema: String,
    pub principal: String,
    pub path: String,
    pub secret_present: bool,
    pub owner_root: Option<bool>,
    pub mode_o640: Option<bool>,
    pub api_authenticated: ForgejoTokenAuthState,
    pub unknowns: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenMetadata {
    pub token_id: u64,
    pub sha1: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenPlanAction {
    pub kind: ForgejoTokenActionKind,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenPlan {
    pub status: String,
    pub actions: Vec<ForgejoTokenPlanAction>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenStatusReport {
    pub schema: String,
    pub principal: String,
    pub token: ForgejoTokenObservation,
    pub forgejo_plan_status: String,
    pub token_action_needed: bool,
    pub requires_human_admin_secret: bool,
    pub plan_action: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenProvisionReport {
    pub schema: String,
    pub principal: String,
    pub status: String,
    pub metadata: Option<ForgejoTokenMetadata>,
    pub token_metadata: Option<ForgejoTokenMetadata>,
    pub action_taken: String,
    pub rollback_intent: String,
    pub unknowns: Vec<String>,
}

pub fn plan_forgejo_token_convergence(
    spec: ForgejoTokenSpec,
    observation: &ForgejoTokenObservation,
) -> ForgejoTokenPlan {
    let mut actions: Vec<ForgejoTokenPlanAction> = Vec::new();
    let mut evidence_refs: Vec<String> = Vec::new();

    if !observation.secret_present {
        actions.push(ForgejoTokenPlanAction {
            kind: ForgejoTokenActionKind::Create,
            reason: "secret_absent".to_string(),
        });
        evidence_refs.push(format!("token-path:{}", spec.target_path));
        return ForgejoTokenPlan {
            status: "blocked".to_string(),
            actions,
            evidence_refs,
        };
    }

    if observation.api_authenticated == ForgejoTokenAuthState::Absent {
        actions.push(ForgejoTokenPlanAction {
            kind: ForgejoTokenActionKind::Create,
            reason: "token_revoked_or_expired".to_string(),
        });
        return ForgejoTokenPlan {
            status: "blocked".to_string(),
            actions,
            evidence_refs,
        };
    }

    if observation.api_authenticated == ForgejoTokenAuthState::Refuted {
        actions.push(ForgejoTokenPlanAction {
            kind: ForgejoTokenActionKind::Remove,
            reason: "token_refuted".to_string(),
        });
        actions.push(ForgejoTokenPlanAction {
            kind: ForgejoTokenActionKind::Create,
            reason: "token_refuted".to_string(),
        });
        return ForgejoTokenPlan {
            status: "blocked".to_string(),
            actions,
            evidence_refs,
        };
    }

    if observation.owner_root == Some(false) || observation.mode_o640 == Some(false) {
        actions.push(ForgejoTokenPlanAction {
            kind: ForgejoTokenActionKind::Remove,
            reason: "integrity_gate_failed".to_string(),
        });
        return ForgejoTokenPlan {
            status: "blocked".to_string(),
            actions,
            evidence_refs,
        };
    }

    ForgejoTokenPlan {
        status: "ready".to_string(),
        actions,
        evidence_refs,
    }
}

pub fn validate_forgejo_token_spec(spec: &ForgejoTokenSpec) -> Vec<String> {
    let mut blockers: Vec<String> = Vec::new();
    if spec.principal.is_empty() {
        blockers.push("forgejo_token_spec.principal.empty".to_string());
    }
    if spec.api_url.is_empty() {
        blockers.push("forgejo_token_spec.api_url.empty".to_string());
    }
    if spec.target_path.is_empty() {
        blockers.push("forgejo_token_spec.target_path.empty".to_string());
    }
    if spec.scopes.is_empty() {
        blockers.push("forgejo_token_spec.scopes.empty".to_string());
    }
    blockers
}
