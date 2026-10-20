//! Build artefact domain — migrated onto the retention trio.
//!
//! The old `BuildArtefactObservationPort` / `BuildArtefactGcExecutionPort`
//! traits and the pure `build_gc_plan` planner are DELETED. The domain now
//! uses `RetentionObserver` / `RetentionPlanner` / `RetentionExecutor` from
//! `cynic-protocol::retention`.

pub mod parse;

pub use cynic_protocol::retention::{
    RETENTION_OBSERVATION_SCHEMA, RETENTION_PLAN_SCHEMA, RETENTION_RESULT_SCHEMA, RetentionError,
    RetentionExecutor, RetentionItem, RetentionItemExecution, RetentionKind, RetentionObservation,
    RetentionObserver, RetentionPlan, RetentionPlanner, RetentionPolicy, RetentionResult,
    RetentionScope,
};
pub use parse::{parse_checkout_list, parse_df_pressure};

/// Pure planner: orphan or old are reclaimable; root-owned is extra signal.
///
/// Under disk pressure the planner also reclaims the freshest artefacts:
/// a full disk blocks every build, so waiting for the 48h TTL is not an
/// option. The oldest are evicted first, and the `preserve_count` freshest
/// are always kept so an in-flight build is never pulled from under itself.
pub fn plan_build_artefacts(obs: &RetentionObservation, policy: &RetentionPolicy) -> RetentionPlan {
    let mut items = Vec::new();
    let mut total = 0u64;
    let pressured = obs.pressure.as_ref().is_some_and(|p| p.is_pressured);
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
            argv: vec!["rm".to_string(), "-rf".to_string(), item.path.clone()],
            is_orphan: item.is_orphan,
            is_root_owned: item.is_root_owned,
            remote_exists: item.remote_exists,
            sha: item.sha.clone(),
            branch: item.branch.clone(),
            owner_uid: item.owner_uid,
            owner_gid: item.owner_gid,
        });
    }

    // Disk pressure: the 48h TTL is a comfort rule, not a safety rule. A
    // builder at 92% fails every build with ENOSPC, so the freshest
    // artefacts are reclaimable too — oldest first, `preserve_count` freshest
    // always kept.
    if pressured {
        let mut fresh: Vec<&RetentionItem> = obs
            .items
            .iter()
            .filter(|item| {
                item.is_orphan != Some(true)
                    && !item
                        .age
                        .and_then(|a| policy.max_age.map(|max| a > max))
                        .unwrap_or(false)
            })
            .collect();
        // Oldest first: the least recently touched artefact is the most
        // disposable. The `preserve_count` freshest are dropped from the
        // tail so the newest survive.
        fresh.sort_by(|a, b| b.age.cmp(&a.age));
        let keep = policy.preserve_count.min(fresh.len());
        let evictable = fresh.len() - keep;
        for item in fresh.into_iter().take(evictable) {
            if items.iter().any(|i| i.path == item.path) {
                continue;
            }
            let age_label = item
                .age
                .map(|a| format!("{}h", a.as_secs() / 3600))
                .unwrap_or_else(|| "?".to_string());
            total += item.size_bytes;
            items.push(RetentionItem {
                id: item.id.clone(),
                path: item.path.clone(),
                size_bytes: item.size_bytes,
                age: item.age,
                reason: format!("disk-pressure(age {age_label})"),
                requires_privilege: item.is_root_owned == Some(true),
                argv: vec!["rm".to_string(), "-rf".to_string(), item.path.clone()],
                is_orphan: item.is_orphan,
                is_root_owned: item.is_root_owned,
                remote_exists: item.remote_exists,
                sha: item.sha.clone(),
                branch: item.branch.clone(),
                owner_uid: item.owner_uid,
                owner_gid: item.owner_gid,
            });
        }
    }

    RetentionPlan {
        schema: cynic_protocol::retention::RETENTION_PLAN_SCHEMA.to_string(),
        items,
        total_reclaimable_bytes: total,
        policy_version: format!("{:?}", policy.kind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use cynic_protocol::retention::PressureSignal;

    fn item(id: &str, age_secs: u64, orphan: bool) -> RetentionItem {
        RetentionItem {
            id: format!("checkout:{id}"),
            path: format!("/var/lib/cynic/checkouts/{id}"),
            size_bytes: 1000,
            age: Some(Duration::from_secs(age_secs)),
            reason: String::new(),
            requires_privilege: false,
            argv: vec![],
            is_orphan: Some(orphan),
            is_root_owned: Some(false),
            remote_exists: Some(!orphan),
            sha: Some(id.to_string()),
            branch: Some(id.to_string()),
            owner_uid: Some(999),
            owner_gid: Some(990),
        }
    }

    fn observation(items: Vec<RetentionItem>, pressured: bool) -> RetentionObservation {
        RetentionObservation {
            schema: RETENTION_OBSERVATION_SCHEMA.to_string(),
            timestamp: "2026-10-20T00:00:00Z".to_string(),
            items,
            total_reclaimable_bytes: 0,
            pressure: Some(PressureSignal {
                use_pct: if pressured { 92 } else { 20 },
                avail_bytes: if pressured {
                    1024 * 1024 * 1024
                } else {
                    100 * 1024 * 1024 * 1024
                },
                is_pressured: pressured,
            }),
        }
    }

    fn policy() -> RetentionPolicy {
        cynic_protocol::retention::default_build_artefact_policy()
    }

    #[test]
    fn without_pressure_the_freshest_checkouts_are_kept() {
        let obs = observation(
            vec![item("fresh", 3600, false), item("older", 7200, false)],
            false,
        );
        let plan = plan_build_artefacts(&obs, &policy());
        assert!(plan.items.is_empty(), "no pressure, no fresh eviction");
    }

    #[test]
    fn under_pressure_the_stale_checkouts_are_reclaimable() {
        let obs = observation(
            vec![item("fresh", 3600, false), item("older", 7200, false)],
            true,
        );
        let plan = plan_build_artefacts(&obs, &policy());
        assert_eq!(
            plan.items.len(),
            1,
            "the freshest is preserved, the older one is reclaimed"
        );
        assert!(plan.items[0].reason.starts_with("disk-pressure"));
        assert_eq!(plan.total_reclaimable_bytes, 1000);
    }

    #[test]
    fn pressure_evicts_the_oldest_first() {
        let obs = observation(
            vec![
                item("freshest", 60, false),
                item("oldest", 10_000, false),
                item("middle", 5_000, false),
            ],
            true,
        );
        let plan = plan_build_artefacts(&obs, &policy());
        let order: Vec<&str> = plan
            .items
            .iter()
            .map(|i| i.path.rsplit('/').next().unwrap())
            .collect();
        assert_eq!(order[0], "oldest", "the oldest is evicted first");
        assert_eq!(order[1], "middle", "then the middle one");
        assert!(
            !order.contains(&"freshest"),
            "the freshest is preserved, not evicted"
        );
        assert_eq!(plan.items.len(), 2);
    }

    #[test]
    fn preserve_count_keeps_the_freshest_under_pressure() {
        let mut policy = policy();
        policy.preserve_count = 1;
        let obs = observation(
            vec![
                item("freshest", 60, false),
                item("oldest", 10_000, false),
                item("middle", 5_000, false),
            ],
            true,
        );
        let plan = plan_build_artefacts(&obs, &policy);
        let paths: Vec<&str> = plan
            .items
            .iter()
            .map(|i| i.path.rsplit('/').next().unwrap())
            .collect();
        assert_eq!(plan.items.len(), 2);
        assert!(!paths.contains(&"freshest"), "the freshest is preserved");
        assert!(paths.contains(&"oldest"));
    }

    #[test]
    fn pressure_does_not_double_count_an_orphan() {
        let obs = observation(vec![item("orphan", 3600, true)], true);
        let plan = plan_build_artefacts(&obs, &policy());
        assert_eq!(plan.items.len(), 1, "the orphan is reclaimed once");
        assert_eq!(plan.items[0].reason, "orphan(no remote)");
    }

    #[test]
    fn the_pressure_path_reuses_the_typed_rm_argv() {
        let obs = observation(
            vec![item("fresh", 3600, false), item("older", 7200, false)],
            true,
        );
        let plan = plan_build_artefacts(&obs, &policy());
        assert_eq!(
            plan.items[0].argv,
            vec![
                "rm".to_string(),
                "-rf".to_string(),
                "/var/lib/cynic/checkouts/older".to_string()
            ]
        );
    }
}
