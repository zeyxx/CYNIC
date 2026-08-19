//! `cynic source` CLI surface.
//!
//! Layer 1 (Cortex → Forgejo) : `sync`
//! Layer 2 (Forgejo → GitHub)  : `release`
//! Security                  : `lint`
//! Cleanup                   : `gc`

use std::process::ExitCode;

use clap::Subcommand;
use serde::Serialize;

use crate::cli_json::emit_pretty_json;

#[derive(Debug, Subcommand)]
pub(crate) enum SourceCommand {
    /// Observe + reconcile + (optionally) apply sync across layers
    Sync {
        /// Preview only — no mutation
        #[arg(long, default_value_t = true)]
        dry_run: bool,
        /// Execute the sync plan
        #[arg(long)]
        apply: bool,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Secret literal lint — scan git-tracked source
    Lint {
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Space cleanup — report + optionally reclaim
    Gc {
        /// Scope of cleanup
        #[arg(long, default_value = "repo")]
        scope: String,
        /// Preview only
        #[arg(long, default_value_t = true)]
        dry_run: bool,
        /// Apply cleanup
        #[arg(long)]
        apply: bool,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Release bot — package forgejo commits into a single github release
    Release {
        /// Preview only
        #[arg(long, default_value_t = true)]
        dry_run: bool,
        /// Execute release
        #[arg(long)]
        apply: bool,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct SourceExit {
    pub status: &'static str,
    pub summary: String,
}

impl SourceExit {
    fn print(&self, json: bool) {
        if json {
            emit_pretty_json(&self, false);
        } else {
            println!("{}", self.summary);
        }
    }
}

pub(crate) fn execute_source(command: SourceCommand) -> ExitCode {
    let root = std::env::current_dir().unwrap_or_default();

    match command {
        SourceCommand::Sync { dry_run, json, apply } => {
            execute_sync(&root, dry_run, apply, json)
        }
        SourceCommand::Lint { json } => execute_lint(&root, json),
        SourceCommand::Gc {
            scope,
            dry_run,
            apply,
            json,
        } => execute_gc(&root, &scope, dry_run, apply, json),
        SourceCommand::Release { dry_run, apply, json } => {
            execute_release(&root, dry_run, apply, json)
        }
    }
}

fn execute_sync(root: &std::path::Path, dry_run: bool, apply: bool, json: bool) -> ExitCode {
    let observation = cynic_host::source_sync::observe_all_layers(root);
    let decision = cynic_host::source_sync::reconcile(&observation);
    let plan = cynic_host::source_sync::plan(&decision);

    if dry_run || !apply {
        let exit = SourceExit {
            status: "dry_run",
            summary: format!(
                "Sync decision ({} actions, review={}):",
                plan.steps.len(),
                decision.needs_review
            ),
        };
        exit.print(json);
        println!();
        for (i, step) in plan.steps.iter().enumerate() {
            let argv = match step {
                cynic_host::source_sync::types::SyncStep::GitCommand { argv, layer: _ } => {
                    argv.join(" ")
                }
                cynic_host::source_sync::types::SyncStep::WriteFile { path, content } => {
                    format!("write {path} ({} bytes)", content.len())
                }
                cynic_host::source_sync::types::SyncStep::Archive {
                    branch,
                    archive_ref,
                } => {
                    format!("archive {branch} → {archive_ref}")
                }
            };
            println!("  [{}] {argv}", i + 1);
        }
        if !decision.unknowns.is_empty() {
            println!();
            for u in &decision.unknowns {
                println!("  ? {u}");
            }
        }
        return ExitCode::SUCCESS;
    }

    if decision.needs_review {
        let exit = SourceExit {
            status: "needs_review",
            summary: "sync requires human review — add --confirm to override".to_string(),
        };
        exit.print(json);
        return ExitCode::from(2);
    }

    let proof = cynic_host::source_sync::execute_plan(root, &plan);
    let exit = SourceExit {
        status: if proof.success { "applied" } else { "failed" },
        summary: format!(
            "sync {} — {} steps executed",
            if proof.success { "succeeded" } else { "failed" },
            proof.steps_completed.len()
        ),
    };
    exit.print(json);
    if proof.success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn execute_lint(root: &std::path::Path, json: bool) -> ExitCode {
    let observation = cynic_host::secret_lint::scan_for_secrets(root);

    if json {
        emit_pretty_json(&observation, observation.has_findings());
        if observation.has_findings() {
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    if observation.has_findings() {
        println!("{}", observation.summary());
        for finding in &observation.findings {
            println!("  {}:{}  {}", finding.file_path, finding.line_number, finding.redacted_line);
        }
        ExitCode::FAILURE
    } else {
        println!("{}", observation.summary());
        ExitCode::SUCCESS
    }
}

fn execute_gc(
    root: &std::path::Path,
    scope: &str,
    dry_run: bool,
    apply: bool,
    json: bool,
) -> ExitCode {
    let scope = match scope {
        "repo" => cynic_host::space_gc::GcScope::Repo,
        "devops" => cynic_host::space_gc::GcScope::Devops,
        "all" => cynic_host::space_gc::GcScope::All,
        other => {
            let msg = format!("unknown scope: {other} (use repo|devops|all)");
            if json {
                emit_pretty_json(&SourceExit { status: "error", summary: msg.clone() }, true);
            } else {
                eprintln!("{}", msg);
            }
            return ExitCode::from(2);
        }
    };

    let observation = cynic_host::space_gc::observe_space_usage(root, scope);

    if dry_run || !apply {
        if json {
            emit_pretty_json(&observation, false);
        } else {
            println!("Space usage (scope={scope}):");
            println!("  total: {} ({:.1} GiB)", observation.total_bytes, observation.total_bytes as f64 / 1.073741824e9);
            for target in &observation.targets {
                println!(
                    "  {}: {} ({:.1} GiB, {} files)",
                    target.path,
                    target.bytes,
                    target.bytes as f64 / 1.073741824e9,
                    target.file_count
                );
            }
        }
        return ExitCode::SUCCESS;
    }

    let mut freed: u64 = 0;
    for target in &observation.targets {
        let dir = root.join(&target.path);
        if dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&dir) {
                eprintln!("failed to remove {}: {e}", target.path);
            } else {
                freed += target.bytes;
            }
        }
    }

    let exit = SourceExit {
        status: "applied",
        summary: format!(
            "freed {} ({:.1} GiB)",
            freed,
            freed as f64 / 1.073741824e9
        ),
    };
    if json {
        emit_pretty_json(&exit, false);
    } else {
        println!("{}", exit.summary);
    }
    ExitCode::SUCCESS
}

fn execute_release(
    root: &std::path::Path,
    _dry_run: bool,
    _apply: bool,
    json: bool,
) -> ExitCode {
    let observation = cynic_host::source_sync::observe_all_layers(root);

    let exit = SourceExit {
        status: "blocked",
        summary: format!(
            "forgejo ahead of github by {} commits — release bot requires github auth setup",
            observation.forgejo.commits_ahead.unwrap_or(0)
        ),
    };
    if json {
        emit_pretty_json(&exit, false);
    } else {
        println!("{}", exit.summary);
    }
    ExitCode::SUCCESS
}
