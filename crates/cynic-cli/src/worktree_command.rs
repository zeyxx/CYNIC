use std::process::ExitCode;

use clap::Subcommand;
use serde::Serialize;

#[derive(Subcommand)]
pub enum WorktreeCommand {
    /// Observe worktree inventory (W1-A + W1-B)
    Inventory {
        #[arg(long, default_value = "/var/lib/cynic/source/repositories/CYNIC")]
        repository_root: String,
        #[arg(long, default_value = "cynic")]
        repository_id: String,
        #[arg(long, default_value = "builder-a")]
        node_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Preflight workspace creation (admission)
    Preflight {
        #[arg(long, default_value = "/var/lib/cynic/source/repositories/CYNIC")]
        source_root: String,
        #[arg(long, required = true)]
        target_root: String,
        #[arg(long, required = true)]
        base_commit: String,
        #[arg(long, default_value = "/var/lib/cynic/source/workspaces")]
        workspace_store: String,
        #[arg(long)]
        json: bool,
    },
    /// Reconcile workspace lifecycle (W2)
    Reconcile {
        #[arg(long)]
        workload_id: String,
        #[arg(long, default_value = "/var/lib/cynic/source/repositories/CYNIC")]
        repository_root: String,
        #[arg(long, default_value = "builder-a")]
        node_id: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Serialize)]
struct InventoryObservation {
    schema: String,
    repository_id: String,
    node_id: String,
    entries: Vec<InventoryEntry>,
}

#[derive(Serialize)]
struct InventoryEntry {
    candidate_id: String,
    path_basis: String,
    relative_path: Option<String>,
    head: Option<String>,
    branch: Option<String>,
    locked: Option<bool>,
    unknowns: Vec<String>,
}

pub(crate) fn execute_worktree(cmd: WorktreeCommand) -> ExitCode {
    match cmd {
        WorktreeCommand::Inventory {
            repository_root,
            repository_id,
            node_id,
            json,
        } => {
            let observation =
                cynic_host::source_worktree_inventory::observe_source_worktree_inventory(
                    std::path::Path::new(&repository_root),
                    &repository_id,
                    &node_id,
                    &cynic_host::update_lifecycle::LocalArgvRunner,
                );
            let result = InventoryObservation {
                schema: observation.schema,
                repository_id: observation.repository_id,
                node_id: observation.node_id,
                entries: observation
                    .entries
                    .iter()
                    .map(|e| InventoryEntry {
                        candidate_id: e.candidate_id.clone(),
                        path_basis: format!("{:?}", e.path_basis),
                        relative_path: e.relative_path.clone(),
                        head: e.head.clone(),
                        branch: e.branch.clone(),
                        locked: e.locked,
                        unknowns: e.unknowns.iter().map(|u| format!("{u:?}")).collect(),
                    })
                    .collect(),
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            } else {
                println!("Worktree Inventory ({} entries):", result.entries.len());
                for entry in &result.entries {
                    println!(
                        "  {} [{}] {}",
                        &entry.candidate_id[..16.min(entry.candidate_id.len())],
                        entry.path_basis,
                        entry.relative_path.as_deref().unwrap_or("(main)")
                    );
                    if let Some(head) = &entry.head {
                        println!("    HEAD: {}", &head[..8.min(head.len())]);
                    }
                    if let Some(branch) = &entry.branch {
                        println!("    Branch: {branch}");
                    }
                    if entry.locked == Some(true) {
                        println!("    LOCKED");
                    }
                    if !entry.unknowns.is_empty() {
                        println!("    Unknowns: {}", entry.unknowns.join(", "));
                    }
                }
            }
            ExitCode::SUCCESS
        }
        WorktreeCommand::Preflight {
            source_root,
            target_root,
            base_commit,
            workspace_store,
            json,
        } => {
            let report = cynic_host::workspace_preflight::workspace_preflight(
                std::path::Path::new(&source_root),
                std::path::Path::new(&target_root),
                &base_commit,
                std::path::Path::new(&workspace_store),
            );
            if json {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else {
                if report.admitted {
                    println!("ADMITTED");
                } else {
                    println!("DENIED:");
                    for b in &report.blockers {
                        println!("  - {b}");
                    }
                }
            }
            ExitCode::SUCCESS
        }
        WorktreeCommand::Reconcile {
            workload_id,
            repository_root,
            node_id,
            json,
        } => {
            let observation =
                cynic_host::source_worktree_inventory::observe_source_worktree_inventory(
                    std::path::Path::new(&repository_root),
                    "cynic",
                    &node_id,
                    &cynic_host::update_lifecycle::LocalArgvRunner,
                );
            let exists = !observation.entries.is_empty();
            let workspace_clean = if exists {
                Some(observation.entries.iter().all(|e| e.clean.unwrap_or(false)))
            } else {
                None
            };
            let lifecycle_obs = serde_json::json!({
                "schema": "cynic.agent_workspace_lifecycle_observation.v1",
                "workload_id": workload_id,
                "workload_state": "active",
                "workload_claim": null,
                "workspace": {
                    "schema": "cynic.agent_workspace_observation.v1",
                    "repository_id": "cynic",
                    "workspace_id": "worktree-w2",
                    "node_id": node_id,
                    "exists": exists,
                    "isolated": observation.entries.len() <= 1,
                    "head_commit": observation.entries.first().and_then(|e| e.head.clone()),
                    "path_preimages": [],
                    "repository_claims": [],
                    "evidence_refs": [],
                },
                "completion_evidence": null,
                "workspace_clean": workspace_clean,
                "retention_until_ms": null,
                "protected_roots": [],
                "collection_claim": null,
            });
            if json {
                println!("{}", serde_json::to_string_pretty(&lifecycle_obs).unwrap());
            } else {
                println!("Workload: {workload_id}");
                println!("Exists: {exists}");
                println!("Workspace clean: {workspace_clean:?}");
            }
            ExitCode::SUCCESS
        }
    }
}
