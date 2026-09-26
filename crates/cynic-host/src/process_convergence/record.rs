use cynic_eval::convergence::{
    CausalConvergenceEpochInput, CausalEvidenceRefs, TrustAssessment, TrustDimensionStatus,
    TrustRequirements, assess_causal_convergence_epoch,
};
use cynic_protocol::{
    EvidenceEnvelope, EvidenceRef, EvidenceRelation, ServiceSupervisorObservation,
    build_evidence_envelope,
};

use super::decision::decide_process_convergence;
use super::types::{
    PROCESS_CONVERGENCE_RECORD_SCHEMA, ProcessConvergenceRecord, ProcessConvergenceRequest,
};
use crate::ports::evidence_store::ContentAddressedEvidenceStorePort;

pub fn record_process_convergence_epoch(
    request: &ProcessConvergenceRequest,
    observed: &ServiceSupervisorObservation,
    read_back: Option<&ServiceSupervisorObservation>,
    observer_id: &str,
    store: &dyn ContentAddressedEvidenceStorePort,
) -> Result<ProcessConvergenceRecord, String> {
    if observer_id.trim().is_empty() {
        return Err("process_convergence_observer_id_missing".to_string());
    }
    let desired = request.desired_unit()?;
    let decision = decide_process_convergence(request, observed, read_back)?;

    let desired_evidence = build_evidence_envelope(
        &request.subject_id,
        "cynic-host:registry-service-policy",
        observed.observed_at_unix_ms,
        desired_provenance(&desired.source),
        desired,
    )?;
    persist_exact(store, &desired_evidence)?;
    let desired_ref = evidence_ref("desired_process_state", &desired_evidence);

    let observed_evidence = build_evidence_envelope(
        &request.subject_id,
        observer_id,
        observed.observed_at_unix_ms,
        observation_provenance(observed),
        observed,
    )?;
    persist_exact(store, &observed_evidence)?;
    let observed_ref = evidence_ref("process_observation", &observed_evidence);

    let decision_evidence = build_evidence_envelope(
        &request.subject_id,
        "cynic-host:process-convergence-decision",
        read_back
            .map(|report| report.observed_at_unix_ms)
            .unwrap_or(observed.observed_at_unix_ms),
        vec![
            desired_evidence.digest.clone(),
            observed_evidence.digest.clone(),
        ],
        &decision,
    )?;
    persist_exact(store, &decision_evidence)?;
    let decision_ref = evidence_ref("process_convergence_decision", &decision_evidence);

    let read_back_evidence = read_back
        .map(|report| {
            let envelope = build_evidence_envelope(
                &request.subject_id,
                observer_id,
                report.observed_at_unix_ms,
                observation_provenance(report),
                report,
            )?;
            persist_exact(store, &envelope)?;
            Ok::<_, String>(envelope)
        })
        .transpose()?;
    let read_back_ref = read_back_evidence
        .as_ref()
        .map(|envelope| evidence_ref("process_read_back", envelope));

    let epoch = assess_causal_convergence_epoch(CausalConvergenceEpochInput {
        epoch_id: request.epoch_id.clone(),
        observed_at_unix_ms: read_back
            .map(|report| report.observed_at_unix_ms)
            .unwrap_or(observed.observed_at_unix_ms),
        subject_id: request.subject_id.clone(),
        vertical_id: "process".to_string(),
        requested_status: decision.requested_status,
        causal_evidence: CausalEvidenceRefs {
            desired_state: desired_ref,
            observations: vec![observed_ref],
            decision: decision_ref,
            authorization: None,
            action_receipt: None,
            read_back: read_back_ref,
        },
        trust_requirements: process_trust_requirements(),
        trust: process_trust(request, observed, read_back),
        residual_unknowns: decision.residual_unknowns.clone(),
        blockers: decision.blockers.clone(),
    });

    let epoch_evidence = build_evidence_envelope(
        &request.subject_id,
        "cynic-host:process-convergence-recorder",
        epoch.observed_at_unix_ms,
        epoch.cell.evidence_refs.clone(),
        &epoch,
    )?;
    persist_exact(store, &epoch_evidence)?;

    let mut persisted_evidence_digests = vec![
        desired_evidence.digest,
        observed_evidence.digest,
        decision_evidence.digest,
    ];
    if let Some(envelope) = read_back_evidence {
        persisted_evidence_digests.push(envelope.digest);
    }
    persisted_evidence_digests.push(epoch_evidence.digest.clone());

    Ok(ProcessConvergenceRecord {
        schema: PROCESS_CONVERGENCE_RECORD_SCHEMA.to_string(),
        epoch,
        epoch_evidence,
        persisted_evidence_digests,
    })
}

