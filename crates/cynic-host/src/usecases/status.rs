use crate::domain::shared::{BoundaryKind, LifecycleDisposition, LifecycleStatus, SurfaceStatus};

use super::HostObservationReports;

pub(super) fn overall_status(observed: &HostObservationReports) -> SurfaceStatus {
    observed.status
}

pub(super) fn blocked_domains(observed: &HostObservationReports) -> Vec<&'static str> {
    observed.blocked_domains.clone()
}

pub(super) fn blocked_domain_statuses(
    principal: &SurfaceStatus,
    privilege: &SurfaceStatus,
    packages: &SurfaceStatus,
    services: &SurfaceStatus,
) -> Vec<&'static str> {
    let mut blocked = Vec::new();
    if *principal != SurfaceStatus::Active {
        blocked.push("principal");
    }
    if *privilege != SurfaceStatus::Active {
        blocked.push("privilege");
    }
    if *packages != SurfaceStatus::Active {
        blocked.push("packages");
    }
    if *services != SurfaceStatus::Active {
        blocked.push("services");
    }
    blocked
}

pub(super) fn next_actions(
    blocked_domains: &[&'static str],
    canonical_next_command: Option<&str>,
) -> Vec<String> {
    let mut actions = Vec::new();
    if blocked_domains.contains(&"principal") {
        actions.push("repair_host_principal".to_string());
    }
    if blocked_domains.contains(&"privilege") {
        actions.push("repair_host_privilege_boundary".to_string());
    }
    if blocked_domains.contains(&"packages") {
        actions.push("repair_host_bootstrap_packages".to_string());
    }
    if blocked_domains.contains(&"services") {
        actions.push("repair_host_runtime_services".to_string());
    }
    if let Some(command) = canonical_next_command {
        actions.push(command.to_string());
    }
    actions.sort();
    actions.dedup();
    actions
}

pub(super) fn canonical_projection(
    observed: &HostObservationReports,
) -> (
    LifecycleStatus,
    LifecycleDisposition,
    BoundaryKind,
    String,
    Option<String>,
) {
    for report in [
        (
            observed.principal.status,
            observed.principal.lifecycle_status,
            observed.principal.lifecycle_disposition,
            observed.principal.next_boundary_kind,
            observed.principal.next_boundary_reason.clone(),
            observed.principal.next_command.clone(),
        ),
        (
            observed.privilege.status,
            observed.privilege.lifecycle_status,
            observed.privilege.lifecycle_disposition,
            observed.privilege.next_boundary_kind,
            observed.privilege.next_boundary_reason.clone(),
            observed.privilege.next_command.clone(),
        ),
        (
            observed.packages.status,
            observed.packages.lifecycle_status,
            observed.packages.lifecycle_disposition,
            observed.packages.next_boundary_kind,
            observed.packages.next_boundary_reason.clone(),
            observed.packages.next_command.clone(),
        ),
        (
            observed.services.status,
            observed.services.lifecycle_status,
            observed.services.lifecycle_disposition,
            observed.services.next_boundary_kind,
            observed.services.next_boundary_reason.clone(),
            observed.services.next_command.clone(),
        ),
    ] {
        if report.0 != SurfaceStatus::Active {
            return (report.1, report.2, report.3, report.4, report.5);
        }
    }

    (
        LifecycleStatus::Ready,
        LifecycleDisposition::ObservableCandidate,
        BoundaryKind::Observe,
        "host substrate domains are active and ready for observation".to_string(),
        None,
    )
}
