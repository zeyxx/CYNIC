//! Idea ledger projection surface: `cynic know idea list/add/status/consume`.
//!
//! Projects `infra/idea-ledger.json`. Pure logic lives in [`crate::idea_ledger`];
//! this module owns I/O and clap.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Subcommand;

use crate::emit_json;
use crate::rust_source;
use crate::idea_ledger::{
    IDEA_LEDGER_SCHEMA, IdeaEntry, IdeaLedger, add_idea, consume, filter_ideas, generate_idea_id,
    update_freshness, update_status,
};

#[derive(Debug, Subcommand)]
pub(crate) enum IdeaCommand {
    /// List ideas in the ledger (with optional filters).
    List {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        freshness: Option<String>,
        /// Path to idea ledger (default: infra/idea-ledger.json).
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
    /// Add a new idea to the ledger.
    Add {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "")]
        description: String,
        #[arg(long, default_value = "general")]
        domain: String,
        #[arg(long, default_value = "seed")]
        status: String,
        #[arg(long, default_value = "fresh")]
        freshness: String,
        #[arg(long)]
        idea_id: Option<String>,
        /// Path to idea ledger (default: infra/idea-ledger.json).
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
    /// Update an idea's status (and optionally freshness).
    Status {
        #[arg(long)]
        json: bool,
        /// Idea id to update.
        #[arg(long)]
        idea_id: String,
        /// New status.
        #[arg(long)]
        status: Option<String>,
        /// New freshness.
        #[arg(long)]
        freshness: Option<String>,
        /// Path to idea ledger (default: infra/idea-ledger.json).
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
    /// Mark an idea as consumed by a consumer.
    Consume {
        #[arg(long)]
        json: bool,
        /// Idea id to consume.
        #[arg(long)]
        idea_id: String,
        /// Consumer identifier.
        #[arg(long)]
        consumer: String,
        /// Path to idea ledger (default: infra/idea-ledger.json).
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
}

pub(crate) fn execute_idea(command: IdeaCommand) -> ExitCode {
    match command {
        IdeaCommand::List {
            json,
            domain,
            status,
            freshness,
            ledger,
        } => {
            let path = resolve_ledger_path(ledger.as_deref());
            let ledger_doc = match read_ledger(&path) {
                Ok(l) => l,
                Err(e) => return fail_msg(&e),
            };
            let filtered = filter_ideas(
                &ledger_doc,
                domain.as_deref(),
                status.as_deref(),
                freshness.as_deref(),
            );
            let entries: Vec<IdeaEntry> = filtered.into_iter().cloned().collect();
            if json {
                let output = serde_json::json!({
                    "schema": IDEA_LEDGER_SCHEMA,
                    "total_ideas": entries.len(),
                    "ideas": entries,
                    "ledger": path.display().to_string(),
                });
                return emit_json(&output, false);
            }
            if entries.is_empty() {
                println!("no ideas");
                return ExitCode::SUCCESS;
            }
            for i in &entries {
                println!(
                    "{} [{}] {} (domain={} status={} freshness={})",
                    i.idea_id, i.status, i.title, i.domain, i.status, i.freshness
                );
            }
            ExitCode::SUCCESS
        }
        IdeaCommand::Add {
            json,
            title,
            description,
            domain,
            status,
            freshness,
            idea_id,
            ledger,
        } => {
            let path = resolve_ledger_path(ledger.as_deref());
            let mut ledger_doc = read_ledger(&path).unwrap_or_else(|_| IdeaLedger::new_empty());
            let now = now_rfc3339();
            let id = idea_id.unwrap_or_else(|| generate_idea_id(&ledger_doc.ideas, now_unix_s()));
            let entry = IdeaEntry {
                idea_id: id.clone(),
                title: title.clone(),
                description: description.clone(),
                domain: domain.clone(),
                status: status.clone(),
                freshness: freshness.clone(),
                consumed_by: Vec::new(),
                compounds: Vec::new(),
                created_at: now.clone(),
                updated_at: now.clone(),
            };
            if let Err(e) = add_idea(&mut ledger_doc, entry.clone()) {
                return fail_msg(&e.to_string());
            }
            if let Err(e) = write_ledger(&path, &ledger_doc) {
                return fail_msg(&e);
            }
            if json {
                return emit_json(&entry, false);
            }
            println!("added idea {}", id);
            ExitCode::SUCCESS
        }
        IdeaCommand::Status {
            json,
            idea_id,
            status,
            freshness,
            ledger,
        } => {
            let path = resolve_ledger_path(ledger.as_deref());
            let mut ledger_doc = match read_ledger(&path) {
                Ok(l) => l,
                Err(e) => return fail_msg(&e),
            };
            let now = now_rfc3339();
            let mut updated = None;
            if let Some(s) = status.as_deref() {
                match update_status(&mut ledger_doc, &idea_id, s, &now) {
                    Ok(u) => updated = Some(u),
                    Err(e) => return fail_msg(&e.to_string()),
                }
            }
            if let Some(f) = freshness.as_deref() {
                match update_freshness(&mut ledger_doc, &idea_id, f, &now) {
                    Ok(u) => updated = Some(u),
                    Err(e) => return fail_msg(&e.to_string()),
                }
            }
            if updated.is_none() {
                // default: no-op but read entry
                if let Some(e) = ledger_doc.ideas.iter().find(|i| i.idea_id == idea_id) {
                    updated = Some(e.clone());
                } else {
                    return fail_msg(&format!("idea not found: {idea_id}"));
                }
            }
            if let Err(e) = write_ledger(&path, &ledger_doc) {
                return fail_msg(&e);
            }
            let updated = updated.unwrap();
            if json {
                return emit_json(&updated, false);
            }
            println!("updated {} -> status={} freshness={}", idea_id, updated.status, updated.freshness);
            ExitCode::SUCCESS
        }
        IdeaCommand::Consume {
            json,
            idea_id,
            consumer,
            ledger,
        } => {
            let path = resolve_ledger_path(ledger.as_deref());
            let mut ledger_doc = match read_ledger(&path) {
                Ok(l) => l,
                Err(e) => return fail_msg(&e),
            };
            let now = now_rfc3339();
            let updated = match consume(&mut ledger_doc, &idea_id, &consumer, &now) {
                Ok(t) => t,
                Err(e) => return fail_msg(&e.to_string()),
            };
            if let Err(e) = write_ledger(&path, &ledger_doc) {
                return fail_msg(&e);
            }
            if json {
                return emit_json(&updated, false);
            }
            println!("consumed {} by {}", idea_id, consumer);
            ExitCode::SUCCESS
        }
    }
}

fn resolve_ledger_path(explicit: Option<&Path>) -> PathBuf {
    if let Some(p) = explicit {
        return p.to_path_buf();
    }
    if let Ok(env) = std::env::var("CYNIC_IDEA_LEDGER") {
        if !env.trim().is_empty() {
            return PathBuf::from(env);
        }
    }
    rust_source::workspace_root().join("infra/idea-ledger.json")
}

fn read_ledger(path: &Path) -> Result<IdeaLedger, String> {
    if !path.exists() {
        return Ok(IdeaLedger::new_empty());
    }
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("cannot read ledger {}: {e}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(IdeaLedger::new_empty());
    }
    let mut doc: IdeaLedger = serde_json::from_str(&raw)
        .map_err(|e| format!("invalid ledger {}: {e}", path.display()))?;
    if doc.schema.is_empty() {
        doc.schema = IDEA_LEDGER_SCHEMA.to_string();
    }
    if doc.status.is_empty() {
        doc.status = "seed".to_string();
    }
    Ok(doc)
}

fn write_ledger(path: &Path, doc: &IdeaLedger) -> Result<(), String> {
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