fn persist_exact(
    store: &dyn ContentAddressedEvidenceStorePort,
    envelope: &EvidenceEnvelope,
) -> Result<(), String> {
    store.persist(envelope)?;
    match store.read(&envelope.digest)? {
        Some(read_back) if read_back == *envelope => Ok(()),
        Some(_) => Err("process_convergence_evidence_read_back_mismatch".to_string()),
        None => Err("process_convergence_evidence_read_back_missing".to_string()),
    }
}

fn evidence_ref(kind: &str, envelope: &EvidenceEnvelope) -> EvidenceRef {
    EvidenceRef {
        kind: kind.to_string(),
        subject: envelope.subject.clone(),
        digest: envelope.digest.clone(),
        lineage: envelope.lineage.clone(),
    }
}

fn desired_provenance(source: &str) -> Vec<String> {
    if source.trim().is_empty() {
        vec!["unknown:desired_process_source".to_string()]
    } else {
        vec![source.to_string()]
    }
}

fn observation_provenance(report: &ServiceSupervisorObservation) -> Vec<String> {
    let mut provenance = [
        report.target_source.as_str(),
        report.supervisor_kind.as_str(),
        report.supervisor_scope.as_str(),
    ]
    .into_iter()
    .filter(|value| !value.trim().is_empty())
    .map(str::to_string)
    .collect::<Vec<_>>();
    provenance.sort();
    provenance.dedup();
    if provenance.is_empty() {
        provenance.push("unknown:process_observation_provenance".to_string());
    }
    provenance
}

fn process_trust_requirements() -> TrustRequirements {
    TrustRequirements {
        authority: true,
        authenticity: false,
        integrity: true,
        freshness: true,
        scope: true,
        replay_safety: false,
        read_back: true,
        independence: false,
    }
}

