use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::{
    RUNTIME_PROOF_BUNDLE_SCHEMA, RUNTIME_PROOF_SCHEMA, RuntimeProof, RuntimeProofBundle,
    RuntimeProofEvidence,
};

use crate::domain::shared::SurfaceStatus;
use crate::usecases::HostObservationReports;

pub fn host_identity_runtime_proof(observed: &HostObservationReports) -> RuntimeProof {
    let evidence = vec![
        RuntimeProofEvidence {
            id: "principal".to_string(),
            status: surface_status(&observed.principal.status).to_string(),
            detail: format!(
                "user_present={} group_present={} home_matches={} missing_state_dirs={}",
                observed.principal.user_present,
                observed.principal.group_present,
                observed.principal.home_matches,
                observed.principal.missing_state_dirs.join(",")
            ),
        },
        RuntimeProofEvidence {
            id: "privilege".to_string(),
            status: surface_status(&observed.privilege.status).to_string(),
            detail: format!(
                "helper_dir_present={} sudoers_projection_present={} denial_proof_ready={}",
                observed.privilege.helper_dir_present,
                observed.privilege.sudoers_projection_present,
                observed.privilege.denial_proof_ready
            ),
        },
        RuntimeProofEvidence {
            id: "packages".to_string(),
            status: surface_status(&observed.packages.status).to_string(),
            detail: format!(
                "approved_sources_declared={} installed_packages={} missing_packages={}",
                observed.packages.approved_sources_declared,
                observed.packages.installed_packages.len(),
                observed.packages.missing_packages.join(",")
            ),
        },
        RuntimeProofEvidence {
            id: "services".to_string(),
            status: surface_status(&observed.services.status).to_string(),
            detail: format!(
                "missing_units={} disabled_units={} inactive_units={}",
                observed.services.missing_units.len(),
                observed.services.disabled_units.len(),
                observed.services.inactive_units.len()
            ),
        },
    ];

    RuntimeProof {
        schema: RUNTIME_PROOF_SCHEMA.to_string(),
        gate: "host_identity_observed".to_string(),
        status: if host_identity_satisfied(observed) {
            "satisfied"
        } else {
            "blocked"
        }
        .to_string(),
        producer: "cynic-host".to_string(),
        subject: proof_subject(observed),
        observed_at_unix_ms: observed_at_unix_ms(),
        evidence,
    }
}

pub fn host_identity_runtime_proof_bundle(observed: &HostObservationReports) -> RuntimeProofBundle {
    let proof = host_identity_runtime_proof(observed);
    RuntimeProofBundle {
        schema: RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
        status: proof.status.clone(),
        proofs: vec![proof],
    }
}

fn host_identity_satisfied(observed: &HostObservationReports) -> bool {
    observed.principal.status == SurfaceStatus::Active
        && observed.privilege.status == SurfaceStatus::Active
        && observed.packages.status == SurfaceStatus::Active
        && observed.services.status == SurfaceStatus::Active
}

fn proof_subject(observed: &HostObservationReports) -> String {
    observed
        .host_resolution
        .as_ref()
        .and_then(|resolution| resolution.matched_node.clone())
        .unwrap_or_else(|| "current_host".to_string())
}

fn surface_status(status: &SurfaceStatus) -> &'static str {
    match status {
        SurfaceStatus::Planned => "planned",
        SurfaceStatus::Active => "active",
        SurfaceStatus::Degraded => "degraded",
        SurfaceStatus::Blocked => "blocked",
    }
}

fn observed_at_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::packages::PackageStatusReport;
    use crate::domain::principal::PrincipalStatusReport;
    use crate::domain::privilege::PrivilegeStatusReport;
    use crate::domain::services::ServiceStatusReport;

    #[test]
    fn host_identity_proof_is_satisfied_only_when_all_host_surfaces_are_active() {
        let observed = HostObservationReports {
            schema: crate::usecases::HOST_OBSERVATION_SCHEMA,
            host_resolution: None,
            status: SurfaceStatus::Blocked,
            blocked_domains: vec!["services"],
            principal: PrincipalStatusReport {
                status: SurfaceStatus::Active,
                user_present: true,
                group_present: true,
                home_matches: true,
                missing_state_dirs: Vec::new(),
                evidences: Vec::new(),
            },
            privilege: PrivilegeStatusReport {
                status: SurfaceStatus::Active,
                helper_dir_present: true,
                sudoers_projection_present: true,
                denial_proof_ready: true,
                evidences: Vec::new(),
            },
            packages: PackageStatusReport {
                status: SurfaceStatus::Active,
                approved_sources_declared: true,
                installed_packages: Vec::new(),
                missing_packages: Vec::new(),
                evidences: Vec::new(),
            },
            services: ServiceStatusReport {
                status: SurfaceStatus::Blocked,
                missing_units: vec!["cynic-node.service".to_string()],
                disabled_units: Vec::new(),
                inactive_units: Vec::new(),
                evidences: Vec::new(),
            },
        };

        let proof = host_identity_runtime_proof(&observed);
        assert_eq!(proof.schema, RUNTIME_PROOF_SCHEMA);
        assert_eq!(proof.gate, "host_identity_observed");
        assert_eq!(proof.status, "blocked");
        assert_eq!(proof.evidence.len(), 4);
    }
}
