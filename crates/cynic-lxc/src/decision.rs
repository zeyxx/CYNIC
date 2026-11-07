use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::CynicErrorCode;
use cynic_protocol::privilege::{admitted_operation, safe_absolute_path};

use super::types::{LxcBridgeAuthority, LxcBridgeDecision, LxcBridgeRequest, REQUEST_SCHEMA};

pub fn admit(request: &LxcBridgeRequest, pct_bin: &str) -> LxcBridgeDecision {
    let now_unix_s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(u64::MAX);
    admit_at_with_incident_authority(request, pct_bin, now_unix_s, false)
}

#[allow(dead_code)]
pub fn admit_at(request: &LxcBridgeRequest, pct_bin: &str, now_unix_s: u64) -> LxcBridgeDecision {
    admit_at_with_incident_authority(request, pct_bin, now_unix_s, false)
}

pub(crate) fn admit_after_incident_authority(
    request: &LxcBridgeRequest,
    pct_bin: &str,
    now_unix_s: u64,
) -> LxcBridgeDecision {
    admit_at_with_incident_authority(request, pct_bin, now_unix_s, true)
}

fn admit_at_with_incident_authority(
    request: &LxcBridgeRequest,
    pct_bin: &str,
    now_unix_s: u64,
    incident_authority_verified: bool,
) -> LxcBridgeDecision {
    let mut blockers = Vec::new();
    if request.schema != REQUEST_SCHEMA {
        blockers.push("request_schema_mismatch".to_string());
    }
    if !(100..=999_999_999).contains(&request.vmid) {
        blockers.push("vmid_out_of_range".to_string());
    }
    if request.purpose.trim().is_empty() || request.purpose.len() > 256 {
        blockers.push("purpose_invalid".to_string());
    }
    if request.agent_id.trim().is_empty() || request.agent_id.len() > 256 {
        blockers.push("agent_id_invalid".to_string());
    }
    if !(1..=60).contains(&request.timeout_seconds) {
        blockers.push("timeout_out_of_range".to_string());
    }
    if !safe_absolute_path(pct_bin) {
        blockers.push("pct_adapter_path_invalid".to_string());
    }

    let mutates = match admitted_operation(&request.argv) {
        Ok(value) => value,
        Err(blocker) => {
            blockers.push(blocker);
            false
        }
    };
    if mutates && request.authority != LxcBridgeAuthority::RecoveryBreakglass {
        blockers.push(
            CynicErrorCode::LxcMutationAuthorityUnverified
                .code()
                .to_string(),
        );
    }
    if request.authority == LxcBridgeAuthority::RecoveryBreakglass {
        let Some(admission) = request.breakglass_admission.as_ref() else {
            blockers.push(
                CynicErrorCode::BreakglassAdmissionMissing
                    .code()
                    .to_string(),
            );
            return LxcBridgeDecision::Blocked { blockers };
        };
        let action = canonical_action(&request.argv).unwrap_or_default();
        blockers.extend(
            admission
                .validate_for(&format!("lxc:{}", request.vmid), &action, now_unix_s)
                .into_iter()
                .map(|error| error.code().to_string()),
        );
        // The root helper currently has no incident-authority verifier and no
        // durable, atomic receipt-consumption store. A caller-provided JSON
        // admission is therefore not authority for a privileged action.
        if !incident_authority_verified {
            blockers.push(
                CynicErrorCode::BreakglassAuthorityVerifierUnavailable
                    .code()
                    .to_string(),
            );
        }
    }
    if mutates && !request.apply {
        blockers.push("mutation_requires_apply".to_string());
    }
    if !blockers.is_empty() {
        return LxcBridgeDecision::Blocked { blockers };
    }

    let mut pct_argv = vec![
        pct_bin.to_string(),
        "exec".to_string(),
        request.vmid.to_string(),
        "--".to_string(),
    ];
    pct_argv.extend(request.argv.clone());
    if mutates {
        LxcBridgeDecision::Apply { pct_argv }
    } else {
        LxcBridgeDecision::Observe { pct_argv }
    }
}

fn canonical_action(argv: &[String]) -> Option<String> {
    match argv {
        [program, operation, unit]
            if program == "/usr/bin/systemctl"
                && matches!(
                    operation.as_str(),
                    "start" | "stop" | "restart" | "enable" | "disable"
                ) =>
        {
            Some(format!("systemctl:{operation}:{unit}"))
        }
        _ => None,
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LxcBridgeAuthority, LxcBridgeRequest};

    fn request(argv: &[&str], apply: bool) -> LxcBridgeRequest {
        LxcBridgeRequest {
            schema: REQUEST_SCHEMA.to_string(),
            vmid: 103,
            purpose: "storage health".to_string(),
            agent_id: "agent-a".to_string(),
            authority: LxcBridgeAuthority::RuntimeOperator,
            breakglass_admission: None,
            argv: argv.iter().map(|value| value.to_string()).collect(),
            timeout_seconds: 10,
            apply,
        }
    }

    #[test]
    fn read_only_service_status_is_admitted_without_apply() {
        assert!(matches!(
            admit(
                &request(
                    &["/usr/bin/systemctl", "is-active", "surrealdb.service"],
                    false
                ),
                "/usr/sbin/pct"
            ),
            LxcBridgeDecision::Observe { .. }
        ));
    }

    #[test]
    fn mutation_requires_apply_and_shells_are_never_admitted() {
        assert!(matches!(
            admit(
                &request(
                    &["/usr/bin/systemctl", "restart", "surrealdb.service"],
                    false
                ),
                "/usr/sbin/pct"
            ),
            LxcBridgeDecision::Blocked { .. }
        ));
        assert!(matches!(
            admit(&request(&["/bin/sh", "-c", "id"], true), "/usr/sbin/pct"),
            LxcBridgeDecision::Blocked { .. }
        ));
        assert!(matches!(
            admit(
                &request(
                    &["/usr/bin/systemctl", "restart", "surrealdb.service"],
                    true
                ),
                "/usr/sbin/pct"
            ),
            LxcBridgeDecision::Blocked { .. }
        ));
    }

    #[test]
    fn breakglass_fails_closed_without_an_incident_authority_verifier() {
        let mut request = request(
            &["/usr/bin/systemctl", "restart", "cynic-node.service"],
            true,
        );
        request.authority = LxcBridgeAuthority::RecoveryBreakglass;
        assert!(matches!(
            admit_at(&request, "/usr/sbin/pct", 1),
            LxcBridgeDecision::Blocked { .. }
        ));
        request.breakglass_admission = Some(cynic_protocol::BreakglassAdmission {
            schema: cynic_protocol::BREAKGLASS_ADMISSION_SCHEMA.to_string(),
            status: "admitted".to_string(),
            authority_class: "RecoveryBreakglass".to_string(),
            receipt_id: "INC-1".to_string(),
            target_scope: "lxc:103".to_string(),
            expires_at_unix_s: 2,
            allowed_actions: vec!["systemctl:restart:cynic-node.service".to_string()],
            consumption_proof: None,
            evidence_refs: vec!["incident:INC-1".to_string()],
            authority_key_id: "sha256:test".to_string(),
            signature_hex: "signature".to_string(),
        });
        assert!(matches!(
            admit_at(&request, "/usr/sbin/pct", 1),
            LxcBridgeDecision::Blocked { blockers }
                if blockers.contains(&"CYNIC-BG-006".to_string())
        ));
    }
}
