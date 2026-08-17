//! Federation CLI Surface (`cynic federation`).
//!
//! Inspects cross-node federation health, Synapse WireGuard mesh status,
//! and orchestrates cryptographic task dispatch and cluster synchronization.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Subcommand;
use cynic_inference::DomainTaskQueue;
use cynic_node::usecases::federated_mesh_probe_uc::probe_federated_synapse_mesh;
use cynic_node::usecases::federated_task_dispatch_uc::FederatedTaskDispatcher;
use cynic_protocol::{DomainTask, TaskDomain, TaskPriority};

#[derive(Subcommand)]
pub(crate) enum FederationCommand {
    /// Read-only status and connectivity probe across the federated Synapse cluster.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Deep latency and service reachability probe across all declared nodes.
    Probe {
        #[arg(long)]
        json: bool,
    },
    /// Dispatch a domain task to a specific or automatically selected federated node.
    Dispatch {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        domain: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        target: Option<String>,
        #[arg(long, default_value = "{}")]
        payload: String,
        #[arg(long, default_value_t = 300)]
        budget_seconds: u64,
        #[arg(long, default_value = "cortex_agent")]
        agent: String,
    },
    /// Synchronize pending tasks and active leases across the Synapse mesh.
    Sync {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        queue_path: Option<PathBuf>,
    },
}

pub(crate) fn execute_federation(command: FederationCommand) -> ExitCode {
    let dispatcher = FederatedTaskDispatcher::new("local-cortex");

    match command {
        FederationCommand::Status { json } | FederationCommand::Probe { json } => {
            if !json {
                eprintln!("cynic federation status/probe requires --json");
                return ExitCode::FAILURE;
            }

            let report = probe_federated_synapse_mesh(&dispatcher.local_node_id);
            println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            ExitCode::SUCCESS
        }
        FederationCommand::Dispatch {
            json,
            domain,
            title,
            target,
            payload,
            budget_seconds,
            agent,
        } => {
            if !json {
                eprintln!("cynic federation dispatch requires --json");
                return ExitCode::FAILURE;
            }

            let task_domain = match parse_domain(&domain) {
                Some(d) => d,
                None => {
                    eprintln!("Invalid domain '{domain}'");
                    return ExitCode::FAILURE;
                }
            };

            let payload_val: serde_json::Value =
                serde_json::from_str(&payload).unwrap_or(serde_json::json!({}));

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let task_id = format!("fed_task_{:?}_{}", task_domain, now).to_lowercase();
            let task = DomainTask::new(
                task_id,
                task_domain,
                TaskPriority::Agent,
                title,
                payload_val,
                agent,
            );

            match dispatcher.dispatch(task, target, budget_seconds) {
                Ok(receipt) => {
                    println!("{}", serde_json::to_string_pretty(&receipt).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("federation dispatch error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        FederationCommand::Sync { json, queue_path } => {
            if !json {
                eprintln!("cynic federation sync requires --json");
                return ExitCode::FAILURE;
            }

            let queue = match queue_path {
                Some(p) => DomainTaskQueue::open(p),
                None => DomainTaskQueue::open_default(),
            };

            let queue = match queue {
                Ok(q) => q,
                Err(e) => {
                    eprintln!("federation sync: cannot open domain task queue: {e}");
                    return ExitCode::FAILURE;
                }
            };

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let pending_tasks = rt.block_on(async {
                queue.list_tasks(None, Some(cynic_protocol::DomainTaskStatus::Pending)).await
            });

            let report = dispatcher.sync_cluster(&pending_tasks);
            println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            ExitCode::SUCCESS
        }
    }
}

fn parse_domain(s: &str) -> Option<TaskDomain> {
    match s.to_lowercase().as_str() {
        "inference" | "llm" => Some(TaskDomain::Inference),
        "infra" | "host" => Some(TaskDomain::Infra),
        "code_audit" | "code" | "audit" => Some(TaskDomain::CodeAudit),
        "cortex" | "epistemic" => Some(TaskDomain::Cortex),
        "governance" | "solana" => Some(TaskDomain::Governance),
        "nightshift" | "background" => Some(TaskDomain::Nightshift),
        _ => None,
    }
}
