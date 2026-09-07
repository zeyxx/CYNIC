//! Node-initiated build pull: select runnable check-phase build workloads.
//!
//! Pure reconcile over the durable workload store plus delivery leases. It
//! never spawns, never mutates, never acquires leases: lease lifecycle stays
//! with the delivery use case. Only `cargo check` argv is eligible — later
//! phases mutate and need their own admission slice. A missing or expired
//! lease skips the workload with an explicit reason instead of running it.

use cynic_protocol::{AgentWorkloadState, DurableAgentWorkloadRecord};

use crate::ports::agent_workload_delivery_lease_port::AgentWorkloadDeliveryLeasePort;
use crate::ports::agent_workload_store_port::AgentWorkloadStorePort;
use crate::ports::build_runner_adapter::{CommandOutput, CommandRunner};

pub const BUILD_PULL_PLAN_SCHEMA: &str = "cynic.node.build_pull_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildPullItem {
    pub workload_id: String,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildPullPlan {
    pub schema: String,
    pub node_id: String,
    pub runnable: Vec<BuildPullItem>,
    pub skipped_terminal: u64,
    pub skipped_no_lease: u64,
    pub skipped_not_build: u64,
    pub blockers: Vec<String>,
}

fn is_check_argv(argv: &[String]) -> bool {
    let mut parts = argv.iter().map(String::as_str);
    match parts.next().map(|program| {
        std::path::Path::new(program)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
    }) {
        Some("cargo") => parts.next() == Some("check"),
        _ => false,
    }
}

/// Select the check-phase build workloads this node may run now.
///
/// `now_ms` bounds lease validity. Every skip is counted by cause; lease
/// read failures block the plan loudly instead of silently dropping work.
pub fn select_build_pull(
    node_id: &str,
    now_ms: u64,
    records: &[DurableAgentWorkloadRecord],
    leases: &dyn AgentWorkloadDeliveryLeasePort,
) -> BuildPullPlan {
    let mut plan = BuildPullPlan {
        schema: BUILD_PULL_PLAN_SCHEMA.to_string(),
        node_id: node_id.to_string(),
        runnable: Vec::new(),
        skipped_terminal: 0,
        skipped_no_lease: 0,
        skipped_not_build: 0,
        blockers: Vec::new(),
    };
    for durable in records {
        let workload_id = durable.record.envelope.workload_id.clone();
        if durable.record.state.terminal() {
            plan.skipped_terminal += 1;
            continue;
        }
        if !is_check_argv(&durable.record.envelope.action.argv) {
            plan.skipped_not_build += 1;
            continue;
        }
        match leases.read(&workload_id) {
            Ok(Some(lease))
                if lease.consumer_instance_id == node_id && lease.expires_at_ms > now_ms =>
            {
                plan.runnable.push(BuildPullItem {
                    workload_id,
                    argv: durable.record.envelope.action.argv.clone(),
                });
            }
            Ok(_) => plan.skipped_no_lease += 1,
            Err(error) => plan.blockers.push(format!(
                "build_pull_lease_read_failed:{workload_id}:{error}"
            )),
        }
    }
    plan
}

/// Dry-run execution plan: read admitted workloads from the store and select
/// what this node would run now. No spawn, no mutation — the argv returned
/// is exactly what the runner would execute.
pub fn plan_pull_run(
    node_id: &str,
    now_ms: u64,
    store: &dyn AgentWorkloadStorePort,
    leases: &dyn AgentWorkloadDeliveryLeasePort,
) -> Result<BuildPullPlan, String> {
    let records = store.durable_records().map_err(|error| {
        format!(
            "build_pull_store_read_failed:{}:{}",
            error.code, error.detail
        )
    })?;
    Ok(select_build_pull(node_id, now_ms, &records, leases))
}

const STDOUT_TAIL_BYTES: usize = 2048;

