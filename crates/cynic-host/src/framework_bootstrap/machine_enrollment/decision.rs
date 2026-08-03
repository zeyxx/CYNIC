use super::types::{
    MACHINE_ENROLLMENT_AUTHORITY_PLAN_SCHEMA, MachineEnrollmentAuthorityAction,
    MachineEnrollmentAuthorityObservation, MachineEnrollmentAuthorityPlan,
    MachineEnrollmentAuthoritySpec,
};

pub fn plan_machine_enrollment_authority(
    spec: &MachineEnrollmentAuthoritySpec,
    observed: &MachineEnrollmentAuthorityObservation,
) -> MachineEnrollmentAuthorityPlan {
    let mut blockers = Vec::new();
    if spec.now_unix_s == 0 {
        blockers.push("machine_enrollment_clock_invalid".to_string());
    }
    if spec.requested_capabilities.is_empty() {
        blockers.push("machine_enrollment_requested_capabilities_missing".to_string());
    }
    if observed.signing_key_present && observed.signing_key_valid != Some(true) {
        blockers.push("machine_enrollment_existing_signing_key_invalid".to_string());
    }
    if observed.authority_present && observed.authority_valid != Some(true) {
        blockers.push("machine_enrollment_existing_authority_invalid".to_string());
    }
    if observed.authority_present && !observed.signing_key_present {
        blockers.push("machine_enrollment_authority_without_signing_key".to_string());
    }
    let converged = blockers.is_empty()
        && observed.authority_id.as_deref() == Some(spec.policy.authority_id.as_str())
        && observed.seed_id.as_deref() == Some(spec.seed_id.as_str())
        && observed.signing_key_mode == Some(0o640)
        && observed.signing_key_owner_id == Some(spec.owner_id)
        && observed.signing_key_group_id == Some(spec.group_id)
        && observed.staged_public_key_matches == Some(true)
        && observed.staged_token_matches == Some(true);
    let actions = if blockers.is_empty() && !converged {
        let mut actions = Vec::new();
        if !observed.signing_key_present {
            actions.push(MachineEnrollmentAuthorityAction::CreateSigningKey);
        } else if observed.signing_key_mode != Some(0o640)
            || observed.signing_key_owner_id != Some(spec.owner_id)
            || observed.signing_key_group_id != Some(spec.group_id)
        {
            actions.push(MachineEnrollmentAuthorityAction::RepairSigningKeyProjection);
        }
        actions.extend([
            MachineEnrollmentAuthorityAction::StageAuthorityPublicKey,
            MachineEnrollmentAuthorityAction::StageOneTimeToken,
            MachineEnrollmentAuthorityAction::InstallEnrollmentAuthority,
        ]);
        actions
    } else {
        Vec::new()
    };
    blockers.sort();
    blockers.dedup();
    let status = if !blockers.is_empty() {
        "blocked"
    } else if converged {
        "converged"
    } else {
        "planned"
    };

    MachineEnrollmentAuthorityPlan {
        schema: MACHINE_ENROLLMENT_AUTHORITY_PLAN_SCHEMA,
        status,
        mutates: false,
        authority_role: spec.authority_role.clone(),
        endpoint_service: spec.endpoint_service.clone(),
        node_id: spec.node_id.clone(),
        seed_id: spec.seed_id.clone(),
        role: spec.role.clone(),
        requested_capabilities: spec.requested_capabilities.clone(),
        actions,
        blockers,
        residual_unknowns: observed.unknowns.clone(),
        required_proofs: vec![
            "enrollment_authority_read_back",
            "enrollment_signing_key_mode_read_back",
            "staged_authority_public_key_read_back",
            "staged_one_time_token_digest_read_back",
        ],
        rollback_intent: "restore the previous authority envelope and remove only newly staged credential material; never rotate an existing signing key implicitly",
    }
}
