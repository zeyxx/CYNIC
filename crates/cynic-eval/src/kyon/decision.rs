use super::types::{
    ConsentGrant, DataClass, DisclosureAdmission, DisclosureRequest,
    KYON_FEDERATED_DISCLOSURE_SCHEMA, KYON_MIRROR_SIGNAL_SCHEMA, KYON_PROTECTIVE_PLAN_SCHEMA,
    KyonAction, MirrorSignal, ProtectivePlan, ShieldPolicy, SignalAssessment,
};

const MAX_BASIS_POINTS: u16 = 10_000;

pub fn reconcile_signal(signal: &MirrorSignal, policy: &ShieldPolicy) -> ProtectivePlan {
    let mut blockers = Vec::new();
    let mut unknowns = Vec::new();

    if signal.schema != KYON_MIRROR_SIGNAL_SCHEMA {
        blockers.push("signal_schema_mismatch".to_string());
    }
    if signal.signal_id.trim().is_empty() {
        blockers.push("signal_id_missing".to_string());
    }
    if signal.subject_node_id.trim().is_empty() {
        blockers.push("subject_node_id_missing".to_string());
    }
    if signal.kind.trim().is_empty() {
        blockers.push("signal_kind_missing".to_string());
    }
    if signal.severity_basis_points > MAX_BASIS_POINTS {
        blockers.push("signal_severity_out_of_range".to_string());
    }
    if signal.uncertainty_basis_points > MAX_BASIS_POINTS {
        blockers.push("signal_uncertainty_out_of_range".to_string());
    }
    if signal.observation_refs.is_empty() {
        unknowns.push("signal_has_no_observation_evidence".to_string());
    }
    if signal.contains_raw_content {
        blockers.push("mirror_signal_must_not_embed_raw_content".to_string());
    }
    if policy.policy_id.trim().is_empty() || policy.policy_version.trim().is_empty() {
        blockers.push("shield_policy_identity_incomplete".to_string());
    }
    if policy.concerning_threshold_basis_points > MAX_BASIS_POINTS {
        blockers.push("shield_policy_threshold_out_of_range".to_string());
    }

    let action = match signal.assessment {
        SignalAssessment::Benign => KyonAction::Allow,
        SignalAssessment::Unknown => {
            unknowns.push("signal_assessment_unknown".to_string());
            policy.unknown_action
        }
        SignalAssessment::Concerning
            if signal.severity_basis_points >= policy.concerning_threshold_basis_points =>
        {
            policy.concerning_action
        }
        SignalAssessment::Concerning => KyonAction::Warn,
    };

    let action = if blockers.is_empty() {
        action
    } else {
        KyonAction::Allow
    };
    let mut evidence_refs = signal
        .observation_refs
        .iter()
        .map(|observation| observation.evidence_ref.clone())
        .collect::<Vec<_>>();
    evidence_refs.sort();
    evidence_refs.dedup();

    ProtectivePlan {
        schema: KYON_PROTECTIVE_PLAN_SCHEMA.to_string(),
        signal_id: signal.signal_id.clone(),
        policy_id: policy.policy_id.clone(),
        policy_version: policy.policy_version.clone(),
        action,
        mutates_local_state: action != KyonAction::Allow,
        requires_user_confirmation: matches!(
            action,
            KyonAction::Block | KyonAction::PreserveEvidence
        ),
        blockers,
        evidence_refs,
        unknowns,
    }
}

