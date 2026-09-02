//! Pure queue planning — reconcile branch graph + discovery + GC plan into a QueuePlan.
//! No I/O, no hard-coded argv: BoundedAction is derived from gc_plan items or branch name.

use cynic_protocol::branch_graph::{is_root, is_stale, BranchGraph};
use cynic_protocol::build_artefact::BuildArtefactGcPlan;
use cynic_protocol::build_discovery::BuildDiscoveryReport;
use cynic_protocol::queue::{BoundedAction, QueueItem, QueuePlan, QueueReason, QUEUE_PLAN_SCHEMA};

fn reason_rank(r: &QueueReason) -> u8 {
    match r {
        QueueReason::OrphanReclaimable => 0,
        QueueReason::AgedReclaimable => 1,
        QueueReason::BranchWithoutUpstream => 2,
        QueueReason::StaleBranch => 3,
        QueueReason::UpstreamDivergent => 4,
    }
}

fn gc_reason(reason_str: &str) -> QueueReason {
    let r = reason_str.to_ascii_lowercase();
    if r.contains("orphan") {
        QueueReason::OrphanReclaimable
    } else if r.contains("age") {
        QueueReason::AgedReclaimable
    } else {
        // conservative fallback — treat unknown reclaimable as orphan
        QueueReason::OrphanReclaimable
    }
}

