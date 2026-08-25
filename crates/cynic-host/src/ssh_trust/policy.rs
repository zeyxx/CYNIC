use crate::transport::SshTargetInspectionReport;

use super::types::{
    SSH_TRUST_ADMISSION_SCHEMA, SSH_TRUST_VERIFICATION_MAX_AGE_MS, SSH_TRUST_VERIFICATION_SCHEMA,
    SshTrustAdmissionReport, SshTrustAnchor, SshTrustStore, SshTrustVerification,
};

pub fn verify_ssh_target_trust(
    inspection: &SshTargetInspectionReport,
    store: &SshTrustStore,
) -> SshTrustVerification {
    verify_ssh_target_trust_at(inspection, store, now_unix_ms())
}

/// Pure trust verification at an explicit time. A matching key is not trusted
/// for route execution after its observation expires.
pub fn verify_ssh_target_trust_at(
    inspection: &SshTargetInspectionReport,
    store: &SshTrustStore,
    now_unix_ms: u64,
) -> SshTrustVerification {
    let observed_fingerprints = inspection
        .host_keys
        .iter()
        .map(|key| format!("{}:{}", key.algorithm, key.fingerprint))
        .collect::<Vec<_>>();
    let anchor = store
        .anchors
        .iter()
        .find(|anchor| anchor.target == inspection.target)
        .cloned();
    let fresh = inspection.observed_at_unix_ms > 0
        && now_unix_ms
            .checked_sub(inspection.observed_at_unix_ms)
            .is_some_and(|age| age <= SSH_TRUST_VERIFICATION_MAX_AGE_MS);
    let (status, residual_risks) = match (&anchor, inspection.status.as_str()) {
        (_, status) if status != "observed" => (
            "not_observed".to_string(),
            vec!["SSH host key observation is unavailable".to_string()],
        ),
        (None, _) => (
            "untrusted_observed".to_string(),
            vec!["no independently admitted SSH host-key anchor".to_string()],
        ),
        (Some(anchor), _)
            if inspection.host_keys.iter().any(|key| {
                key.algorithm == anchor.algorithm && key.fingerprint == anchor.fingerprint
            }) && fresh =>
        {
            ("trusted_match".to_string(), Vec::new())
        }
        (Some(anchor), _)
            if inspection.host_keys.iter().any(|key| {
                key.algorithm == anchor.algorithm && key.fingerprint == anchor.fingerprint
            }) =>
        {
            (
                "trust_stale".to_string(),
                vec![
                    "SSH host-key observation exceeded the route trust freshness window"
                        .to_string(),
                ],
            )
        }
        (Some(_), _) => (
            "trust_refuted".to_string(),
            vec!["observed SSH host keys do not match the admitted anchor".to_string()],
        ),
    };
    SshTrustVerification {
        schema: SSH_TRUST_VERIFICATION_SCHEMA,
        target: inspection.target.clone(),
        status,
        anchor,
        observed_fingerprints,
        observed_at_unix_ms: inspection.observed_at_unix_ms,
        max_age_ms: SSH_TRUST_VERIFICATION_MAX_AGE_MS,
        expires_at_unix_ms: inspection
            .observed_at_unix_ms
            .saturating_add(SSH_TRUST_VERIFICATION_MAX_AGE_MS),
        residual_risks,
    }
}

pub fn project_trust_verification(
    inspection: &mut SshTargetInspectionReport,
    verification: &SshTrustVerification,
) {
    inspection.trust_state = verification.status.clone();
    inspection.next_action = match verification.status.as_str() {
        "trusted_match" => "admitted SSH host-key anchor matches current observation".to_string(),
        "trust_stale" => {
            "stop execution; repeat SSH host-key observation before using this route".to_string()
        }
        "trust_refuted" => {
            "stop execution; re-verify host identity through an independent operator channel"
                .to_string()
        }
        "untrusted_observed" => {
            "admit an independently verified SSH host-key fingerprint before execution".to_string()
        }
        _ => inspection.next_action.clone(),
    };
}