fn process_trust(
    request: &ProcessConvergenceRequest,
    observed: &ServiceSupervisorObservation,
    read_back: Option<&ServiceSupervisorObservation>,
) -> TrustAssessment {
    let authority = request
        .desired_unit()
        .map_or(TrustDimensionStatus::Blocked, |desired| {
            if desired.source.starts_with("registry:") {
                TrustDimensionStatus::Satisfied
            } else {
                TrustDimensionStatus::Unknown
            }
        });
    let scope = if observed.subject == request.subject_id
        && read_back.is_none_or(|report| report.subject == request.subject_id)
    {
        TrustDimensionStatus::Satisfied
    } else {
        TrustDimensionStatus::Blocked
    };
    let freshness = match read_back {
        Some(report)
            if observed.observed_at_unix_ms > 0
                && report.observed_at_unix_ms >= observed.observed_at_unix_ms =>
        {
            TrustDimensionStatus::Satisfied
        }
        Some(_) => TrustDimensionStatus::Blocked,
        None => TrustDimensionStatus::Unknown,
    };

    TrustAssessment {
        authority,
        authenticity: TrustDimensionStatus::Unknown,
        integrity: TrustDimensionStatus::Satisfied,
        freshness,
        scope,
        replay_safety: TrustDimensionStatus::Unknown,
        read_back: if read_back.is_some() {
            TrustDimensionStatus::Satisfied
        } else {
            TrustDimensionStatus::Unknown
        },
        independence: EvidenceRelation::Unknown {
            missing_evidence: vec!["independent_process_observer".to_string()],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::local_evidence_store::LocalContentAddressedEvidenceStore;
    use crate::domain::services::{ServiceManagerKind, ServicePolicy, ServiceUnitPolicy};
    use cynic_protocol::{
        ConvergenceStatus, SERVICE_SUPERVISOR_OBSERVATION_EVIDENCE_ID,
        SERVICE_SUPERVISOR_OBSERVATION_SCHEMA, ServiceSupervisorUnitObservation,
    };

    fn request() -> ProcessConvergenceRequest {
        ProcessConvergenceRequest {
            epoch_id: "epoch-local-kernel-1".to_string(),
            subject_id: "local-kernel/process".to_string(),
            service_id: "cynic-kernel".to_string(),
            policy: ServicePolicy {
                manager: ServiceManagerKind::Systemd,
                unit_dir: "/etc/systemd/system".to_string(),
                units: vec![ServiceUnitPolicy {
                    service_id: "cynic-kernel".to_string(),
                    unit_name: "cynic-kernel.service".to_string(),
                    enable_on_boot: true,
                    restart_mode: "on-failure".to_string(),
                    source: "registry:infra/registry.json".to_string(),
                    health_port: Some(3030),
                    process_name: Some("cynic-kernel".to_string()),
                    supervisor_provider: Some("process-port".to_string()),
                    health_kind: Some("tcp".to_string()),
                }],
            },
        }
    }

    fn observation(observed_at_unix_ms: u128) -> ServiceSupervisorObservation {
        ServiceSupervisorObservation {
            schema: SERVICE_SUPERVISOR_OBSERVATION_SCHEMA,
            evidence_id: SERVICE_SUPERVISOR_OBSERVATION_EVIDENCE_ID,
            status: "active",
            subject: "local-kernel/process".to_string(),
            supervisor_kind: "process_port".to_string(),
            supervisor_scope: "local".to_string(),
            target_source: "registry:infra/registry.json".to_string(),
            observed_at_unix_ms,
            service_count: 1,
            active_count: 1,
            degraded_count: 0,
            unknown_count: 0,
            services: vec![ServiceSupervisorUnitObservation {
                service_id: "cynic-kernel".to_string(),
                unit_name: "cynic-kernel.service".to_string(),
                health_port: Some(3030),
                health_kind: Some("tcp".to_string()),
                desired_provider: Some("process-port".to_string()),
                selected_provider: "process_port".to_string(),
                selected_scope: "local".to_string(),
                target_source: "registry:infra/registry.json".to_string(),
                desired_state: "enabled_and_active".to_string(),
                restart_policy: "on-failure".to_string(),
                probe_status: "ok",
                probe_detail: "tcp probe".to_string(),
                provider_attempts: Vec::new(),
                load_state: "unknown".to_string(),
                unit_file_state: "unknown".to_string(),
                active_state: "process_running".to_string(),
                sub_state: "unknown".to_string(),
                restart_count: None,
                active_enter_timestamp_monotonic: None,
                status: "active",
            }],
        }
    }

    #[test]
    fn complete_epoch_is_persisted_only_after_exact_phase_read_back() {
        let root = tempfile::tempdir().unwrap();
        let store = LocalContentAddressedEvidenceStore::new(root.path());
        let before = observation(1);
        let after = observation(2);

        let record = record_process_convergence_epoch(
            &request(),
            &before,
            Some(&after),
            "test:process-observer",
            &store,
        )
        .unwrap();

        assert_eq!(record.epoch.cell.status, ConvergenceStatus::Complete);
        assert_eq!(record.epoch.cell.evidence_refs.len(), 4);
        assert_eq!(record.persisted_evidence_digests.len(), 5);
        assert_eq!(
            store.read(&record.epoch_evidence.digest).unwrap(),
            Some(record.epoch_evidence)
        );
    }

    #[test]
    fn absent_read_back_is_retained_as_unknown_epoch() {
        let root = tempfile::tempdir().unwrap();
        let store = LocalContentAddressedEvidenceStore::new(root.path());
        let before = observation(1);

        let record = record_process_convergence_epoch(
            &request(),
            &before,
            None,
            "test:process-observer",
            &store,
        )
        .unwrap();

        assert_eq!(record.epoch.cell.status, ConvergenceStatus::Unknown);
        assert!(
            record
                .epoch
                .residual_unknowns
                .contains(&"process_read_back_missing".to_string())
        );
        assert!(
            record
                .epoch
                .residual_unknowns
                .contains(&"trust_read_back_unknown".to_string())
        );
    }
}
