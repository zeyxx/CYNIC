use crate::domain::shared::SurfaceStatus;

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

pub(super) fn next_actions(blocked_domains: &[&'static str]) -> Vec<&'static str> {
    let mut actions = Vec::new();
    if blocked_domains.contains(&"principal") {
        actions.push("repair_host_principal");
    }
    if blocked_domains.contains(&"privilege") {
        actions.push("repair_host_privilege_boundary");
    }
    if blocked_domains.contains(&"packages") {
        actions.push("repair_host_bootstrap_packages");
    }
    if blocked_domains.contains(&"services") {
        actions.push("repair_host_runtime_services");
    }
    actions
}