pub fn admit_federated_disclosure(
    request: &DisclosureRequest,
    grant: Option<&ConsentGrant>,
) -> DisclosureAdmission {
    let mut blockers = Vec::new();

    if request.schema != KYON_FEDERATED_DISCLOSURE_SCHEMA {
        blockers.push("disclosure_schema_mismatch".to_string());
    }
    if request.request_id.trim().is_empty() {
        blockers.push("disclosure_request_id_missing".to_string());
    }
    if request.evidence_refs.is_empty() {
        blockers.push("disclosure_evidence_missing".to_string());
    }
    if request.data_class == DataClass::RawPrivate {
        blockers.push("raw_private_data_is_not_federatable".to_string());
    }

    if let Some(grant) = grant {
        if grant.subject_node_id != request.subject_node_id {
            blockers.push("consent_subject_mismatch".to_string());
        }
        if grant.recipient_id != request.recipient_id {
            blockers.push("consent_recipient_mismatch".to_string());
        }
        if !grant.purposes.contains(&request.purpose) {
            blockers.push("consent_purpose_not_granted".to_string());
        }
        if !grant.allowed_data_classes.contains(&request.data_class) {
            blockers.push("consent_data_class_not_granted".to_string());
        }
        if request.requested_at_unix_ms < grant.issued_at_unix_ms
            || request.requested_at_unix_ms >= grant.expires_at_unix_ms
        {
            blockers.push("consent_not_active_at_request_time".to_string());
        }
        if grant
            .revoked_at_unix_ms
            .is_some_and(|revoked_at| revoked_at <= request.requested_at_unix_ms)
        {
            blockers.push("consent_revoked".to_string());
        }
    } else {
        blockers.push("explicit_consent_grant_missing".to_string());
    }

    DisclosureAdmission {
        schema: "cynic.kyon.disclosure_admission.v1".to_string(),
        request_id: request.request_id.clone(),
        status: if blockers.is_empty() {
            "admitted".to_string()
        } else {
            "blocked".to_string()
        },
        grant_id: grant.map(|grant| grant.grant_id.clone()),
        blockers,
        evidence_refs: request.evidence_refs.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kyon::ObservationRef;

    fn signal(assessment: SignalAssessment, severity_basis_points: u16) -> MirrorSignal {
        MirrorSignal {
            schema: KYON_MIRROR_SIGNAL_SCHEMA.to_string(),
            signal_id: "signal-1".to_string(),
            subject_node_id: "alice-phone".to_string(),
            kind: "unsolicited_media_pattern".to_string(),
            assessment,
            severity_basis_points,
            uncertainty_basis_points: 500,
            observation_refs: vec![ObservationRef {
                evidence_ref: "sha256:event".to_string(),
                data_class: DataClass::Commitment,
                observed_at_unix_ms: 10,
                source_kind: "device_media".to_string(),
                raw_content_retained: false,
            }],
            contains_raw_content: false,
        }
    }

    fn policy() -> ShieldPolicy {
        ShieldPolicy {
            policy_id: "alice-default".to_string(),
            policy_version: "1".to_string(),
            concerning_threshold_basis_points: 7_000,
            concerning_action: KyonAction::Quarantine,
            unknown_action: KyonAction::Blur,
        }
    }

    #[test]
    fn mirror_signal_is_reconciled_by_the_subject_policy() {
        let plan = reconcile_signal(&signal(SignalAssessment::Concerning, 8_000), &policy());

        assert_eq!(plan.action, KyonAction::Quarantine);
        assert!(plan.mutates_local_state);
        assert!(plan.blockers.is_empty());
        assert_eq!(plan.evidence_refs, vec!["sha256:event"]);
    }

    #[test]
    fn invalid_or_raw_signal_fails_open_without_claiming_convergence() {
        let mut signal = signal(SignalAssessment::Concerning, 9_000);
        signal.contains_raw_content = true;

        let plan = reconcile_signal(&signal, &policy());

        assert_eq!(plan.action, KyonAction::Allow);
        assert!(
            plan.blockers
                .contains(&"mirror_signal_must_not_embed_raw_content".to_string())
        );
    }

    #[test]
    fn unknown_is_preserved_and_uses_the_explicit_subject_policy() {
        let plan = reconcile_signal(&signal(SignalAssessment::Unknown, 0), &policy());

        assert_eq!(plan.action, KyonAction::Blur);
        assert!(
            plan.unknowns
                .contains(&"signal_assessment_unknown".to_string())
        );
    }

    #[test]
    fn federation_requires_active_scoped_consent() {
        let request = DisclosureRequest {
            schema: KYON_FEDERATED_DISCLOSURE_SCHEMA.to_string(),
            request_id: "share-1".to_string(),
            subject_node_id: "alice-phone".to_string(),
            recipient_id: "trusted-node".to_string(),
            purpose: "safeguarding".to_string(),
            data_class: DataClass::ZeroKnowledgeAttestation,
            evidence_refs: vec!["sha256:proof".to_string()],
            requested_at_unix_ms: 20,
        };
        let grant = ConsentGrant {
            grant_id: "grant-1".to_string(),
            subject_node_id: "alice-phone".to_string(),
            recipient_id: "trusted-node".to_string(),
            purposes: vec!["safeguarding".to_string()],
            allowed_data_classes: vec![DataClass::ZeroKnowledgeAttestation],
            issued_at_unix_ms: 10,
            expires_at_unix_ms: 30,
            revoked_at_unix_ms: None,
        };

        let admitted = admit_federated_disclosure(&request, Some(&grant));
        assert_eq!(admitted.status, "admitted");

        let blocked = admit_federated_disclosure(&request, None);
        assert_eq!(blocked.status, "blocked");
        assert!(
            blocked
                .blockers
                .contains(&"explicit_consent_grant_missing".to_string())
        );
    }

    #[test]
    fn raw_private_material_never_enters_federation_even_with_consent() {
        let request = DisclosureRequest {
            schema: KYON_FEDERATED_DISCLOSURE_SCHEMA.to_string(),
            request_id: "share-raw".to_string(),
            subject_node_id: "alice-phone".to_string(),
            recipient_id: "trusted-node".to_string(),
            purpose: "safeguarding".to_string(),
            data_class: DataClass::RawPrivate,
            evidence_refs: vec!["sha256:raw".to_string()],
            requested_at_unix_ms: 20,
        };
        let grant = ConsentGrant {
            grant_id: "grant-raw".to_string(),
            subject_node_id: "alice-phone".to_string(),
            recipient_id: "trusted-node".to_string(),
            purposes: vec!["safeguarding".to_string()],
            allowed_data_classes: vec![DataClass::RawPrivate],
            issued_at_unix_ms: 10,
            expires_at_unix_ms: 30,
            revoked_at_unix_ms: None,
        };

        let admission = admit_federated_disclosure(&request, Some(&grant));

        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"raw_private_data_is_not_federatable".to_string())
        );
    }
}