pub fn plan_ssh_trust_admission(
    inspection: &SshTargetInspectionReport,
    algorithm: &str,
    fingerprint: &str,
    attestation_source: &str,
    requested_apply: bool,
) -> SshTrustAdmissionReport {
    let mut blockers = Vec::new();
    if inspection.status != "observed" {
        blockers.push("ssh_host_key_not_observed".to_string());
    }
    if algorithm.trim().is_empty() || fingerprint.trim().is_empty() {
        blockers.push("ssh_trust_anchor_missing".to_string());
    }
    if algorithm != "ED25519" {
        blockers.push("ssh_trust_anchor_algorithm_not_ed25519".to_string());
    }
    if attestation_source.trim().is_empty() {
        blockers.push("independent_attestation_source_required".to_string());
    }
    if !inspection
        .host_keys
        .iter()
        .any(|key| key.algorithm == algorithm && key.fingerprint == fingerprint)
    {
        blockers.push("attested_fingerprint_not_in_current_observation".to_string());
    }
    if !requested_apply {
        blockers.push("explicit_yes_required".to_string());
    }
    blockers.sort();
    blockers.dedup();
    let anchor = blockers.is_empty().then(|| SshTrustAnchor {
        target: inspection.target.clone(),
        algorithm: algorithm.to_string(),
        fingerprint: fingerprint.to_string(),
        attestation_source: attestation_source.to_string(),
        admitted_at_unix_ms: now_unix_ms(),
    });
    let verification = anchor.as_ref().map_or_else(
        || verify_ssh_target_trust(inspection, &SshTrustStore::empty()),
        |anchor| {
            verify_ssh_target_trust(
                inspection,
                &SshTrustStore {
                    schema: super::types::SSH_TRUST_STORE_SCHEMA.to_string(),
                    anchors: vec![anchor.clone()],
                },
            )
        },
    );
    SshTrustAdmissionReport {
        schema: SSH_TRUST_ADMISSION_SCHEMA,
        target: inspection.target.clone(),
        status: if blockers.is_empty() {
            "planned".to_string()
        } else {
            "blocked".to_string()
        },
        mutates: false,
        requested_apply,
        verification,
        anchor,
        blockers,
    }
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use crate::transport::{
        ExecutableObservation, SshHostKeyObservation, SshTargetInspectionReport,
    };

    use super::*;

    fn inspection(fingerprint: &str) -> SshTargetInspectionReport {
        SshTargetInspectionReport {
            schema: "cynic.ssh_target_inspection.v1",
            target: "10.10.10.1".to_string(),
            status: "observed".to_string(),
            observed_at_unix_ms: now_unix_ms(),
            ssh: ExecutableObservation {
                binary: "ssh".to_string(),
                status: "available".to_string(),
                path: None,
            },
            ssh_keyscan: ExecutableObservation {
                binary: "ssh-keyscan".to_string(),
                status: "available".to_string(),
                path: None,
            },
            host_keys: vec![SshHostKeyObservation {
                algorithm: "ED25519".to_string(),
                fingerprint: fingerprint.to_string(),
            }],
            trust_state: "untrusted_observed".to_string(),
            mutation: false,
            next_action: "compare".to_string(),
            detail: None,
        }
    }

    #[test]
    fn admitted_anchor_matches_only_exact_observation() {
        let anchor = SshTrustAnchor {
            target: "10.10.10.1".to_string(),
            algorithm: "ED25519".to_string(),
            fingerprint: "SHA256:expected".to_string(),
            attestation_source: "console".to_string(),
            admitted_at_unix_ms: 1,
        };
        let store = SshTrustStore {
            schema: super::super::types::SSH_TRUST_STORE_SCHEMA.to_string(),
            anchors: vec![anchor],
        };
        assert_eq!(
            verify_ssh_target_trust(&inspection("SHA256:expected"), &store).status,
            "trusted_match"
        );
        assert_eq!(
            verify_ssh_target_trust(&inspection("SHA256:rotated"), &store).status,
            "trust_refuted"
        );
    }

    #[test]
    fn admission_requires_exact_current_fingerprint_and_explicit_yes() {
        let report = plan_ssh_trust_admission(
            &inspection("SHA256:expected"),
            "ED25519",
            "SHA256:expected",
            "console",
            false,
        );
        assert_eq!(report.status, "blocked");
        assert!(
            report
                .blockers
                .contains(&"explicit_yes_required".to_string())
        );
        let report = plan_ssh_trust_admission(
            &inspection("SHA256:expected"),
            "ED25519",
            "SHA256:other",
            "console",
            true,
        );
        assert!(
            report
                .blockers
                .contains(&"attested_fingerprint_not_in_current_observation".to_string())
        );
    }

    #[test]
    fn admission_refuses_non_ed25519_host_identity() {
        let report = plan_ssh_trust_admission(
            &inspection("SHA256:expected"),
            "RSA",
            "SHA256:expected",
            "console",
            true,
        );
        assert!(
            report
                .blockers
                .contains(&"ssh_trust_anchor_algorithm_not_ed25519".to_string())
        );
    }

    #[test]
    fn exact_anchor_match_becomes_stale_after_the_freshness_window() {
        let anchor = SshTrustAnchor {
            target: "10.10.10.1".to_string(),
            algorithm: "ED25519".to_string(),
            fingerprint: "SHA256:expected".to_string(),
            attestation_source: "console".to_string(),
            admitted_at_unix_ms: 1,
        };
        let store = SshTrustStore {
            schema: super::super::types::SSH_TRUST_STORE_SCHEMA.to_string(),
            anchors: vec![anchor],
        };
        let mut report = inspection("SHA256:expected");
        report.observed_at_unix_ms = 1;

        let verification =
            verify_ssh_target_trust_at(&report, &store, 1 + SSH_TRUST_VERIFICATION_MAX_AGE_MS + 1);

        assert_eq!(verification.status, "trust_stale");
        assert_eq!(verification.max_age_ms, SSH_TRUST_VERIFICATION_MAX_AGE_MS);
    }
}
