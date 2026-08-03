use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::{
    RUNTIME_PROOF_BUNDLE_SCHEMA, RUNTIME_PROOF_SCHEMA, RuntimeProof, RuntimeProofBundle,
    RuntimeProofEvidence,
};

use crate::build::BuildProof;
use crate::domain::shared::SurfaceStatus;
use crate::usecases::HostObservationReports;
use crate::vertical_conformance::VerticalConformanceReport;

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

pub fn assemble_runtime_proof_bundle(
    identity: RuntimeProof,
    verticals: &VerticalConformanceReport,
) -> RuntimeProofBundle {
    let vertical_status = match verticals.status {
        "complete" => "satisfied",
        "partial" => "partial",
        "blocked" => "blocked",
        _ => "unknown",
    };
    let vertical_proof = RuntimeProof {
        schema: RUNTIME_PROOF_SCHEMA.to_string(),
        gate: "vertical_conformance".to_string(),
        status: vertical_status.to_string(),
        producer: "cynic-host".to_string(),
        subject: identity.subject.clone(),
        observed_at_unix_ms: observed_at_unix_ms(),
        evidence: verticals
            .verticals
            .iter()
            .map(|vertical| RuntimeProofEvidence {
                id: vertical.id.clone(),
                status: vertical.status.to_string(),
                detail: if vertical.blockers.is_empty() {
                    format!("coverage={}", vertical.declared_coverage)
                } else {
                    format!("blockers={}", vertical.blockers.join(","))
                },
            })
            .collect(),
    };
    let status = if identity.status != "satisfied" || vertical_status != "satisfied" {
        if identity.status == "blocked" || vertical_status == "blocked" {
            "blocked"
        } else if identity.status == "unknown" || vertical_status == "unknown" {
            "unknown"
        } else {
            "partial"
        }
    } else {
        "satisfied"
    };
    RuntimeProofBundle {
        schema: RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
        status: status.to_string(),
        proofs: vec![identity, vertical_proof],
    }
}

pub fn assemble_runtime_proof_persistence_record(
    bundle: &RuntimeProofBundle,
    verticals: &VerticalConformanceReport,
    build_proof: Option<&BuildProof>,
) -> cynic_protocol::RuntimeProofPersistenceRecord {
    let node = bundle
        .proofs
        .first()
        .map(|proof| proof.subject.clone())
        .unwrap_or_else(|| "unknown-node".to_string());
    let revision = build_proof
        .map(|proof| proof.source_commit.clone())
        .filter(|revision| !revision.is_empty())
        .unwrap_or_else(|| "observation-only".to_string());
    let evidence_refs = verticals
        .verticals
        .iter()
        .flat_map(|vertical| vertical.evidence_refs.iter().cloned())
        .chain(build_proof.into_iter().map(|_| "build_proof".to_string()))
        .collect();
    cynic_protocol::runtime_proof_persistence_record(
        node,
        revision,
        observed_at_unix_ms(),
        bundle.status.clone(),
        verticals.residual_unknowns.clone(),
        verticals.blockers.clone(),
        evidence_refs,
        bundle.clone(),
    )
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
    use crate::domain::shared::{BoundaryKind, LifecycleDisposition, LifecycleStatus};

    #[test]
    fn host_identity_proof_is_satisfied_only_when_all_host_surfaces_are_active() {
        let observed = HostObservationReports {
            schema: crate::usecases::HOST_OBSERVATION_SCHEMA,
            host_resolution: None,
            status: SurfaceStatus::Blocked,
            lifecycle_status: LifecycleStatus::Missing,
            lifecycle_disposition: LifecycleDisposition::NotDiscovered,
            next_boundary_kind: BoundaryKind::Adopt,
            next_boundary_reason:
                "one or more declared service units are missing from observation".to_string(),
            next_command: Some("cynic host network-baseline plan".to_string()),
            blocked_domains: vec!["services"],
            principal: PrincipalStatusReport {
                status: SurfaceStatus::Active,
                lifecycle_status: LifecycleStatus::Ready,
                lifecycle_disposition: LifecycleDisposition::ObservableCandidate,
                next_boundary_kind: BoundaryKind::Observe,
                next_boundary_reason:
                    "runtime principal is present, aligned, and observable".to_string(),
                next_command: None,
                user_present: true,
                group_present: true,
                home_matches: true,
                missing_state_dirs: Vec::new(),
                misowned_paths: Vec::new(),
                ownership_unknown_paths: Vec::new(),
                evidences: Vec::new(),
            },
            privilege: PrivilegeStatusReport {
                status: SurfaceStatus::Active,
                lifecycle_status: LifecycleStatus::Ready,
                lifecycle_disposition: LifecycleDisposition::ObservableCandidate,
                next_boundary_kind: BoundaryKind::Observe,
                next_boundary_reason:
                    "privilege boundary is present, denial proof is satisfied, and the surface is observable"
                        .to_string(),
                next_command: None,
                helper_dir_present: true,
                sudoers_projection_present: true,
                denial_proof_ready: true,
                evidences: Vec::new(),
            },
            packages: PackageStatusReport {
                status: SurfaceStatus::Active,
                lifecycle_status: LifecycleStatus::Ready,
                lifecycle_disposition: LifecycleDisposition::ObservableCandidate,
                next_boundary_kind: BoundaryKind::Observe,
                next_boundary_reason:
                    "approved package sources are declared and bootstrap packages are present"
                        .to_string(),
                next_command: None,
                approved_sources_declared: true,
                installed_packages: Vec::new(),
                missing_packages: Vec::new(),
                evidences: Vec::new(),
            },
            services: ServiceStatusReport {
                status: SurfaceStatus::Blocked,
                lifecycle_status: LifecycleStatus::Missing,
                lifecycle_disposition: LifecycleDisposition::NotDiscovered,
                next_boundary_kind: BoundaryKind::Adopt,
                next_boundary_reason:
                    "one or more declared service units are missing from observation".to_string(),
                next_command: Some("cynic host network-baseline plan".to_string()),
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
