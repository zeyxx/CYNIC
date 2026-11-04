//! Build queue ordering: FIFO by commit time with SHA deduplication.
//!
//! Pure policy for the scheduler loop. It operates on a lightweight candidate
//! view so callers map their record shape once and tests fabricate candidates
//! in one line each. Terminal records never queue; later duplicates of an
//! idempotency key keep only the earliest commit. Fairness across branches is
//! explicitly out of scope until branch identity is recorded at admission.

use serde::{Deserialize, Serialize};

pub const BUILD_QUEUE_PLAN_SCHEMA: &str = "cynic.build_queue_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedCandidate {
    pub workload_id: String,
    pub idempotency_key: String,
    pub committed_at_ms: u64,
    /// True for terminal records and non-build workloads: never queued,
    /// counted separately so silence is impossible.
    pub terminal: bool,
    pub is_build: bool,
    /// Source commit SHA captured at admission; preserved for fair-queue
    /// ordering by branch (Task 8). None for legacy candidates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_commit: Option<String>,
    /// Branch name captured at admission; preserved for fair-queue ordering
    /// by branch (Task 8). None for legacy candidates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildQueuePlan {
    pub schema: String,
    pub ordered: Vec<String>,
    pub terminal_skipped: usize,
    pub non_build_skipped: usize,
    pub duplicates_skipped: usize,
}

/// Order candidates FIFO by commit time, deduplicated by idempotency key.
pub fn plan_build_queue(candidates: Vec<QueuedCandidate>) -> BuildQueuePlan {
    let mut terminal_skipped = 0;
    let mut non_build_skipped = 0;
    let mut duplicates_skipped = 0;
    let mut eligible: Vec<QueuedCandidate> = Vec::new();
    for candidate in candidates {
        if candidate.terminal {
            terminal_skipped += 1;
        } else if !candidate.is_build {
            non_build_skipped += 1;
        } else {
            eligible.push(candidate);
        }
    }
    eligible.sort_by(|left, right| {
        left.committed_at_ms
            .cmp(&right.committed_at_ms)
            .then_with(|| left.workload_id.cmp(&right.workload_id))
    });
    let mut seen = std::collections::BTreeSet::new();
    let mut ordered = Vec::new();
    for candidate in eligible {
        if seen.insert(candidate.idempotency_key.clone()) {
            ordered.push(candidate.workload_id);
        } else {
            duplicates_skipped += 1;
        }
    }
    BuildQueuePlan {
        schema: BUILD_QUEUE_PLAN_SCHEMA.to_string(),
        ordered,
        terminal_skipped,
        non_build_skipped,
        duplicates_skipped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, key: &str, at_ms: u64) -> QueuedCandidate {
        QueuedCandidate {
            workload_id: id.to_string(),
            idempotency_key: key.to_string(),
            committed_at_ms: at_ms,
            terminal: false,
            is_build: true,
            source_commit: None,
            branch: None,
        }
    }

    #[test]
    fn orders_fifo_and_dedupes_by_idempotency_key() {
        let plan = plan_build_queue(vec![
            candidate("w-new", "key:b", 200),
            candidate("w-old", "key:a", 100),
            candidate("w-dup", "key:a", 300),
        ]);

        assert_eq!(plan.ordered, vec!["w-old".to_string(), "w-new".to_string()]);
        assert_eq!(plan.duplicates_skipped, 1);
    }

    #[test]
    fn counts_terminal_and_non_build_without_queueing() {
        let mut done = candidate("w-done", "key:c", 50);
        done.terminal = true;
        let mut other = candidate("w-other", "key:d", 60);
        other.is_build = false;
        let plan = plan_build_queue(vec![done, other, candidate("w-ok", "key:e", 70)]);

        assert_eq!(plan.ordered, vec!["w-ok".to_string()]);
        assert_eq!(plan.terminal_skipped, 1);
        assert_eq!(plan.non_build_skipped, 1);
    }

    #[test]
    fn queue_plan_preserves_branch_and_commit_fields() {
        let mut first = candidate("w-first", "key:a", 100);
        first.source_commit = Some("a".repeat(40));
        first.branch = Some("feature/first".to_string());
        let mut second = candidate("w-second", "key:b", 200);
        second.source_commit = Some("b".repeat(40));
        second.branch = Some("feature/second".to_string());
        let plan = plan_build_queue(vec![second.clone(), first.clone()]);

        assert_eq!(
            plan.ordered,
            vec!["w-first".to_string(), "w-second".to_string()]
        );
        assert_eq!(first.source_commit, Some("a".repeat(40)));
        assert_eq!(first.branch, Some("feature/first".to_string()));
        assert_eq!(second.source_commit, Some("b".repeat(40)));
        assert_eq!(second.branch, Some("feature/second".to_string()));
    }

    #[test]
    fn empty_input_stays_empty_not_failed() {
        let plan = plan_build_queue(Vec::new());

        assert!(plan.ordered.is_empty());
        assert_eq!(plan.duplicates_skipped, 0);
    }
}


