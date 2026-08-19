//! Domain Task Queue CLI Surface (`cynic task`).
//!
//! Exposes push, claim, list, complete, and fail operations for domain-partitioned agent workloads.

#![allow(dead_code)]
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Subcommand;
use cynic_inference::{DomainTaskQueue, DomainTaskQueueError};
use cynic_protocol::{
    DomainTask, DomainTaskStatus, TaskDomain, TaskPriority, DOMAIN_TASK_LIST_SCHEMA,
};

#[derive(Subcommand)]
pub(crate) enum TaskCommand {
    /// Push a new structured task to a domain queue.
    Push {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        domain: String,
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "agent")]
        priority: String,
        #[arg(long, default_value = "{}")]
        payload: String,
        #[arg(long, default_value = "cli_operator")]
        agent: String,
        #[arg(long)]
        queue_path: Option<PathBuf>,
    },
    /// List tasks in the domain queue with optional filters.
    List {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        queue_path: Option<PathBuf>,
    },
    /// Claim the highest-priority pending task in a domain.
    Claim {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        domain: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        queue_path: Option<PathBuf>,
    },
    /// Complete a task with result evidence payload.
    Complete {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        agent: String,
        #[arg(long, default_value = "{}")]
        evidence: String,
        #[arg(long)]
        queue_path: Option<PathBuf>,
    },
    /// Mark a task as failed with an explanation.
    Fail {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        queue_path: Option<PathBuf>,
    },
}

pub(crate) fn execute_task(command: TaskCommand) -> ExitCode {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    match command {
        TaskCommand::Push {
            json,
            domain,
            title,
            priority,
            payload,
            agent,
            queue_path,
        } => {
            if !json {
                eprintln!("cynic task push requires --json");
                return ExitCode::FAILURE;
            }

            let task_domain = match parse_domain(&domain) {
                Some(d) => d,
                None => {
                    eprintln!("Invalid domain '{domain}' — options: inference, infra, code_audit, cortex, governance, nightshift");
                    return ExitCode::FAILURE;
                }
            };

            let task_priority = parse_priority(&priority);
            let payload_val: serde_json::Value =
                serde_json::from_str(&payload).unwrap_or(serde_json::json!({}));

            let queue = match open_queue(queue_path) {
                Ok(q) => q,
                Err(e) => return emit_error(&e),
            };

            let result = rt.block_on(async {
                queue
                    .push_task(task_domain, task_priority, title, payload_val, agent)
                    .await
            });

            match result {
                Ok(task) => emit_success(&task),
                Err(e) => emit_error(&e),
            }
        }
        TaskCommand::List {
            json,
            domain,
            status,
            queue_path,
        } => {
            if !json {
                eprintln!("cynic task list requires --json");
                return ExitCode::FAILURE;
            }

            let domain_filter = domain.as_deref().and_then(parse_domain);
            let status_filter = status.as_deref().and_then(parse_status);

            let queue = match open_queue(queue_path) {
                Ok(q) => q,
                Err(e) => return emit_error(&e),
            };

            let tasks = rt.block_on(async { queue.list_tasks(domain_filter, status_filter).await });

            let output = serde_json::json!({
                "schema": DOMAIN_TASK_LIST_SCHEMA,
                "total_tasks": tasks.len(),
                "tasks": tasks,
            });

            println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
            ExitCode::SUCCESS
        }
        TaskCommand::Claim {
            json,
            domain,
            agent,
            queue_path,
        } => {
            if !json {
                eprintln!("cynic task claim requires --json");
                return ExitCode::FAILURE;
            }

            let task_domain = match parse_domain(&domain) {
                Some(d) => d,
                None => {
                    eprintln!("Invalid domain '{domain}'");
                    return ExitCode::FAILURE;
                }
            };

            let queue = match open_queue(queue_path) {
                Ok(q) => q,
                Err(e) => return emit_error(&e),
            };

            let result = rt.block_on(async { queue.claim_task(task_domain, agent).await });

            match result {
                Ok(Some(task)) => emit_success(&task),
                Ok(None) => {
                    let output = serde_json::json!({
                        "status": "empty",
                        "domain": domain,
                        "message": "No pending tasks found for domain"
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => emit_error(&e),
            }
        }
        TaskCommand::Complete {
            json,
            task_id,
            agent,
            evidence,
            queue_path,
        } => {
            if !json {
                eprintln!("cynic task complete requires --json");
                return ExitCode::FAILURE;
            }

            let evidence_val: serde_json::Value =
                serde_json::from_str(&evidence).unwrap_or(serde_json::json!({}));

            let queue = match open_queue(queue_path) {
                Ok(q) => q,
                Err(e) => return emit_error(&e),
            };

            let result = rt.block_on(async {
                queue.complete_task(&task_id, &agent, evidence_val).await
            });

            match result {
                Ok(task) => emit_success(&task),
                Err(e) => emit_error(&e),
            }
        }
        TaskCommand::Fail {
            json,
            task_id,
            reason,
            queue_path,
        } => {
            if !json {
                eprintln!("cynic task fail requires --json");
                return ExitCode::FAILURE;
            }

            let queue = match open_queue(queue_path) {
                Ok(q) => q,
                Err(e) => return emit_error(&e),
            };

            let result = rt.block_on(async { queue.fail_task(&task_id, reason).await });

            match result {
                Ok(task) => emit_success(&task),
                Err(e) => emit_error(&e),
            }
        }
    }
}

fn open_queue(path: Option<PathBuf>) -> Result<DomainTaskQueue, DomainTaskQueueError> {
    match path {
        Some(p) => DomainTaskQueue::open(p),
        None => DomainTaskQueue::open_default(),
    }
}

fn emit_success(task: &DomainTask) -> ExitCode {
    println!("{}", serde_json::to_string_pretty(task).unwrap_or_default());
    ExitCode::SUCCESS
}

fn emit_error(err: &DomainTaskQueueError) -> ExitCode {
    let output = serde_json::json!({
        "status": "error",
        "error": err.to_string()
    });
    eprintln!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
    ExitCode::FAILURE
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

fn parse_priority(s: &str) -> TaskPriority {
    match s.to_lowercase().as_str() {
        "critical" | "emergency" => TaskPriority::Critical,
        "user" => TaskPriority::User,
        "nightshift" => TaskPriority::Nightshift,
        "background" => TaskPriority::Background,
        _ => TaskPriority::Agent,
    }
}

fn parse_status(s: &str) -> Option<DomainTaskStatus> {
    match s.to_lowercase().as_str() {
        "pending" => Some(DomainTaskStatus::Pending),
        "claimed" => Some(DomainTaskStatus::Claimed),
        "running" => Some(DomainTaskStatus::Running),
        "completed" => Some(DomainTaskStatus::Completed),
        "failed" => Some(DomainTaskStatus::Failed),
        "cancelled" => Some(DomainTaskStatus::Cancelled),
        _ => None,
    }
}