fn tail(output: &str) -> String {
    if output.len() <= STDOUT_TAIL_BYTES {
        return output.to_string();
    }
    output[output.len() - STDOUT_TAIL_BYTES..].to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRunItem {
    pub workload_id: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub state: AgentWorkloadState,
    pub stdout_tail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRunReport {
    pub schema: String,
    pub node_id: String,
    pub ran: Vec<BuildRunItem>,
    pub blockers: Vec<String>,
}

/// Execute up to `max_items` runnable check workloads locally and record
/// their terminal state. Spawn goes only through the injected runner, argv
/// only from the admitted plan. A revision conflict stops the run loudly:
/// someone else moved the workload. Receipt emission stays with delivery.
pub fn execute_pull_run(
    node_id: &str,
    now_ms: u64,
    committed_at_ms: u64,
    max_items: usize,
    store: &dyn AgentWorkloadStorePort,
    leases: &dyn AgentWorkloadDeliveryLeasePort,
    runner: &dyn CommandRunner,
) -> BuildRunReport {
    let mut report = BuildRunReport {
        schema: BUILD_PULL_PLAN_SCHEMA.to_string(),
        node_id: node_id.to_string(),
        ran: Vec::new(),
        blockers: Vec::new(),
    };
    let plan = match plan_pull_run(node_id, now_ms, store, leases) {
        Ok(plan) => plan,
        Err(error) => {
            report.blockers.push(error);
            return report;
        }
    };
    report.blockers.extend(plan.blockers);
    for item in plan.runnable.into_iter().take(max_items.max(1)) {
        let durable = match store.read(&item.workload_id) {
            Ok(Some(durable)) => durable,
            Ok(None) => {
                report
                    .blockers
                    .push(format!("build_run_record_vanished:{}", item.workload_id));
                break;
            }
            Err(error) => {
                report.blockers.push(format!(
                    "build_run_record_read_failed:{}:{}:{}",
                    item.workload_id, error.code, error.detail
                ));
                break;
            }
        };
        let refs: Vec<&str> = item.argv.iter().map(String::as_str).collect();
        let out: CommandOutput = runner.run(&refs);
        let mut next = durable.record.clone();
        next.state = if out.success() {
            AgentWorkloadState::Completed
        } else {
            AgentWorkloadState::Failed
        };
        next.consumed_actions += 1;
        match store.compare_and_swap(
            &item.workload_id,
            Some(durable.revision),
            &next,
            committed_at_ms,
        ) {
            Ok(_) => report.ran.push(BuildRunItem {
                workload_id: item.workload_id,
                success: out.success(),
                exit_code: out.exit_code,
                state: next.state,
                stdout_tail: tail(&out.stdout),
            }),
            Err(error) => {
                report.blockers.push(format!(
                    "build_run_revision_conflict:{}:{}:{}",
                    item.workload_id, error.code, error.detail
                ));
                break;
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        AGENT_DELEGATION_ENVELOPE_SCHEMA, AGENT_WORKLOAD_ENVELOPE_SCHEMA,
        AGENT_WORKLOAD_RECORD_SCHEMA, AgentBoundedAction, AgentBudget, AgentCapabilityGrant,
        AgentDelegationEnvelope, AgentEvidenceRef, AgentWorkloadDeliveryLease,
        AgentWorkloadEnvelope, AgentWorkloadRecord, DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA,
        EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization, MutationContract,
        POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority, PolicyDecision,
        PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof, digest_json,
    };
    use std::collections::HashMap;

    struct MapLeases {
        entries: HashMap<String, AgentWorkloadDeliveryLease>,
        fail: Vec<String>,
    }

    impl AgentWorkloadDeliveryLeasePort for MapLeases {
        fn read(&self, workload_id: &str) -> Result<Option<AgentWorkloadDeliveryLease>, String> {
            if self.fail.iter().any(|id| id == workload_id) {
                return Err("store offline".to_string());
            }
            Ok(self.entries.get(workload_id).cloned())
        }

        fn persist(
            &self,
            _workload_id: &str,
            _lease: &AgentWorkloadDeliveryLease,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    fn lease(holder: &str, expires_at_ms: u64) -> AgentWorkloadDeliveryLease {
        AgentWorkloadDeliveryLease {
            schema: "cynic.agent_workload_delivery_lease.v1".to_string(),
            lease_id: format!("lease-{holder}"),
            fencing_token: 1,
            consumer_instance_id: holder.to_string(),
            accepted_at_ms: 1,
            expires_at_ms,
            record_digest: "sha256:abc".to_string(),
        }
    }

    fn envelope(workload_id: &str, argv: Vec<&str>) -> AgentWorkloadEnvelope {
        let action = AgentBoundedAction {
            argv: argv.into_iter().map(str::to_string).collect(),
            working_directory: Some("checkout".into()),
            input_digest: Some(format!("sha256:{}", "b".repeat(64))),
            mutates: false,
            requires_privilege: false,
            stop_condition: "stop on first non-zero exit".into(),
            rollback_intent: "discard isolated checkout".into(),
        };
        let plan_digest = digest_json(&action).expect("action digest");
        AgentWorkloadEnvelope {
            schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.into(),
            workload_id: workload_id.into(),
            idempotency_key: format!("key-{workload_id}"),
            plan_digest: plan_digest.clone(),
            mandate_binding: None,
            input: None,
            action,
            delegation: AgentDelegationEnvelope {
                schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.into(),
                delegation_id: format!("delegation-{workload_id}"),
                node_id: "builder-1".into(),
                agent_id: "build-agent-1".into(),
                parent_agent_id: None,
                depth: 0,
                objective: "run canonical check".into(),
                authority_snapshot: AgentEvidenceRef {
                    kind: "authority_snapshot".into(),
                    subject: workload_id.into(),
                    digest: format!("sha256:{}", "e".repeat(64)),
                    lineage: None,
                },
                context_refs: vec![],
                capabilities: vec![AgentCapabilityGrant {
                    id: "repository_validation".into(),
                    mutates: false,
                }],
                budget: AgentBudget {
                    context_tokens: 1_000,
                    actions: 2,
                    children: 0,
                    wall_time_ms: 900_000,
                    cpu_ms: None,
                },
                expected_proofs: vec!["canonical_gate_report".into()],
            },
            parent_delegation: None,
            policy_admission: PolicyAdmissionReport {
                schema: POLICY_ADMISSION_REPORT_SCHEMA.into(),
                policy_set: PolicySetRef {
                    id: "build.validation".into(),
                    version: "v1".into(),
                    digest: format!("sha256:{}", "c".repeat(64)),
                },
                intent: PolicyIntentRef {
                    id: format!("key-{workload_id}"),
                    kind: "repository_validation".into(),
                    actor: "build-agent-1".into(),
                    subject: "commit-a".into(),
                    digest: format!("sha256:{}", "d".repeat(64)),
                    plan_digest,
                },
                mutation: MutationContract {
                    kind: "repository_validation".into(),
                    scope: PolicyScope {
                        domain: "build".into(),
                        target: "commit-a".into(),
                    },
                    mutates: false,
                    requires_privilege: false,
                    requires_explicit_apply: false,
                    required_authority: PolicyAuthority::RuntimePrincipal,
                    preconditions: vec!["source_commit_observed".into()],
                    rollback_intent: "discard isolated checkout".into(),
                    expected_proofs: vec![RequiredProof {
                        gate: "canonical_gate_report".into(),
                        subject: "commit-a".into(),
                    }],
                },
                decision: PolicyDecision::Allow {
                    required_authority: PolicyAuthority::RuntimePrincipal,
                    required_proofs: vec![],
                },
            },
            execution_authorization: ExecutionAuthorization {
                schema: EXECUTION_AUTHORIZATION_SCHEMA.into(),
                actor: "build-agent-1".into(),
                authority: PolicyAuthority::RuntimePrincipal,
                explicit_apply: false,
                satisfied_preconditions: vec!["source_commit_observed".into()],
            },
        }
    }

    fn record(
        workload_id: &str,
        state: AgentWorkloadState,
        argv: Vec<&str>,
    ) -> DurableAgentWorkloadRecord {
        DurableAgentWorkloadRecord {
            schema: DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.into(),
            revision: 1,
            previous_record_digest: None,
            record_digest: "sha256:abc".into(),
            record: AgentWorkloadRecord {
                schema: AGENT_WORKLOAD_RECORD_SCHEMA.into(),
                envelope: envelope(workload_id, argv),
                state,
                claim: None,
                last_fencing_token: 0,
                cancellation_generation: 0,
                consumed_actions: 0,
                completion: None,
            },
            committed_at_ms: 1,
        }
    }

    fn check_argv() -> Vec<&'static str> {
        vec!["cargo", "check", "--package", "cynic-host", "--lib"]
    }

    #[test]
    fn selects_held_check_workload() {
        let mut entries = HashMap::new();
        entries.insert("w1".to_string(), lease("node-1", 999));
        let leases = MapLeases {
            entries,
            fail: Vec::new(),
        };
        let records = vec![record("w1", AgentWorkloadState::Dispatched, check_argv())];

        let plan = select_build_pull("node-1", 100, &records, &leases);

        assert!(plan.blockers.is_empty());
        assert_eq!(plan.runnable.len(), 1);
        assert_eq!(plan.runnable[0].workload_id, "w1");
    }

    #[test]
    fn skips_terminal_other_holder_expired_and_non_build() {
        let mut entries = HashMap::new();
        entries.insert("other".to_string(), lease("node-9", 999));
        entries.insert("expired".to_string(), lease("node-1", 10));
        let leases = MapLeases {
            entries,
            fail: Vec::new(),
        };
        let records = vec![
            record("done", AgentWorkloadState::Completed, check_argv()),
            record("other", AgentWorkloadState::Dispatched, check_argv()),
            record("expired", AgentWorkloadState::Dispatched, check_argv()),
            record(
                "shell",
                AgentWorkloadState::Dispatched,
                vec!["sh", "-c", "cargo check"],
            ),
        ];

        let plan = select_build_pull("node-1", 100, &records, &leases);

        assert!(plan.runnable.is_empty());
        assert_eq!(plan.skipped_terminal, 1);
        assert_eq!(plan.skipped_no_lease, 2);
        assert_eq!(plan.skipped_not_build, 1);
    }

    #[test]
    fn lease_read_failure_blocks_loudly() {
        let leases = MapLeases {
            entries: HashMap::new(),
            fail: vec!["w1".to_string()],
        };
        let records = vec![record("w1", AgentWorkloadState::Dispatched, check_argv())];

        let plan = select_build_pull("node-1", 100, &records, &leases);

        assert!(plan.runnable.is_empty());
        assert_eq!(plan.blockers.len(), 1);
    }

    use crate::adapters::local_agent_workload_store::LocalAgentWorkloadStore;
    use crate::usecases::build_admission::admit_build_check;
    use std::sync::atomic::{AtomicU64, Ordering};

    static PULL_DRY_SEQ: AtomicU64 = AtomicU64::new(0);

    const DRY_COMMIT: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4";

    fn dry_store() -> LocalAgentWorkloadStore {
        let id = PULL_DRY_SEQ.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("cynic-pull-dry-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create pull dry-run store dir");
        LocalAgentWorkloadStore::new(&dir)
    }

    fn dry_workload_id() -> String {
        format!("build-{}-cynic-host", &DRY_COMMIT[..12])
    }

    #[test]
    fn dry_run_plans_admitted_store_content_without_spawning() {
        let store = dry_store();
        admit_build_check(
            DRY_COMMIT,
            "cynic-host",
            "cynic-builder",
            "node-1",
            "agent-1",
            100,
            &store,
        )
        .expect("admit dry-run build");
        let mut entries = HashMap::new();
        entries.insert(dry_workload_id(), lease("node-1", 999));
        let leases = MapLeases {
            entries,
            fail: Vec::new(),
        };

        let plan = plan_pull_run("node-1", 100, &store, &leases).expect("dry-run plan");

        assert!(plan.blockers.is_empty());
        assert_eq!(plan.runnable.len(), 1);
        assert_eq!(plan.runnable[0].workload_id, dry_workload_id());
        assert_eq!(plan.runnable[0].argv[0], "cargo");
        assert_eq!(plan.runnable[0].argv[1], "check");
        let records = store.durable_records().expect("list records");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record.state, AgentWorkloadState::Dispatched);
    }

    #[test]
    fn dry_run_empty_store_plans_nothing() {
        let store = dry_store();
        let leases = MapLeases {
            entries: HashMap::new(),
            fail: Vec::new(),
        };

        let plan = plan_pull_run("node-1", 100, &store, &leases).expect("dry-run plan");

        assert!(plan.blockers.is_empty());
        assert!(plan.runnable.is_empty());
    }

    struct FakeCommandRunner {
        outputs: std::sync::Mutex<Vec<CommandOutput>>,
        seen: std::sync::Mutex<Vec<Vec<String>>>,
    }

    impl FakeCommandRunner {
        fn with(outputs: Vec<CommandOutput>) -> Self {
            Self {
                outputs: std::sync::Mutex::new(outputs.into_iter().rev().collect()),
                seen: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn seen_argv(&self) -> Vec<Vec<String>> {
            self.seen.lock().expect("read seen argv").clone()
        }
    }

    impl crate::ports::build_runner_adapter::CommandRunner for FakeCommandRunner {
        fn run(&self, argv: &[&str]) -> CommandOutput {
            self.seen
                .lock()
                .expect("record argv")
                .push(argv.iter().map(|arg| (*arg).to_string()).collect());
            self.outputs
                .lock()
                .expect("pop output")
                .pop()
                .unwrap_or_default()
        }
    }

    fn ok_output(stdout: &str) -> CommandOutput {
        CommandOutput {
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
            spawn_error: None,
        }
    }

    fn fail_output() -> CommandOutput {
        CommandOutput {
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "error[E0432]".to_string(),
            spawn_error: None,
        }
    }

    fn leased_holder(store_id: &str) -> MapLeases {
        let mut entries = HashMap::new();
        entries.insert(store_id.to_string(), lease("node-1", 999));
        MapLeases {
            entries,
            fail: Vec::new(),
        }
    }

    #[test]
    fn executes_held_workload_to_completed() {
        let store = dry_store();
        admit_build_check(
            DRY_COMMIT,
            "cynic-host",
            "cynic-builder",
            "node-1",
            "agent-1",
            100,
            &store,
        )
        .expect("admit");
        let runner = FakeCommandRunner::with(vec![ok_output("Checking cynic-host")]);

        let report = execute_pull_run(
            "node-1",
            100,
            200,
            1,
            &store,
            &leased_holder(&dry_workload_id()),
            &runner,
        );

        assert!(report.blockers.is_empty());
        assert_eq!(report.ran.len(), 1);
        assert!(report.ran[0].success);
        assert_eq!(report.ran[0].state, AgentWorkloadState::Completed);
        assert_eq!(report.ran[0].stdout_tail, "Checking cynic-host");
        let seen = runner.seen_argv();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0][0], "cargo");
        let stored = store
            .read(&dry_workload_id())
            .expect("read back")
            .expect("record present");
        assert_eq!(stored.record.state, AgentWorkloadState::Completed);
    }

    #[test]
    fn failing_exit_records_failed_state() {
        let store = dry_store();
        admit_build_check(
            DRY_COMMIT,
            "cynic-host",
            "cynic-builder",
            "node-1",
            "agent-1",
            100,
            &store,
        )
        .expect("admit");
        let runner = FakeCommandRunner::with(vec![fail_output()]);

        let report = execute_pull_run(
            "node-1",
            100,
            200,
            1,
            &store,
            &leased_holder(&dry_workload_id()),
            &runner,
        );

        assert!(report.blockers.is_empty());
        assert_eq!(report.ran.len(), 1);
        assert!(!report.ran[0].success);
        assert_eq!(report.ran[0].state, AgentWorkloadState::Failed);
    }

    #[test]
    fn second_run_skips_terminal_without_respawning() {
        let store = dry_store();
        admit_build_check(
            DRY_COMMIT,
            "cynic-host",
            "cynic-builder",
            "node-1",
            "agent-1",
            100,
            &store,
        )
        .expect("admit");
        let leases = leased_holder(&dry_workload_id());
        let first_runner = FakeCommandRunner::with(vec![ok_output("ok")]);
        let first = execute_pull_run("node-1", 100, 200, 1, &store, &leases, &first_runner);
        assert_eq!(first.ran.len(), 1);

        let second_runner = FakeCommandRunner::with(vec![ok_output("ok")]);
        let second = execute_pull_run("node-1", 100, 300, 1, &store, &leases, &second_runner);

        assert!(second.ran.is_empty());
        assert!(second_runner.seen_argv().is_empty());
    }
}
