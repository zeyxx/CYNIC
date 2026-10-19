//! Build artefact domain — migrated onto the retention trio.
//!
//! The old `BuildArtefactObservationPort` / `BuildArtefactGcExecutionPort`
//! traits and the pure `build_gc_plan` planner are DELETED. The domain now
//! uses `RetentionObserver` / `RetentionPlanner` / `RetentionExecutor` from
//! `cynic-protocol::retention`.

pub mod parse;

pub use cynic_protocol::retention::{
    RetentionError, RetentionExecutor, RetentionItem, RetentionItemExecution, RetentionKind,
    RetentionObserver, RetentionObservation, RetentionPlan, RetentionPlanner, RetentionPolicy,
    RetentionResult, RetentionScope, RETENTION_OBSERVATION_SCHEMA, RETENTION_PLAN_SCHEMA,
    RETENTION_RESULT_SCHEMA,
};
pub use parse::{parse_checkout_list, parse_df_pressure};

/// Pure planner: orphan or old are reclaimable; root-owned is extra signal.
/// Shared between local and remote adapters.
pub fn plan_build_artefacts(
    obs: &RetentionObservation,
    policy: &RetentionPolicy,
) -> RetentionPlan {
    let mut items = Vec::new();
    let mut total = 0u64;
    for item in &obs.items {
        let mut reasons = Vec::new();
        if item.is_orphan == Some(true) {
            reasons.push("orphan(no remote)".to_string());
        }
        let is_old = item
            .age
            .and_then(|a| policy.max_age.map(|max| a > max))
            .unwrap_or(false);
        if is_old {
            if let Some(age) = item.age {
                reasons.push(format!("age {}h", age.as_secs() / 3600));
            }
        }
        if reasons.is_empty() {
            continue;
        }
        if item.is_root_owned == Some(true) {
            reasons.push("root-owned".to_string());
        }
        let reason = reasons.join("+");
        total += item.size_bytes;
        items.push(RetentionItem {
            id: item.id.clone(),
            path: item.path.clone(),
            size_bytes: item.size_bytes,
            age: item.age,
            reason,
            requires_privilege: item.is_root_owned == Some(true),
            argv: vec![
                "rm".to_string(),
                "-rf".to_string(),
                item.path.clone(),
            ],
            is_orphan: item.is_orphan,
            is_root_owned: item.is_root_owned,
            remote_exists: item.remote_exists,
            sha: item.sha.clone(),
            branch: item.branch.clone(),
            owner_uid: item.owner_uid,
            owner_gid: item.owner_gid,
        });
    }
    RetentionPlan {
        schema: cynic_protocol::retention::RETENTION_PLAN_SCHEMA.to_string(),
        items,
        total_reclaimable_bytes: total,
        policy_version: format!("{:?}", policy.kind),
    }
}
