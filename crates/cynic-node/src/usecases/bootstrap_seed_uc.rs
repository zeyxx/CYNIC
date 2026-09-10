use cynic_protocol::{
    DevelopmentGenesisBuildPlan, NodeExecutionMode, NodeExecutionSurface,
    node_execution_mode_allows,
};
use std::{collections::BTreeMap, path::Path, time::Duration};
use tokio_util::sync::CancellationToken;

use crate::ports::build_scheduler_port::{
    BuildExecutionError, BuildExecutionPort, BuildExecutionReadBack, BuildExecutionRequest,
};

pub const BOOTSTRAP_SEED_RECEIPT_SCHEMA: &str = "cynic.node.bootstrap_seed.receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapSeedExecutionDecision {
    Ready { argv: Vec<Vec<String>> },
    Blocked { blockers: Vec<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapSeedStatus {
    Proved,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapSeedStep {
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Node-owned proof of one bounded seed run. It retains the full Development
/// Genesis lineage plus the exact executed read-back; it never contains secret
/// material because the execution environment was empty by construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapSeedReceipt {
    pub schema: String,
    pub status: BootstrapSeedStatus,
    pub request_id: String,
    pub source_commit: String,
    pub source_tree: String,
    pub source_authority_ref: String,
    pub builder_image: String,
    pub argv_requested: Vec<Vec<String>>,
    pub steps: Vec<BootstrapSeedStep>,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
}

pub fn decide_bootstrap_seed_execution(
    execution_mode: NodeExecutionMode,
    plan: DevelopmentGenesisBuildPlan,
) -> BootstrapSeedExecutionDecision {
    if execution_mode != NodeExecutionMode::BootstrapSeed
        || !node_execution_mode_allows(execution_mode, NodeExecutionSurface::BootstrapBuild)
    {
        return BootstrapSeedExecutionDecision::Blocked {
            blockers: vec!["bootstrap_seed_mode_required".to_string()],
        };
    }
    if plan.status == "ready" && !plan.argv.is_empty() {
        return BootstrapSeedExecutionDecision::Ready { argv: plan.argv };
    }
    let mut blockers = plan.blockers;
    blockers.extend(plan.unknowns);
    if blockers.is_empty() {
        blockers.push(format!("bootstrap_seed_plan_not_ready:{}", plan.status));
    }
    blockers.sort();
    blockers.dedup();
    BootstrapSeedExecutionDecision::Blocked { blockers }
}

pub fn bootstrap_seed_execution_requests(
    decision: &BootstrapSeedExecutionDecision,
    working_directory: &Path,
    timeout: Duration,
) -> Result<Vec<BuildExecutionRequest>, Vec<String>> {
    if !working_directory.is_absolute() {
        return Err(vec![
            "bootstrap_seed_working_directory_not_absolute".to_string(),
        ]);
    }
    if timeout.is_zero() {
        return Err(vec!["bootstrap_seed_execution_timeout_zero".to_string()]);
    }
    match decision {
        BootstrapSeedExecutionDecision::Ready { argv } => Ok(argv
            .iter()
            .cloned()
            .map(|argv| BuildExecutionRequest {
                argv,
                working_directory: working_directory.to_path_buf(),
                environment: BTreeMap::new(),
                timeout,
                sandbox: crate::ports::build_scheduler_port::BuildSandboxMode::Off,
            })
            .collect()),
        BootstrapSeedExecutionDecision::Blocked { blockers } => Err(blockers.clone()),
    }
}

pub fn bootstrap_seed_receipt(
    plan: &DevelopmentGenesisBuildPlan,
    requests: &[BuildExecutionRequest],
    outcomes: &[Result<BuildExecutionReadBack, BuildExecutionError>],
) -> BootstrapSeedReceipt {
    let mut steps = Vec::new();
    let mut blockers = Vec::new();
    let mut status = BootstrapSeedStatus::Proved;
    for (request, outcome) in requests.iter().zip(outcomes.iter()) {
        match outcome {
            Ok(read_back) => {
                steps.push(BootstrapSeedStep {
                    argv: request.argv.clone(),
                    exit_code: read_back.exit_code,
                    stdout: read_back.stdout.clone(),
                    stderr: read_back.stderr.clone(),
                });
                if read_back.exit_code != Some(0) {
                    blockers.push(format!(
                        "bootstrap_seed_build_exit_code:{:?}",
                        read_back.exit_code
                    ));
                    status = BootstrapSeedStatus::Failed;
                    break;
                }
            }
            Err(BuildExecutionError::Cancelled) => {
                blockers.push("build_execution_cancelled".to_string());
                status = BootstrapSeedStatus::Cancelled;
                break;
            }
            Err(error) => {
                blockers.push(format!("bootstrap_seed_execution_error:{error:?}"));
                status = BootstrapSeedStatus::Failed;
                break;
            }
        }
    }
    if status == BootstrapSeedStatus::Proved && steps.len() != requests.len() {
        blockers.push("bootstrap_seed_step_count_mismatch".to_string());
        status = BootstrapSeedStatus::Failed;
    }
    blockers.sort();
    blockers.dedup();

    BootstrapSeedReceipt {
        schema: BOOTSTRAP_SEED_RECEIPT_SCHEMA.to_string(),
        status,
        request_id: plan.request.request_id.clone(),
        source_commit: plan.request.source_commit.clone(),
        source_tree: plan.request.source_tree.clone(),
        source_authority_ref: plan.request.source_authority_ref.clone(),
        builder_image: plan.request.builder_image.clone(),
        argv_requested: plan.argv.clone(),
        steps,
        evidence_refs: plan.request.evidence_refs.clone(),
        blockers,
    }
}

/// Orchestrates one bounded seed run against an injected execution port.
///
/// Sequence is strict: requests run in order, the first failure, timeout, or
/// cancellation stops everything, and the receipt is built from what actually
/// happened — never from intent. No network, service, publication, or runtime
/// activation occurs here; the port owns process execution only.
pub async fn execute_bootstrap_seed(
    plan: &DevelopmentGenesisBuildPlan,
    working_directory: &Path,
    timeout: Duration,
    cancellation: &CancellationToken,
    execution: &impl BuildExecutionPort,
) -> Result<BootstrapSeedReceipt, Vec<String>> {
    if cancellation.is_cancelled() {
        return Err(vec!["bootstrap_seed_cancelled_before_start".to_string()]);
    }
    let decision = decide_bootstrap_seed_execution(NodeExecutionMode::BootstrapSeed, plan.clone());
    let requests = bootstrap_seed_execution_requests(&decision, working_directory, timeout)?;
    let mut outcomes = Vec::with_capacity(requests.len());
    for request in &requests {
        if cancellation.is_cancelled() {
            outcomes.push(Err(BuildExecutionError::Cancelled));
            break;
        }
        match execution.execute(request, cancellation).await {
            Ok(read_back) => {
                let failed = read_back.exit_code != Some(0);
                outcomes.push(Ok(read_back));
                if failed {
                    break;
                }
            }
            Err(error) => {
                outcomes.push(Err(error));
                break;
            }
        }
    }

    Ok(bootstrap_seed_receipt(plan, &requests, &outcomes))
}
#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::development_genesis::{
        DEVELOPMENT_GENESIS_BUILD_REQUEST_SCHEMA, DevelopmentGenesisAction,
        DevelopmentGenesisBuildAdmission, DevelopmentGenesisBuildPlan,
        DevelopmentGenesisBuildRequest,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn plan(
        status: &str,
        argv: Vec<Vec<String>>,
        blockers: Vec<String>,
    ) -> DevelopmentGenesisBuildPlan {
        DevelopmentGenesisBuildPlan {
            schema: "cynic.development_genesis.build_plan.v1".to_string(),
            status: status.to_string(),
            request_id: "seed-1".to_string(),
            request: DevelopmentGenesisBuildRequest {
                schema: DEVELOPMENT_GENESIS_BUILD_REQUEST_SCHEMA.to_string(),
                request_id: "seed-1".to_string(),
                source_commit: "a".repeat(40),
                source_tree: "b".repeat(40),
                source_authority_ref: "test:source".to_string(),
                builder_image: format!("test@sha256:{}", "c".repeat(64)),
                expires_at_unix_s: 200,
                allowed_actions: vec![DevelopmentGenesisAction::MakeCheck],
                requested_capabilities: vec!["build_artifact".to_string()],
                evidence_refs: vec![format!("source-capsule:sha256:{}", "d".repeat(64))],
            },
            admission: DevelopmentGenesisBuildAdmission {
                schema: "cynic.development_genesis.build_admission.v1".to_string(),
                status: "admitted".to_string(),
                request_id: "seed-1".to_string(),
                blockers: Vec::new(),
            },
            source_materialization: None,
            argv,
            blockers,
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn bootstrap_seed_executes_only_a_ready_plan_in_bootstrap_seed_mode() {
        let decision = decide_bootstrap_seed_execution(
            cynic_protocol::NodeExecutionMode::BootstrapSeed,
            plan(
                "ready",
                vec![vec!["make".to_string(), "check".to_string()]],
                Vec::new(),
            ),
        );

        assert_eq!(
            decision,
            BootstrapSeedExecutionDecision::Ready {
                argv: vec![vec!["make".to_string(), "check".to_string()]],
            }
        );
    }

    #[test]
    fn bootstrap_seed_refuses_unknown_or_blocked_plans_before_any_executor_port() {
        let decision = decide_bootstrap_seed_execution(
            cynic_protocol::NodeExecutionMode::BootstrapSeed,
            plan(
                "unknown",
                Vec::new(),
                vec!["source_read_back_unavailable".to_string()],
            ),
        );

        assert_eq!(
            decision,
            BootstrapSeedExecutionDecision::Blocked {
                blockers: vec!["source_read_back_unavailable".to_string()],
            }
        );
    }

    #[test]
    fn ready_seed_decision_becomes_bounded_node_execution_requests() {
        let requests = bootstrap_seed_execution_requests(
            &BootstrapSeedExecutionDecision::Ready {
                argv: vec![vec!["make".to_string(), "check".to_string()]],
            },
            std::path::Path::new("/var/lib/cynic/bootstrap/seed-1/source"),
            std::time::Duration::from_secs(60),
        )
        .expect("ready seed requests");

        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].argv, vec!["make", "check"]);
        assert_eq!(
            requests[0].working_directory,
            std::path::PathBuf::from("/var/lib/cynic/bootstrap/seed-1/source")
        );
        assert!(requests[0].environment.is_empty());
        assert_eq!(requests[0].timeout, std::time::Duration::from_secs(60));
    }

    #[test]
    fn all_zero_exit_steps_prove_a_receipt_with_full_genesis_lineage() {
        let seed_plan = plan(
            "ready",
            vec![vec!["make".to_string(), "check".to_string()]],
            Vec::new(),
        );
        let requests = bootstrap_seed_execution_requests(
            &decide_bootstrap_seed_execution(
                cynic_protocol::NodeExecutionMode::BootstrapSeed,
                seed_plan.clone(),
            ),
            std::path::Path::new("/var/lib/cynic/bootstrap/seed-1/source"),
            std::time::Duration::from_secs(60),
        )
        .expect("requests");
        let outcomes = vec![Ok(
            crate::ports::build_scheduler_port::BuildExecutionReadBack {
                exit_code: Some(0),
                stdout: "ok".to_string(),
                stderr: String::new(),
                peak_memory_bytes: None,
            },
        )];

        let receipt = bootstrap_seed_receipt(&seed_plan, &requests, &outcomes);

        assert_eq!(receipt.status, BootstrapSeedStatus::Proved);
        assert_eq!(receipt.request_id, "seed-1");
        assert_eq!(receipt.source_commit, "a".repeat(40));
        assert_eq!(receipt.source_tree, "b".repeat(40));
        assert_eq!(
            receipt.builder_image,
            format!("test@sha256:{}", "c".repeat(64))
        );
        assert_eq!(
            receipt.argv_requested,
            vec![vec!["make".to_string(), "check".to_string()]]
        );
        assert_eq!(receipt.steps.len(), 1);
        assert!(receipt.blockers.is_empty());
    }

    #[test]
    fn nonzero_exit_fails_the_receipt_at_that_step() {
        let seed_plan = plan(
            "ready",
            vec![
                vec!["make".to_string(), "check".to_string()],
                vec!["cargo".to_string()],
            ],
            Vec::new(),
        );
        let requests: Vec<crate::ports::build_scheduler_port::BuildExecutionRequest> = seed_plan
            .argv
            .iter()
            .map(
                |argv| crate::ports::build_scheduler_port::BuildExecutionRequest {
                    argv: argv.clone(),
                    working_directory: std::path::PathBuf::from("/tmp/seed"),
                    environment: Default::default(),
                    timeout: std::time::Duration::from_secs(60),
                    sandbox: crate::ports::build_scheduler_port::BuildSandboxMode::Off,
                },
            )
            .collect();
        let outcomes = vec![
            Ok(crate::ports::build_scheduler_port::BuildExecutionReadBack {
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                peak_memory_bytes: None,
            }),
            Ok(crate::ports::build_scheduler_port::BuildExecutionReadBack {
                exit_code: Some(1),
                stdout: String::new(),
                stderr: "boom".to_string(),
                peak_memory_bytes: None,
            }),
        ];

        let receipt = bootstrap_seed_receipt(&seed_plan, &requests, &outcomes);

        assert_eq!(receipt.status, BootstrapSeedStatus::Failed);
        assert!(
            receipt
                .blockers
                .contains(&"bootstrap_seed_build_exit_code:Some(1)".to_string())
        );
        assert_eq!(receipt.steps.len(), 2);
    }

    #[test]
    fn cancellation_marks_the_receipt_cancelled_never_proved() {
        let seed_plan = plan(
            "ready",
            vec![vec!["make".to_string(), "check".to_string()]],
            Vec::new(),
        );
        let requests: Vec<crate::ports::build_scheduler_port::BuildExecutionRequest> = seed_plan
            .argv
            .iter()
            .map(
                |argv| crate::ports::build_scheduler_port::BuildExecutionRequest {
                    argv: argv.clone(),
                    working_directory: std::path::PathBuf::from("/tmp/seed"),
                    environment: Default::default(),
                    timeout: std::time::Duration::from_secs(60),
                    sandbox: crate::ports::build_scheduler_port::BuildSandboxMode::Off,
                },
            )
            .collect();
        let outcomes = vec![Err(
            crate::ports::build_scheduler_port::BuildExecutionError::Cancelled,
        )];

        let receipt = bootstrap_seed_receipt(&seed_plan, &requests, &outcomes);

        assert_eq!(receipt.status, BootstrapSeedStatus::Cancelled);
        assert!(
            receipt
                .blockers
                .contains(&"build_execution_cancelled".to_string())
        );
    }

    #[test]
    fn spawn_failure_fails_without_inventing_a_step_read_back() {
        let seed_plan = plan(
            "ready",
            vec![vec!["make".to_string(), "check".to_string()]],
            Vec::new(),
        );
        let requests: Vec<crate::ports::build_scheduler_port::BuildExecutionRequest> = seed_plan
            .argv
            .iter()
            .map(
                |argv| crate::ports::build_scheduler_port::BuildExecutionRequest {
                    argv: argv.clone(),
                    working_directory: std::path::PathBuf::from("/tmp/seed"),
                    environment: Default::default(),
                    timeout: std::time::Duration::from_secs(60),
                    sandbox: crate::ports::build_scheduler_port::BuildSandboxMode::Off,
                },
            )
            .collect();
        let outcomes = vec![Err(
            crate::ports::build_scheduler_port::BuildExecutionError::Spawn(
                "no such file".to_string(),
            ),
        )];

        let receipt = bootstrap_seed_receipt(&seed_plan, &requests, &outcomes);

        assert_eq!(receipt.status, BootstrapSeedStatus::Failed);
        assert!(receipt.steps.is_empty());
        assert!(
            receipt
                .blockers
                .iter()
                .any(|blocker| blocker.starts_with("bootstrap_seed_execution_error:"))
        );
    }

    struct SequentialPort {
        outcomes: std::sync::Mutex<Vec<Result<BuildExecutionReadBack, BuildExecutionError>>>,
        calls: AtomicUsize,
    }

    impl SequentialPort {
        fn new(outcomes: Vec<Result<BuildExecutionReadBack, BuildExecutionError>>) -> Self {
            Self {
                outcomes: std::sync::Mutex::new(outcomes),
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl BuildExecutionPort for SequentialPort {
        fn execute<'a>(
            &'a self,
            _request: &'a BuildExecutionRequest,
            _cancellation: &'a CancellationToken,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<BuildExecutionReadBack, BuildExecutionError>,
                    > + Send
                    + 'a,
            >,
        > {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let mut outcomes = self.outcomes.lock().expect("outcomes lock");
            if outcomes.is_empty() {
                return Box::pin(async move {
                    Err(BuildExecutionError::Invalid(
                        "sequential_port_outcome_exhausted".to_string(),
                    ))
                });
            }
            let outcome = outcomes.remove(0);
            Box::pin(async move { outcome })
        }
    }

    fn read_back(exit_code: Option<i32>) -> BuildExecutionReadBack {
        BuildExecutionReadBack {
            exit_code,
            stdout: String::new(),
            stderr: String::new(),
            peak_memory_bytes: None,
        }
    }

    fn seed_plan() -> DevelopmentGenesisBuildPlan {
        plan(
            "ready",
            vec![
                vec!["make".to_string(), "check".to_string()],
                vec!["cargo".to_string(), "check".to_string()],
            ],
            Vec::new(),
        )
    }

    #[tokio::test]
    async fn executes_every_request_in_order_and_proves_the_receipt() {
        let port = SequentialPort::new(vec![Ok(read_back(Some(0))), Ok(read_back(Some(0)))]);
        let cancellation = CancellationToken::new();

        let receipt = execute_bootstrap_seed(
            &seed_plan(),
            Path::new("/var/lib/cynic/bootstrap/seed-1/source"),
            Duration::from_secs(60),
            &cancellation,
            &port,
        )
        .await
        .expect("proved receipt");

        assert_eq!(port.calls.load(Ordering::SeqCst), 2);
        assert_eq!(receipt.status, BootstrapSeedStatus::Proved);
        assert_eq!(receipt.steps.len(), 2);
        assert!(receipt.blockers.is_empty());
    }

    #[tokio::test]
    async fn first_failure_stops_remaining_requests_and_fails_the_receipt() {
        let port = SequentialPort::new(vec![Ok(read_back(Some(0))), Ok(read_back(Some(1)))]);
        let cancellation = CancellationToken::new();

        let receipt = execute_bootstrap_seed(
            &seed_plan(),
            Path::new("/var/lib/cynic/bootstrap/seed-1/source"),
            Duration::from_secs(60),
            &cancellation,
            &port,
        )
        .await
        .expect("failed receipt");

        assert_eq!(port.calls.load(Ordering::SeqCst), 2);
        assert_eq!(receipt.status, BootstrapSeedStatus::Failed);
        assert_eq!(receipt.steps.len(), 2);
        assert!(
            receipt
                .blockers
                .contains(&"bootstrap_seed_build_exit_code:Some(1)".to_string())
        );
    }

    #[tokio::test]
    async fn cancellation_before_start_never_touches_the_execution_port() {
        let port = SequentialPort::new(Vec::new());
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        let error = execute_bootstrap_seed(
            &seed_plan(),
            Path::new("/var/lib/cynic/bootstrap/seed-1/source"),
            Duration::from_secs(60),
            &cancellation,
            &port,
        )
        .await;

        assert_eq!(
            error.unwrap_err(),
            vec!["bootstrap_seed_cancelled_before_start".to_string()]
        );
        assert_eq!(port.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn mid_run_cancellation_marks_the_receipt_cancelled() {
        let port = SequentialPort::new(vec![
            Ok(read_back(Some(0))),
            Err(BuildExecutionError::Cancelled),
        ]);
        let cancellation = CancellationToken::new();

        let receipt = execute_bootstrap_seed(
            &seed_plan(),
            Path::new("/var/lib/cynic/bootstrap/seed-1/source"),
            Duration::from_secs(60),
            &cancellation,
            &port,
        )
        .await
        .expect("cancelled receipt");

        assert_eq!(receipt.status, BootstrapSeedStatus::Cancelled);
        assert!(
            receipt
                .blockers
                .contains(&"build_execution_cancelled".to_string())
        );
    }
}