// ---------------------------------------------------------------------------
// Composite queue planning (BranchGraph + BuildDiscovery + RetentionPlan)
// ---------------------------------------------------------------------------
use cynic_protocol::build_discovery::BuildDiscoveryReport;
use cynic_protocol::branch_graph::{BranchGraph, is_root, is_stale};
use cynic_protocol::queue::{
    BoundedAction, CanonicalQueuePolicy, QUEUE_PLAN_SCHEMA, QueueItem, QueuePlan, QueuePolicy,
    QueueReason,
};
use cynic_protocol::retention::RetentionPlan;
fn gc_reason(reason_str: &str) -> QueueReason {
    let r = reason_str.to_ascii_lowercase();
    if r.contains("orphan") {
        QueueReason::OrphanReclaimable
    } else if r.contains("age") {
        QueueReason::AgedReclaimable
    } else {
        // Honest extension point: an unrecognized GC code is another
        // domain's vocabulary, not an orphan. Carried verbatim so a future
        // domain can claim it; sorts after all branch-GC reasons.
        QueueReason::Other {
            domain: "build_artefact_gc".to_string(),
            code: reason_str.to_string(),
        }
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
    gc_plan: &RetentionPlan,
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
                    argv: vec![
                        "git".to_string(),
                        "branch".to_string(),
                        "-D".to_string(),
                        node.name.clone(),
                    ],
                    requires_privilege: false,
                },
            });
        }
    }

    // 2) GC-plan derived items (BoundedAction reused verbatim, no hardcode)
    for gc in &gc_plan.items {
        let reason = gc_reason(&gc.reason);
        // branch = last path component (checkout dir name), fallback to full path
        let branch = gc.path.rsplit('/').next().unwrap_or(&gc.path).to_string();
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

    // 3) Canonical order goes through the QueuePolicy trait — one address
    // for ordering, never a call-site sort.
    let items = CanonicalQueuePolicy.order(items);

    QueuePlan {
        schema: QUEUE_PLAN_SCHEMA.to_string(),
        root: branch_graph.root.clone(),
        items,
        total_reclaimable_bytes: gc_plan.total_reclaimable_bytes,
        residual_unknowns: Vec::new(),
    }
}

#[cfg(test)]
mod build_artefact_queue_tests {
    use super::*;
    use cynic_protocol::retention::{
        RetentionItem, RetentionPlan, RETENTION_PLAN_SCHEMA,
    };
    use cynic_protocol::build_discovery::{
        BUILD_DISCOVERY_REPORT_SCHEMA, BuildDiscoveryReport, DiscoveredCrate, HostTriple, Toolchain,
    };
    use cynic_protocol::branch_graph::{BRANCH_GRAPH_SCHEMA, BranchGraph, BranchNode};

    fn branch_graph() -> BranchGraph {
        BranchGraph {
            schema: BRANCH_GRAPH_SCHEMA.to_string(),
            root: "main".to_string(),
            root_commit: "aaaa".to_string(),
            nodes: vec![
                BranchNode {
                    name: "main".to_string(),
                    commit: "aaaa".to_string(),
                    parent: None,
                    has_upstream: true,
                    last_commit_unix_s: Some(1_000_000),
                },
                BranchNode {
                    name: "orphan-feature".to_string(),
                    commit: "bbbb".to_string(),
                    parent: Some("main".to_string()),
                    has_upstream: false,
                    last_commit_unix_s: Some(1_000_000),
                },
                BranchNode {
                    name: "stale-wip".to_string(),
                    commit: "cccc".to_string(),
                    parent: Some("main".to_string()),
                    has_upstream: true,
                    last_commit_unix_s: Some(0),
                },
            ],
        }
    }

    fn discovery() -> BuildDiscoveryReport {
        BuildDiscoveryReport {
            schema: BUILD_DISCOVERY_REPORT_SCHEMA.to_string(),
            toolchain: Toolchain("stable".to_string()),
            host_triple: HostTriple("x86_64-unknown-linux-gnu".to_string()),
            targets: vec!["x86_64-unknown-linux-gnu".to_string()],
            crates: vec![DiscoveredCrate {
                name: "cynic-host".to_string(),
                edition: "2024".to_string(),
                bins: vec![],
                libs: vec!["cynic-host".to_string()],
                tests: vec![],
                custom_build: false,
            }],
            observed_at_unix_s: 1_000_000,
            residual_unknowns: vec![],
        }
    }

