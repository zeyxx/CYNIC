//! Unified mask/judge/judgment surface.
//!
//! A *mask* is a bounded projection of agent citizenship: a declared identity,
//! an authority scope, and monotonic budgets handed to the local agent harness
//! through the same `desired state → observe → reconcile → bounded action →
//! read-back proof` coherence pattern used at every other surface.
//!
//! Orchestrator only — does NOT own domain policy:
//!  - forge profile derivation lives in `cynic-inference::profile_bootstrap`
//!  - harness start delegates to `cynic runtime inference-start`
//!  - citizenship types are canonical in `cynic-protocol::agent_citizenship`

use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Subcommand;

use crate::cli_json::emit_pretty_json;
use cynic_protocol::AGENT_CITIZENSHIP_SCHEMA;

use crate::inference_surface::{InferenceCommand, execute_inference_command};

mod render;
mod scan;
mod types;

pub(crate) use types::{AgentBudgets, AgentCitizenship};

// ── Mask commands ───────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub(crate) enum MaskCommand {
    /// Forge a vault profile from a BenchmarkReport, then launch the harness
    /// behind the resulting mask (alias). Hand-off is explicit: the profile is
    /// written, then `inference-start --yes` is dispatched against it.
    Forge {
        report: String,
        #[arg(long, default_value = "")]
        source: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Start a local agent harness under a cynic mask (alias + citizenship).
    Start {
        #[arg(default_value = "auto")]
        alias: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
    },
}

pub(crate) fn execute_mask(command: MaskCommand) -> ExitCode {
    match command {
        MaskCommand::Forge {
            report,
            source,
            json,
            yes,
        } => execute_mask_forge(&report, &source, json, yes),
        MaskCommand::Start { alias, json, yes } => execute_mask_start(alias.as_str(), json, yes),
    }
}

fn execute_mask_forge(report_path: &str, _source_note: &str, json: bool, yes: bool) -> ExitCode {
    let exit = execute_inference_command(InferenceCommand::Forge {
        report: report_path.to_string(),
        alias: "auto".to_string(),
        json,
        yes,
    });
    if exit != ExitCode::SUCCESS {
        return exit;
    }
    if yes {
        let _ = execute_inference_command(InferenceCommand::Start {
            alias: "auto".to_string(),
            json,
            no_wait: false,
            yes,
        });
    }
    if json {
        let citizenship = AgentCitizenship {
            schema: AGENT_CITIZENSHIP_SCHEMA,
            agent_id: "mask-forge".to_string(),
            scope: "node".to_string(),
            mode: "baseline".to_string(),
            budgets: AgentBudgets {
                actions: 1000,
                wall_seconds: 3600,
            },
            residual_risk: Some(
                "local-only mask; provider/transport adapters injected at runtime".to_string(),
            ),
        };
        let _ = emit_pretty_json(&citizenship);
    }
    exit
}

fn execute_mask_start(alias: &str, json: bool, yes: bool) -> ExitCode {
    if !yes {
        eprintln!("refusing to start — mutation requires explicit `--yes`");
        return ExitCode::FAILURE;
    }
    let citizenship = AgentCitizenship {
        schema: AGENT_CITIZENSHIP_SCHEMA,
        agent_id: alias.to_string(),
        scope: "node".to_string(),
        mode: "baseline".to_string(),
        budgets: AgentBudgets {
            actions: 1000,
            wall_seconds: 3600,
        },
        residual_risk: Some(
            "local-only mask; provider/transport adapters injected at runtime".to_string(),
        ),
    };
    if json {
        let _ = emit_pretty_json(&citizenship);
    } else {
        println!(
            "starting harness behind mask '{}' (scope: {})",
            citizenship.agent_id, citizenship.scope
        );
    }
    execute_inference_command(InferenceCommand::Start {
        alias: alias.to_string(),
        json,
        no_wait: false,
        yes,
    })
}

