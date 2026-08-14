use serde::{Deserialize, Serialize};

use super::enrollment::FrameworkEnrollmentRequestReport;

pub trait FrameworkEnrollmentAuthorityPort {
    fn verify_grant(&self, grant: &FrameworkEnrollmentGrant) -> FrameworkEnrollmentVerification;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameworkEnrollmentGrant {
    pub schema: String,
    pub grant_id: String,
    pub authority_id: String,
    pub request_id: String,
    pub seed_id: String,
    pub machine_id: String,
    pub requested_role: String,
    pub proof_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameworkEnrollmentVerification {
    pub trusted: bool,
    pub proof_id: Option<String>,
    pub blocker: Option<String>,
}

pub struct UnavailableFrameworkEnrollmentAuthority;

impl FrameworkEnrollmentAuthorityPort for UnavailableFrameworkEnrollmentAuthority {
    fn verify_grant(&self, _grant: &FrameworkEnrollmentGrant) -> FrameworkEnrollmentVerification {
        FrameworkEnrollmentVerification {
            trusted: false,
            proof_id: None,
            blocker: Some("external_enrollment_authority_unavailable".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkEnrollmentAdmissionReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub request_id: String,
    pub grant_id: String,
    pub authority_id: String,
    pub requested_role: String,
    pub proof_id: Option<String>,
    pub activation_permitted: bool,
    pub blockers: Vec<String>,
    pub rollback_intent: &'static str,
}

pub fn admit_framework_enrollment(
    request: &FrameworkEnrollmentRequestReport,
    grant: &FrameworkEnrollmentGrant,
    authority: &dyn FrameworkEnrollmentAuthorityPort,
) -> FrameworkEnrollmentAdmissionReport {
    let mut blockers = grant_binding_blockers(request, grant);
    let verification = authority.verify_grant(grant);
    if !verification.trusted {
        blockers.push(
            verification
                .blocker
                .unwrap_or_else(|| "external_enrollment_grant_untrusted".to_string()),
        );
    }
    blockers.sort();
    blockers.dedup();
    let activation_permitted = blockers.is_empty();

    FrameworkEnrollmentAdmissionReport {
        schema: "cynic.framework_enrollment.admission.v1",
        status: if activation_permitted {
            "admitted"
        } else {
            "blocked"
        },
        mutates: false,
        request_id: request.request_id.clone(),
        grant_id: grant.grant_id.clone(),
        authority_id: grant.authority_id.clone(),
        requested_role: request.requested_role.to_string(),
        proof_id: verification.proof_id,
        activation_permitted,
        blockers,
        rollback_intent: "invalidate the grant and keep the target role inactive",
    }
}

fn grant_binding_blockers(
    request: &FrameworkEnrollmentRequestReport,
    grant: &FrameworkEnrollmentGrant,
) -> Vec<String> {
    let mut blockers = Vec::new();
    check(
        grant.schema == "cynic.framework_enrollment.grant.v1",
        "grant_schema_invalid",
        &mut blockers,
    );
    check(
        !grant.grant_id.trim().is_empty(),
        "grant_id_missing",
        &mut blockers,
    );
    check(
        !grant.authority_id.trim().is_empty(),
        "grant_authority_missing",
        &mut blockers,
    );
    check(
        !grant.proof_reference.trim().is_empty(),
        "grant_proof_reference_missing",
        &mut blockers,
    );
    check(
        grant.request_id == request.request_id,
        "grant_request_binding_mismatch",
        &mut blockers,
    );
    check(
        grant.seed_id == request.seed_id,
        "grant_seed_binding_mismatch",
        &mut blockers,
    );
    check(
        grant.machine_id == request.machine_id,
        "grant_machine_binding_mismatch",
        &mut blockers,
    );
    check(
        grant.requested_role == request.requested_role,
        "grant_role_binding_mismatch",
        &mut blockers,
    );
    blockers
}

fn check(condition: bool, blocker: &str, blockers: &mut Vec<String>) {
    if !condition {
        blockers.push(blocker.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TrustedAuthority;

    impl FrameworkEnrollmentAuthorityPort for TrustedAuthority {
        fn verify_grant(
            &self,
            _grant: &FrameworkEnrollmentGrant,
        ) -> FrameworkEnrollmentVerification {
            FrameworkEnrollmentVerification {
                trusted: true,
                proof_id: Some("authority-proof-42".to_string()),
                blocker: None,
            }
        }
    }

    fn request() -> FrameworkEnrollmentRequestReport {
        FrameworkEnrollmentRequestReport {
            schema: "cynic.framework_enrollment.request.v1",
            status: "pending_external_admission",
            mutates: false,
            request_id: "request-42".to_string(),
            seed_id: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            machine_id: "pve-vm-103".to_string(),
            requested_role: "kernel",
            identity_class: "MachineEnrollment",
            admission_status: "external_authority_required",
            required_proofs: Vec::new(),
            blockers: vec!["external_enrollment_authority_unavailable"],
            rollback_intent: "discard",
        }
    }

    fn grant() -> FrameworkEnrollmentGrant {
        FrameworkEnrollmentGrant {
            schema: "cynic.framework_enrollment.grant.v1".to_string(),
            grant_id: "grant-42".to_string(),
            authority_id: "cynic-runtime-authority".to_string(),
            request_id: "request-42".to_string(),
            seed_id: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            machine_id: "pve-vm-103".to_string(),
            requested_role: "kernel".to_string(),
            proof_reference: "sha256:proof".to_string(),
        }
    }

    #[test]
    fn trusted_grant_bound_to_request_is_admitted_without_mutation() {
        let report = admit_framework_enrollment(&request(), &grant(), &TrustedAuthority);

        assert_eq!(report.status, "admitted");
        assert!(report.activation_permitted);
        assert!(!report.mutates);
        assert_eq!(report.proof_id.as_deref(), Some("authority-proof-42"));
    }

    #[test]
    fn grant_for_another_machine_is_rejected_even_when_authority_trusts_it() {
        let mut grant = grant();
        grant.machine_id = "pve-vm-104".to_string();
        let report = admit_framework_enrollment(&request(), &grant, &TrustedAuthority);

        assert_eq!(report.status, "blocked");
        assert!(!report.activation_permitted);
        assert!(
            report
                .blockers
                .contains(&"grant_machine_binding_mismatch".to_string())
        );
    }

    #[test]
    fn unavailable_authority_never_admits_a_locally_well_formed_grant() {
        let report = admit_framework_enrollment(
            &request(),
            &grant(),
            &UnavailableFrameworkEnrollmentAuthority,
        );

        assert_eq!(report.status, "blocked");
        assert!(
            report
                .blockers
                .contains(&"external_enrollment_authority_unavailable".to_string())
        );
    }
}