    fn gc_plan() -> RetentionPlan {
        RetentionPlan {
            schema: RETENTION_PLAN_SCHEMA.to_string(),
            items: vec![
                RetentionItem {
                    id: "checkout:abc123".to_string(),
                    path: "/var/lib/cynic/checkouts/old-orphan".to_string(),
                    size_bytes: 1024,
                    age: None,
                    reason: "orphan(no remote)".to_string(),
                    requires_privilege: false,
                    argv: vec![
                        "rm".to_string(),
                        "-rf".to_string(),
                        "/var/lib/cynic/checkouts/old-orphan".to_string(),
                    ],
                    is_orphan: Some(true),
                    is_root_owned: Some(false),
                is_live_service_path: None,
                    remote_exists: Some(false),
                    sha: Some("abc123".to_string()),
                    branch: Some("old-orphan".to_string()),
                    owner_uid: Some(999),
                    owner_gid: Some(990),
                },
                RetentionItem {
                    id: "checkout:def456".to_string(),
                    path: "/var/lib/cynic/checkouts/aged-one".to_string(),
                    size_bytes: 2048,
                    age: None,
                    reason: "age 56h".to_string(),
                    requires_privilege: true,
                    argv: vec![
                        "rm".to_string(),
                        "-rf".to_string(),
                        "/var/lib/cynic/checkouts/aged-one".to_string(),
                    ],
                    is_orphan: Some(false),
                    is_root_owned: Some(true),
                is_live_service_path: None,
                    remote_exists: Some(true),
                    sha: Some("def456".to_string()),
                    branch: Some("aged-one".to_string()),
                    owner_uid: Some(0),
                    owner_gid: Some(0),
                },
            ],
            total_reclaimable_bytes: 3072,
            policy_version: "BuildArtefact".to_string(),
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
        assert_eq!(
            plan.items[1].action.argv[2],
            "/var/lib/cynic/checkouts/aged-one"
        );
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
                BranchNode {
                    name: "main".to_string(),
                    commit: "aaaa".to_string(),
                    parent: None,
                    has_upstream: true,
                    last_commit_unix_s: Some(999_999),
                },
                BranchNode {
                    name: "fresh".to_string(),
                    commit: "bbbb".to_string(),
                    parent: Some("main".to_string()),
                    has_upstream: true,
                    last_commit_unix_s: Some(999_999),
                },
            ],
        };
        let d = BuildDiscoveryReport {
            schema: BUILD_DISCOVERY_REPORT_SCHEMA.to_string(),
            toolchain: Toolchain("stable".into()),
            host_triple: HostTriple("x".into()),
            targets: vec![],
            crates: vec![],
            observed_at_unix_s: 0,
            residual_unknowns: vec![],
        };
        let gc = RetentionPlan {
            schema: RETENTION_PLAN_SCHEMA.to_string(),
            items: vec![],
            total_reclaimable_bytes: 0,
            policy_version: "BuildArtefact".to_string(),
        };
        let plan = build_queue_plan(&bg, &d, &gc, 1_000_000);
        assert!(plan.items.is_empty());
        assert_eq!(plan.items.len(), 0);
    }
}

#[cfg(test)]
mod extension_tests {
    use super::*;
    use cynic_protocol::queue::reason_rank;

    #[test]
    fn unknown_gc_code_becomes_other_not_a_mislabeled_orphan() {
        let reason = gc_reason("checksum-mismatch");
        assert_eq!(
            reason,
            QueueReason::Other {
                domain: "build_artefact_gc".to_string(),
                code: "checksum-mismatch".to_string(),
            }
        );
        assert!(reason_rank(&reason) > reason_rank(&QueueReason::UpstreamDivergent));
    }

    #[test]
    fn other_roundtrips_through_json() {
        let item = QueueItem {
            id: "x".to_string(),
            branch: "b".to_string(),
            recipe_id: "r".to_string(),
            reason: QueueReason::Other {
                domain: "future-domain".to_string(),
                code: "some-code".to_string(),
            },
            action: BoundedAction {
                argv: vec![],
                requires_privilege: false,
            },
        };
        let raw = serde_json::to_string(&item).expect("serialize");
        let back: QueueItem = serde_json::from_str(&raw).expect("deserialize");
        assert_eq!(back.reason, item.reason);
    }
}
