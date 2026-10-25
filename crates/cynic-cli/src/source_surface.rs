//! Source layer management surface (Cortex, Forgejo, GitHub, worktrees & releases).

use clap::Subcommand;
use cynic_types::paths::CynicPaths;
use serde::Serialize;
use std::path::Path;
use std::process::ExitCode;

mod divergence;
mod mirrors;
mod pr;
mod release;
mod release_deployment_cli;
mod sync;
pub(crate) mod sync_coherence;
mod worktree;
mod worktree_allocation;

pub(crate) use pr::PrCommand;
use release::execute_release;
use release_deployment_cli::execute_deploy;
use sync::{execute_publish, execute_status, execute_sync};
use crate::audit_surface::execute_gate_receipt;
use mirrors::execute_mirrors;
use worktree::execute_worktree;
use worktree_allocation::execute_allocate_worktree;
use divergence::execute_divergence;

#[derive(Debug, Subcommand)]
pub(crate) enum SourceReceiptCommand {
    /// Verify a machine-readable gate receipt artifact.
    Verify {
        #[arg(long, default_value = "gate-receipt.json")]
        path: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum SourceCommand {
    /// Observe source layer convergence status (Cortex, Forgejo, GitHub).
    Status {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        json: bool,
    },
    /// Run release gate checks before promoting release.
    Release {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Reconcile and synchronize source layers.
    Sync {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Reconcile, rebase, and fast-forward agent source worktrees.
    Worktree {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long, default_value = "/tmp/cynic-workspaces")]
        workspace_store: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
        /// Allocate an isolated agent worktree for the given branch name
        #[arg(long)]
        allocate: Option<String>,
        /// Base ref for isolated worktree allocation (defaults to origin/main)
        #[arg(long)]
        base: Option<String>,
        /// Explicit target path for allocation (defaults to ~/.cynic/worktrees/<branch>)
        #[arg(long)]
        path: Option<String>,
    },
    /// Manage Pull Requests on the sovereign VCS provider (read from infra/registry.json).
    Pr {
        #[command(subcommand)]
        command: PrCommand,
    },
    /// Deploy built release binaries atomically to <state>/releases/<version>.
    Deploy {
        #[arg(long, default_value_t = CynicPaths::state_dir().display().to_string())]
        state_root: String,
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        restart_unit: Vec<String>,
    },
    /// Compare Forgejo main vs GitHub main SHAs (read-only).
    Divergence {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        json: bool,
    },
    /// List Forgejo push mirrors with last sync + errors (read-only).
    Mirrors {
        #[arg(long)]
        json: bool,
    },
    /// Verify a gate receipt artifact (read-only).
    Receipt {
        #[command(subcommand)]
        command: SourceReceiptCommand,
    },
    /// Plan the public projection of the private tree from contracts/publication.json.
    Publish {
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
pub(crate) struct SourceExit {
    pub(crate) status: &'static str,
    pub(crate) summary: String,
}

pub(crate) fn execute_source(command: SourceCommand) -> ExitCode {
    match command {
        SourceCommand::Status { root, json } => execute_status(Path::new(&root), json),
        SourceCommand::Release {
            root,
            dry_run,
            apply,
            json,
        } => execute_release(Path::new(&root), dry_run, apply, json),
        SourceCommand::Sync { root, apply, json } => {
            execute_sync(Path::new(&root), apply, json)
        }
        SourceCommand::Worktree {
            root,
            workspace_store,
            apply,
            json,
            allocate,
            base,
            path,
        } => {
            if let Some(branch) = allocate {
                execute_allocate_worktree(
                    Path::new(&root),
                    Path::new(&workspace_store),
                    &branch,
                    base.as_deref(),
                    path.as_deref().map(Path::new),
                    json,
                )
            } else {
                execute_worktree(Path::new(&root), Path::new(&workspace_store), apply, json, max_age_days)
            }
        }
        SourceCommand::Pr { command } => pr::execute_pr(Path::new("."), command),
        SourceCommand::Deploy {
            state_root,
            root,
            version,
            apply,
            json,
            restart_unit,
        } => execute_deploy(
            Path::new(&root),
            Path::new(&state_root),
            version.as_deref(),
            apply,
            json,
            &restart_unit,
        ),
        SourceCommand::Publish { root, json } => execute_publish(Path::new(&root), json),
        SourceCommand::Divergence { root, json } => execute_divergence(Path::new(&root), json),
        SourceCommand::Mirrors { json } => execute_mirrors(Path::new("."), json),
        SourceCommand::Receipt { command } => match command {
            SourceReceiptCommand::Verify { path, json } => execute_gate_receipt(&path, json),
        },
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cli_source_surface_builds() {
        let cmd = super::SourceCommand::Status {
            root: ".".into(),
            json: false,
        };
        let _ = super::execute_source(cmd);
    }
}
