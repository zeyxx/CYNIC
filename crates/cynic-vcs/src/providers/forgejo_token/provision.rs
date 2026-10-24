//! Forgejo token observation and provisioning logic.

use std::path::PathBuf;

use cynic_types::paths::CynicPaths;
use crate::forgejo_token::{
    plan_forgejo_token_convergence, validate_forgejo_token_spec,
    ForgejoTokenActionKind, ForgejoTokenMetadata, ForgejoTokenObservation,
    ForgejoTokenPlan, ForgejoTokenProvisionReport, ForgejoTokenRuntimeSpec,
    ForgejoTokenSpec, ForgejoTokenStatusReport,
    FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA,
};

use super::api::*;
use super::{forgejo_token_directory, forgejo_token_path, observe_forgejo_token, ForgejoTokenApiPort};

/// This is the runtime entry point for `cynic forgejo token-status`.
pub fn observe_forgejo_token_runtime(spec: &ForgejoTokenRuntimeSpec) -> ForgejoTokenObservation {
    spec.adapter.observe(&spec.token_spec)
}

/// Provision (create/rotate) the token through the injected port and emit a
/// read-back proof report. Follows the `observe → apply → read-back` coherence
/// pattern. This is the runtime entry point for `cynic forgejo token-create|rotate`.
pub fn provision_forgejo_token_apply(
    spec: &ForgejoTokenRuntimeSpec,
) -> ForgejoTokenProvisionReport {
    provision_forgejo_token(&spec.token_spec, &*spec.adapter)
}
pub fn provision_forgejo_token(
    spec: &ForgejoTokenSpec,
    port: &dyn ForgejoTokenApiPort,
) -> ForgejoTokenProvisionReport {
    if spec.target_path != forgejo_token_path().display().to_string() {
        return ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
            status: "blocked".to_string(),
            principal: spec.principal.clone(),
            metadata: None,
            token_metadata: None,
            action_taken: "none".to_string(),
            rollback_intent: "no mutation; non-canonical token path".to_string(),
            unknowns: vec!["forgejo_token_target_path_noncanonical".to_string()],
        };
    }
    let observation = port.observe(spec);

    if observation.api_authenticated == ForgejoTokenAuthState::Proven {
        return ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
            status: "ready".to_string(),
            principal: spec.principal.clone(),
            metadata: None,
            token_metadata: None,
            action_taken: "none".to_string(),
            rollback_intent: "no mutation; nothing to roll back".to_string(),
            unknowns: Vec::new(),
        };
    }

    match port.create(spec) {
        Ok(metadata) => {
            let after = port.observe(spec);
            let converged = after.api_authenticated == ForgejoTokenAuthState::Proven;
            let mut unknowns = after.unknowns.clone();
            if !converged {
                unknowns.push("forgejo_token_provision_read_back_not_converged".to_string());
            }
            let sha1_for_rollback = metadata.sha1.clone();
            ForgejoTokenProvisionReport {
                schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
                status: if converged {
                    "completed".to_string()
                } else {
                    "blocked".to_string()
                },
                principal: spec.principal.clone(),
                metadata: if converged && metadata.sha1 != "<unavailable>" {
                    Some(metadata.clone())
                } else {
                    None
                },
                token_metadata: if converged && metadata.sha1 != "<unavailable>" {
                    Some(metadata)
                } else {
                    None
                },
                action_taken: if converged {
                    "created".to_string()
                } else {
                    "blocked".to_string()
                },
                rollback_intent: format!(
                    "revoke Forgejo token {} for principal '{}' via the same port",
                    sha1_for_rollback, spec.principal
                ),
                unknowns,
            }
        }
        Err(e) => ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
            status: "blocked".to_string(),
            principal: spec.principal.clone(),
            metadata: None,
            token_metadata: None,
            action_taken: "none".to_string(),
            rollback_intent: "no mutation due to error; nothing to roll back".to_string(),
            unknowns: vec![format!("forgejo_token_create_failed:{e}")],
        },
    }
}

/// Rotate by revoking the exact current token, creating its replacement, and
/// requiring a fresh observation proving the replacement.
pub fn rotate_forgejo_token(
    spec: &ForgejoTokenSpec,
    port: &dyn ForgejoTokenApiPort,
) -> ForgejoTokenProvisionReport {
    if spec.target_path != forgejo_token_path().display().to_string() {
        return ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
            status: "blocked".to_string(),
            principal: spec.principal.clone(),
            metadata: None,
            token_metadata: None,
            action_taken: "none".to_string(),
            rollback_intent: "no mutation; non-canonical token path".to_string(),
            unknowns: vec!["forgejo_token_target_path_noncanonical".to_string()],
        };
    }
    let before = port.observe(spec);
    let old = match port.current_token(spec) {
        Ok(metadata) if validate_token_identity(&metadata.sha1).is_ok() => metadata,
        _ => {
            return blocked_rotation_report(
                spec,
                port,
                before,
                "forgejo_token_current_metadata_unavailable",
            );
        }
    };
    if let Err(error) = port.remove(spec, &old.sha1) {
        return blocked_rotation_report(
            spec,
            port,
            before,
            &format!("forgejo_token_revoke_failed:{error}"),
        );
    }
    match port.is_revoked(spec, &old.sha1) {
        Ok(true) => {}
        Ok(false) => {
            return blocked_rotation_report(
                spec,
                port,
                before,
                "forgejo_token_revocation_read_back_refuted",
            );
        }
        Err(error) => {
            return blocked_rotation_report(
                spec,
                port,
                before,
                &format!("forgejo_token_revocation_read_back_unknown:{error}"),
            );
        }
    }
    match port.create(spec) {
        Ok(metadata) => {
            let after = port.observe(spec);
            let converged = after.api_authenticated == ForgejoTokenAuthState::Proven;
            let mut unknowns = after.unknowns.clone();
            if !converged {
                unknowns.push("forgejo_token_rotation_read_back_not_converged".to_string());
            }
            ForgejoTokenProvisionReport {
                schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
                status: if converged {
                    "completed".to_string()
                } else {
                    "blocked".to_string()
                },
                principal: spec.principal.clone(),
                metadata: if converged {
                    Some(metadata.clone())
                } else {
                    None
                },
                token_metadata: if converged { Some(metadata) } else { None },
                action_taken: if converged {
                    "rotated".to_string()
                } else {
                    "blocked".to_string()
                },
                rollback_intent: format!(
                    "revoke replacement token for principal '{}' via the same port",
                    spec.principal
                ),
                unknowns,
            }
        }
        Err(error) => blocked_rotation_report(
            spec,
            port,
            before,
            &format!("forgejo_token_create_failed_after_revoke:{error}"),
        ),
    }
}

fn blocked_rotation_report(
    spec: &ForgejoTokenSpec,
    port: &dyn ForgejoTokenApiPort,
    before: ForgejoTokenObservation,
    reason: &str,
) -> ForgejoTokenProvisionReport {
    ForgejoTokenProvisionReport {
        schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
        status: "blocked".to_string(),
        principal: spec.principal.clone(),
        metadata: None,
        token_metadata: None,
        action_taken: "none".to_string(),
        rollback_intent: "rotation blocked; inspect revocation and replacement state".to_string(),
        unknowns: vec![reason.to_string()],
    }
}

// ── internal helpers (local-exec + http) ──────────────────────────────