/// Pure: build a canonical QueuePlan from the three observed inputs.
/// - `branch_graph` provides branch liveness / upstream / staleness.
/// - `discovery` provides `recipe_id` (first crate name, or "unknown").
/// - `gc_plan` provides reclaimable artefacts (orphan/aged) + BoundedAction argv.
/// - `now` is unix seconds for staleness check (via `is_stale`).
pub fn build_queue_plan(
    branch_graph: &BranchGraph,
    discovery: &BuildDiscoveryReport,
    gc_plan: &BuildArtefactGcPlan,
    now: u64,
) -> QueuePlan {
    let recipe_id = discovery
        .crates
        .first()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let mut items: Vec<QueueItem> = Vec::new();

    // 1) Branch-graph derived items (typed QueueReason, no hardcoded GC reason string)
    for node in &branch_graph.nodes {
        if is_root(node) {
            continue;
        }
        let reason = if !node.has_upstream {
            Some(QueueReason::BranchWithoutUpstream)
        } else if is_stale(node, now) {
            Some(QueueReason::StaleBranch)
        } else {
            None
        };
        if let Some(reason) = reason {
            items.push(QueueItem {
                id: format!("branch:{}", node.name),
                branch: node.name.clone(),
                recipe_id: recipe_id.clone(),
                reason,
                action: BoundedAction {
                    // derived from branch name, not a magic constant list
                    argv: vec!["git".to_string(), "branch".to_string(), "-D".to_string(), node.name.clone()],
                    requires_privilege: false,
                },
            });
        }
    }

    // 2) GC-plan derived items (BoundedAction reused verbatim, no hardcode)
    for gc in &gc_plan.items {
        let reason = gc_reason(&gc.reason);
        // branch = last path component (checkout dir name), fallback to full path
        let branch = gc
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&gc.path)
            .to_string();
        items.push(QueueItem {
            id: gc.id.clone(),
            branch,
            recipe_id: recipe_id.clone(),
            reason,
            action: BoundedAction {
                argv: gc.argv.clone(),
                requires_privilege: gc.requires_privilege,
            },
        });
    }

    // 3) Canonical order: reason rank -> branch -> id
    items.sort_by(|a, b| {
        reason_rank(&a.reason)
            .cmp(&reason_rank(&b.reason))
            .then(a.branch.cmp(&b.branch))
            .then(a.id.cmp(&b.id))
    });

    QueuePlan {
        schema: QUEUE_PLAN_SCHEMA.to_string(),
        root: branch_graph.root.clone(),
        items,
        total_reclaimable_bytes: gc_plan.total_reclaimable_bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::branch_graph::{BranchGraph, BranchNode, BRANCH_GRAPH_SCHEMA};
    use cynic_protocol::build_artefact::{BuildArtefactGcItem, BuildArtefactGcPlan, BUILD_ARTEFACT_GC_PLAN_SCHEMA};
    use cynic_protocol::build_discovery::{BuildDiscoveryReport, DiscoveredCrate, HostTriple, Toolchain, BUILD_DISCOVERY_REPORT_SCHEMA};

    fn branch_graph() -> BranchGraph {
        BranchGraph {
            schema: BRANCH_GRAPH_SCHEMA.to_string(),
            root: "main".to_string(),
            root_commit: "aaaa".to_string(),
            nodes: vec![
                BranchNode { name: "main".to_string(), commit: "aaaa".to_string(), parent: None, has_upstream: true, last_commit_unix_s: Some(1_000_000) },
                BranchNode { name: "orphan-feature".to_string(), commit: "bbbb".to_string(), parent: Some("main".to_string()), has_upstream: false, last_commit_unix_s: Some(1_000_000) },
                BranchNode { name: "stale-wip".to_string(), commit: "cccc".to_string(), parent: Some("main".to_string()), has_upstream: true, last_commit_unix_s: Some(0) },
            ],
        }
    }

    fn discovery() -> BuildDiscoveryReport {
        BuildDiscoveryReport {
            schema: BUILD_DISCOVERY_REPORT_SCHEMA.to_string(),
            toolchain: Toolchain("stable".to_string()),
            host_triple: HostTriple("x86_64-unknown-linux-gnu".to_string()),
            targets: vec!["x86_64-unknown-linux-gnu".to_string()],
            crates: vec![DiscoveredCrate { name: "cynic-host".to_string(), edition: "2024".to_string(), bins: vec![], libs: vec!["cynic-host".to_string()], tests: vec![], custom_build: false }],
            observed_at_unix_s: 1_000_000,
            residual_unknowns: vec![],
        }
    }

    fn gc_plan() -> BuildArtefactGcPlan {
        BuildArtefactGcPlan {
            schema: BUILD_ARTEFACT_GC_PLAN_SCHEMA.to_string(),
            items: vec![
                BuildArtefactGcItem { id: "checkout:abc123".to_string(), label: "orphan".to_string(), path: "/var/lib/cynic/checkouts/old-orphan".to_string(), size_human: "1M".to_string(), size_bytes: 1024, reason: "orphan(no remote)".to_string(), requires_privilege: false, argv: vec!["rm".to_string(), "-rf".to_string(), "/var/lib/cynic/checkouts/old-orphan".to_string()] },
                BuildArtefactGcItem { id: "checkout:def456".to_string(), label: "aged".to_string(), path: "/var/lib/cynic/checkouts/aged-one".to_string(), size_human: "2M".to_string(), size_bytes: 2048, reason: "age 56h".to_string(), requires_privilege: true, argv: vec!["rm".to_string(), "-rf".to_string(), "/var/lib/cynic/checkouts/aged-one".to_string()] },
            ],
            total_reclaimable_bytes: 3072,
            total_reclaimable_human: "3M".to_string(),
        }
    }

    #[test]
    fn builds_canonical_plan() {
        let bg = branch_graph();
        let d = discovery();
        let gc = gc_plan();
        let now = 1_000_000 + 31 * 24 * 3600; // stale threshold 30d
        let plan = build_queue_plan(&bg, &d, &gc, now);
        assert_eq!(plan.root, "main");
        assert_eq!(plan.total_reclaimable_bytes, 3072);
        // 2 branch + 2 gc = 4
        assert_eq!(plan.items.len(), 4);
        // canonical order: Orphan(0) -> Aged(1) -> BranchWithoutUpstream(2) -> StaleBranch(3)
        assert_eq!(plan.items[0].reason, QueueReason::OrphanReclaimable);
        assert_eq!(plan.items[1].reason, QueueReason::AgedReclaimable);
        assert_eq!(plan.items[2].reason, QueueReason::BranchWithoutUpstream);
        assert_eq!(plan.items[3].reason, QueueReason::StaleBranch);
        // BoundedAction not hardcoded: gc argv reused, privilege preserved
        assert_eq!(plan.items[1].action.requires_privilege, true);
        assert_eq!(plan.items[1].action.argv[2], "/var/lib/cynic/checkouts/aged-one");
        assert_eq!(plan.items[2].action.argv[3], "orphan-feature");
        assert_eq!(plan.items[2].recipe_id, "cynic-host");
    }

    #[test]
    fn skips_root_and_fresh_branch() {
        let bg = BranchGraph {
            schema: BRANCH_GRAPH_SCHEMA.to_string(),
            root: "main".to_string(),
            root_commit: "aaaa".to_string(),
            nodes: vec![
                BranchNode { name: "main".to_string(), commit: "aaaa".to_string(), parent: None, has_upstream: true, last_commit_unix_s: Some(999_999) },
                BranchNode { name: "fresh".to_string(), commit: "bbbb".to_string(), parent: Some("main".to_string()), has_upstream: true, last_commit_unix_s: Some(999_999) },
            ],
        };
        let d = BuildDiscoveryReport { schema: BUILD_DISCOVERY_REPORT_SCHEMA.to_string(), toolchain: Toolchain("stable".into()), host_triple: HostTriple("x".into()), targets: vec![], crates: vec![], observed_at_unix_s: 0, residual_unknowns: vec![] };
        let gc = BuildArtefactGcPlan { schema: BUILD_ARTEFACT_GC_PLAN_SCHEMA.to_string(), items: vec![], total_reclaimable_bytes: 0, total_reclaimable_human: "0".into() };
        let plan = build_queue_plan(&bg, &d, &gc, 1_000_000);
        assert!(plan.items.is_empty());
        assert_eq!(plan.items.len(), 0);
    }
}
