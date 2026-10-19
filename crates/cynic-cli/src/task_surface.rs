//! Task ledger projection surface: `cynic task list/add/status`.
//!
//! Projects `infra/task-ledger.json` (kairotic + operational opportunities).
//! Pure logic lives in [`crate::task_ledger`]; this module owns I/O and clap.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Subcommand;

use crate::emit_json;
use crate::rust_source;
use crate::task_ledger::{
    TASK_LEDGER_SCHEMA, TaskEntry, TaskLedger, add_task, filter_tasks, generate_task_id,
    update_status,
};

#[derive(Subcommand)]
pub(crate) enum TaskCommand {
    /// List tasks in the ledger (with optional filters).
    List {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        /// Path to task ledger (default: infra/task-ledger.json).
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
    /// Add a new task to the ledger.
    Add {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "medium")]
        priority: String,
        #[arg(long, default_value = "operational")]
        kind: String,
        #[arg(long, default_value = "open")]
        status: String,
        #[arg(long)]
        link: Option<String>,
        #[arg(long)]
        task_id: Option<String>,
        /// Path to task ledger (default: infra/task-ledger.json).
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
    /// Update a task's status.
    Status {
        #[arg(long)]
        json: bool,
        /// Task id to update.
        #[arg(long)]
        task_id: String,
        /// New status.
        #[arg(long)]
        status: String,
        /// Path to task ledger (default: infra/task-ledger.json).
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
}

pub(crate) fn execute_task(command: TaskCommand) -> ExitCode {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    match command {
        TaskCommand::List {
            json,
            kind,
            status,
            priority,
            ledger,
        } => {
            let path = resolve_ledger_path(ledger.as_deref());
            let ledger_doc = match read_ledger(&path) {
                Ok(l) => l,
                Err(e) => return fail_msg(&e),
            };
            let filtered = filter_tasks(
                &ledger_doc,
                kind.as_deref(),
                status.as_deref(),
                priority.as_deref(),
            );
            let entries: Vec<TaskEntry> = filtered.into_iter().cloned().collect();
            if json {
                let output = serde_json::json!({
                    "schema": TASK_LEDGER_SCHEMA,
                    "total_tasks": entries.len(),
                    "tasks": entries,
                    "ledger": path.display().to_string(),
                });
                return emit_json(&output, false);
            }
            if entries.is_empty() {
                println!("no tasks");
                return ExitCode::SUCCESS;
            }
            for t in &entries {
                let link = t.link.clone().unwrap_or_default();
                println!(
                    "{} [{}] {} ({}) kind={} status={} link={}",
                    t.task_id, t.priority, t.title, t.kind, t.kind, t.status, link
                );
            }
            ExitCode::SUCCESS
        }
        TaskCommand::Add {
            json,
            title,
            priority,
            kind,
            status,
            link,
            task_id,
            ledger,
        } => {
            let path = resolve_ledger_path(ledger.as_deref());
            let mut ledger_doc = read_ledger(&path).unwrap_or_else(|_| TaskLedger::new_empty());
            let now = now_rfc3339();
            let id = task_id.unwrap_or_else(|| generate_task_id(&ledger_doc.tasks, now_unix_s()));
            let entry = TaskEntry {
                task_id: id.clone(),
                title: title.clone(),
                priority: priority.clone(),
                status: status.clone(),
                kind: kind.clone(),
                link: link.clone(),
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            if let Err(e) = add_task(&mut ledger_doc, entry.clone()) {
                return fail_msg(&e.to_string());
            }
            if let Err(e) = write_ledger(&path, &ledger_doc) {
                return fail_msg(&e);
            }
            if json {
                return emit_json(&entry, false);
            }
            println!("added task {}", id);
            ExitCode::SUCCESS
        }
        TaskCommand::Status {
            json,
            task_id,
            status,
            ledger,
        } => {
            let path = resolve_ledger_path(ledger.as_deref());
            let mut ledger_doc = match read_ledger(&path) {
                Ok(l) => l,
                Err(e) => return fail_msg(&e),
            };
            let now = now_rfc3339();
            let updated = match update_status(&mut ledger_doc, &task_id, &status, &now) {
                Ok(t) => t,
                Err(e) => return fail_msg(&e.to_string()),
            };
            if let Err(e) = write_ledger(&path, &ledger_doc) {
                return fail_msg(&e);
            }
            if json {
                return emit_json(&updated, false);
            }
            println!("updated {} -> {}", task_id, status);
            ExitCode::SUCCESS
        }
    }
}

fn resolve_ledger_path(explicit: Option<&Path>) -> PathBuf {
    if let Some(p) = explicit {
        return p.to_path_buf();
    }
    if let Ok(env) = std::env::var("CYNIC_TASK_LEDGER") {
        if !env.trim().is_empty() {
            return PathBuf::from(env);
        }
    }
    rust_source::workspace_root().join("infra/task-ledger.json")
}

fn read_ledger(path: &Path) -> Result<TaskLedger, String> {
    if !path.exists() {
        return Ok(TaskLedger::new_empty());
    }
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("cannot read ledger {}: {e}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(TaskLedger::new_empty());
    }
    let mut doc: TaskLedger = serde_json::from_str(&raw)
        .map_err(|e| format!("invalid ledger {}: {e}", path.display()))?;
    if doc.schema.is_empty() {
        doc.schema = TASK_LEDGER_SCHEMA.to_string();
    }
    if doc.status.is_empty() {
        doc.status = "seed".to_string();
    }
    Ok(doc)
}

fn write_ledger(path: &Path, doc: &TaskLedger) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create dir {}: {e}", parent.display()))?;
        }
    }
    let payload = serde_json::to_string_pretty(doc).map_err(|e| format!("serialize error: {e}"))?;
    fs::write(path, payload).map_err(|e| format!("cannot write ledger {}: {e}", path.display()))
}

fn fail_msg(msg: &str) -> ExitCode {
    eprintln!("{msg}");
    ExitCode::FAILURE
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}