// ── Judge commands ─────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub(crate) enum JudgeCommand {
    /// Judge an extracted patch (diff vs origin/main) and emit CynicJudgment.
    Patch {
        #[arg(long)]
        branch: String,
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        json: bool,
    },
    /// Project the runtime's current state into a single CynicJudgment.
    Runtime {
        #[arg(long)]
        json: bool,
        /// Optional subject id (defaults to local hostname).
        #[arg(long)]
        subject: Option<String>,
    },
    /// Output the content-addressed canonical digest of a judgment.
    Digest {
        #[arg(long)]
        json: bool,
        /// Optional subject id (defaults to local hostname).
        #[arg(long)]
        subject: Option<String>,
    },
}

pub(crate) fn execute_judge(command: JudgeCommand) -> ExitCode {
    match command {
        JudgeCommand::Patch { branch, root, json } => {
            let obs = scan::observe_patch(Path::new(&root), &branch);
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let input = cynic_eval::judge_patch::PatchJudgmentInput {
                observation: obs.clone(),
                judged_at_unix_s: now,
            };
            let judgment = cynic_eval::judge_patch::judge_patch(&input);
            if json {
                #[derive(serde::Serialize)]
                struct Out<'a> {
                    observation: &'a cynic_eval::judge_patch::PatchObservation,
                    judgment: &'a cynic_eval::judgment::CynicJudgment,
                }
                emit_pretty_json(&Out {
                    observation: &obs,
                    judgment: &judgment,
                })
            } else {
                println!(
                    "judge patch {}: actionable={} confidence={:.2} domain={:?} unknowns={:?}",
                    branch,
                    judgment.actionable,
                    judgment.confidence,
                    judgment.cynefin_domain,
                    judgment.residual_unknowns
                );
                println!(
                    "  diff {} bytes hash {}..",
                    obs.diff_bytes_len,
                    &obs.diff_hash[..8.min(obs.diff_hash.len())]
                );
                ExitCode::SUCCESS
            }
        }
        JudgeCommand::Runtime { json, subject } => {
            execute_judgment(JudgmentCommand::Judgment { json, subject })
        }
        JudgeCommand::Digest { json, subject } => {
            execute_judgment(JudgmentCommand::Digest { json, subject })
        }
    }
}

// ── Judgment commands ──────────────────────────────────────────────────────

#[derive(Subcommand)]
pub(crate) enum JudgmentCommand {
    /// Project the runtime's current state into a single CynicJudgment.
    #[command(name = "judgment")]
    Judgment {
        #[arg(long)]
        json: bool,
        /// Optional subject id (defaults to local hostname).
        #[arg(long)]
        subject: Option<String>,
    },
    /// Output the content-addressed canonical digest of a judgment.
    #[command(name = "digest")]
    Digest {
        #[arg(long)]
        json: bool,
        /// Optional subject id (defaults to local hostname).
        #[arg(long)]
        subject: Option<String>,
    },
}

pub(crate) fn execute_judgment(command: JudgmentCommand) -> ExitCode {
    match command {
        JudgmentCommand::Judgment { json, subject } => {
            let report = scan::observe_judgment(subject.as_deref());
            if json {
                crate::cli_json::emit_pretty_json(&report)
            } else {
                println!("{}", render::render_judgment(&report.judgment));
                println!("Canonical Digest: {}", report.digest);
                ExitCode::SUCCESS
            }
        }
        JudgmentCommand::Digest { json, subject } => {
            let report = scan::observe_judgment(subject.as_deref());
            if json {
                let payload = serde_json::json!({
                    "schema": "cynic.judgment_digest.v1",
                    "judgment_id": report.judgment.judgment_id,
                    "digest": report.digest,
                });
                crate::cli_json::emit_pretty_json(&payload)
            } else {
                println!("{}", report.digest);
                ExitCode::SUCCESS
            }
        }
    }
}
