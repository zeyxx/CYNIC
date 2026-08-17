//! Resource, pressure, and fleet extraction helpers for introspection.

use crate::domain::probe::{
    EnvironmentSnapshot, FleetDetails, PressureDetails, ProbeDetails, ResourceDetails,
};
use crate::introspection::types::Alert;

/// Extract ResourceDetails from an EnvironmentSnapshot's probes.
pub fn extract_resource(snap: &EnvironmentSnapshot) -> Option<ResourceDetails> {
    snap.probes.iter().find_map(|p| match &p.details {
        ProbeDetails::Resource(r) => Some(r.clone()),
        _ => None,
    })
}

/// Extract PressureDetails from an EnvironmentSnapshot's probes.
pub fn extract_pressure(snap: &EnvironmentSnapshot) -> Option<PressureDetails> {
    snap.probes.iter().find_map(|p| match &p.details {
        ProbeDetails::Pressure(pr) => Some(pr.clone()),
        _ => None,
    })
}

/// Extract FleetDetails from an EnvironmentSnapshot's probes.
pub fn extract_fleet(snap: &EnvironmentSnapshot) -> Option<FleetDetails> {
    snap.probes.iter().find_map(|p| match &p.details {
        ProbeDetails::Fleet(f) => Some(f.clone()),
        _ => None,
    })
}

/// Check resource thresholds and push alerts.
pub fn check_resource_thresholds(r: &ResourceDetails, alerts: &mut Vec<Alert>) {
    // Memory pressure
    if let (Some(used), Some(total)) = (r.memory_used_gb, r.memory_total_gb)
        && total > 0.0
    {
        let ratio = used / total;
        if ratio > 0.95 {
            alerts.push(Alert {
                kind: "memory_pressure",
                message: format!(
                    "Memory critical: {:.1}% used ({:.1}/{:.1} GB)",
                    ratio * 100.0,
                    used,
                    total
                ),
                severity: "critical",
            });
        } else if ratio > 0.90 {
            alerts.push(Alert {
                kind: "memory_pressure",
                message: format!(
                    "Memory high: {:.1}% used ({:.1}/{:.1} GB)",
                    ratio * 100.0,
                    used,
                    total
                ),
                severity: "warning",
            });
        }
    }

    // CPU sustained (skip None — first tick or non-Linux)
    if let Some(cpu) = r.cpu_usage_percent
        && cpu > 80.0
    {
        alerts.push(Alert {
            kind: "cpu_sustained",
            message: format!("CPU high: {cpu:.1}%"),
            severity: "warning",
        });
    }

    // Disk low
    if let (Some(avail), Some(total)) = (r.disk_available_gb, r.disk_total_gb)
        && total > 0.0
    {
        let avail_ratio = avail / total;
        if avail_ratio < 0.05 {
            alerts.push(Alert {
                kind: "disk_low",
                message: format!(
                    "Disk critical: {:.1}% available ({:.0}/{:.0} GB)",
                    avail_ratio * 100.0,
                    avail,
                    total
                ),
                severity: "critical",
            });
        } else if avail_ratio < 0.10 {
            alerts.push(Alert {
                kind: "disk_low",
                message: format!(
                    "Disk low: {:.1}% available ({:.0}/{:.0} GB)",
                    avail_ratio * 100.0,
                    avail,
                    total
                ),
                severity: "warning",
            });
        }
    }
}
