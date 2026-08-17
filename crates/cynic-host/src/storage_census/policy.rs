use super::types::{
    CensusKnowledge, StorageCensusAction, StorageCensusDecision, StorageCensusReport,
    StorageCensusState,
};
use crate::domain::disk::{DiskPartitionState, DiskReport, build_cleanup_plan};

pub fn decide_storage_census(report: &StorageCensusReport) -> StorageCensusDecision {
    let mut state = StorageCensusState::Healthy;
    let mut residual_unknowns = report.residual_unknowns.clone();
    let mut reclaimable_bytes = report
        .docker
        .as_ref()
        .and_then(|docker| docker.reclaimable_bytes);
    if let Some(disk) = &report.disk {
        if let Some(reclaimable_from_disk) = disk_preflight_reclaimable(disk) {
            let current = reclaimable_bytes.unwrap_or(0);
            reclaimable_bytes = Some(current + reclaimable_from_disk);
        } else {
            residual_unknowns.push("disk cleanup preflight estimate is unavailable".into());
        }
    } else {
        residual_unknowns
            .push("disk preflight and cleanup report are absent from this census pass".into());
    }

    if report
        .disk
        .as_ref()
        .is_some_and(|disk| disk.partitions.is_empty())
    {
        residual_unknowns.push(
            "disk partition telemetry is unavailable for preflight pressure assessment".into(),
        );
    }
    if matches!(
        report.coverage.devices,
        CensusKnowledge::Unknown | CensusKnowledge::Blocked
    ) || matches!(
        report.coverage.filesystems,
        CensusKnowledge::Unknown | CensusKnowledge::Blocked
    ) {
        state = StorageCensusState::Unknown;
        residual_unknowns.push("device or filesystem coverage is unavailable".into());
    }
    if report.disk.as_ref().is_some_and(|disk| {
        let has_warn = disk
            .partitions
            .iter()
            .any(|partition| matches!(partition.state, DiskPartitionState::PressureWarn));
        if has_warn {
            return true;
        }

        disk.filesystems
            .iter()
            .any(|fs| parse_pct(&fs.use_pct).is_some_and(|value| (90..95).contains(&value)))
    }) {
        state = max_state(state, StorageCensusState::Degraded);
        residual_unknowns.push("at least one observed filesystem is above 90% usage".into());
    }
    if report.disk.as_ref().is_some_and(|disk| {
        let has_critical = disk
            .partitions
            .iter()
            .any(|partition| matches!(partition.state, DiskPartitionState::PressureCritical));
        if has_critical {
            return true;
        }

        disk.filesystems
            .iter()
            .any(|fs| parse_pct(&fs.use_pct).is_some_and(|value| value >= 95))
    }) {
        state = StorageCensusState::Critical;
        residual_unknowns.push("an observed filesystem is full".into());
    }
    if report.docker.is_some() && matches!(report.coverage.containers, CensusKnowledge::Partial) {
        residual_unknowns.push("Docker storage is only partially attributed".into());
    }
    if reclaimable_bytes.is_none() {
        residual_unknowns.push("reclaimable storage is not measured".into());
    }
    let actions = match state {
        StorageCensusState::Healthy => vec![StorageCensusAction::Continue],
        StorageCensusState::Degraded => vec![
            StorageCensusAction::ThrottleStorageHeavyWorkloads,
            StorageCensusAction::PauseNonEssentialIngestion,
        ],
        StorageCensusState::Critical => vec![
            StorageCensusAction::ThrottleStorageHeavyWorkloads,
            StorageCensusAction::PauseNonEssentialIngestion,
            StorageCensusAction::PreserveEvidence,
            StorageCensusAction::RequestHumanIntervention,
        ],
        StorageCensusState::Unknown => vec![StorageCensusAction::RequestHumanIntervention],
    };
    StorageCensusDecision {
        schema: "cynic.host.storage_census_decision.v1".into(),
        node_id: report.node_id.clone(),
        state,
        actions,
        reclaimable_bytes,
        residual_unknowns,
        mutation_authorized: false,
    }
}

fn disk_preflight_reclaimable(disk: &DiskReport) -> Option<u64> {
    if disk.filesystems.is_empty() && disk.known_caches.is_empty() {
        return None;
    }

    Some(build_cleanup_plan(disk).total_reclaimable_bytes)
}

fn parse_pct(value: &str) -> Option<u8> {
    value.trim().trim_end_matches('%').parse::<u8>().ok()
}

fn max_state(left: StorageCensusState, right: StorageCensusState) -> StorageCensusState {
    fn rank(state: StorageCensusState) -> u8 {
        match state {
            StorageCensusState::Healthy => 0,
            StorageCensusState::Degraded => 1,
            StorageCensusState::Critical => 2,
            StorageCensusState::Unknown => 3,
        }
    }
    if rank(right) > rank(left) {
        right
    } else {
        left
    }
}
