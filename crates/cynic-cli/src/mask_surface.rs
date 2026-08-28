//! `cynic mask` — thin harness-launch surface behind a cynic mask.
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

use std::process::ExitCode;

use clap::Subcommand;

use crate::cli_json::emit_pretty_json;
use cynic_protocol::AGENT_CITIZENSHIP_SCHEMA;

use crate::runtime_surface::RuntimeCommand;
use crate::runtime_surface::execute_runtime;

/// Agent citizenship record projected through the mask.
/// Thin re-declaration so the CLI can serialize its intent; the canonical
/// trait lives in `cynic-protocol::agent_citizenship::Citoyenneté`.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct AgentCitizenship {
    pub schema: &'static str,
    pub agent_id: String,
    pub scope: String,
    pub mode: String,
    pub budgets: AgentBudgets,
    pub residual_risk: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct AgentBudgets {
    pub actions: u64,
    pub wall_seconds: u64,
}

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
    let exit = execute_runtime(RuntimeCommand::InferenceForge {
        report: report_path.to_string(),
        alias: "auto".to_string(),
        json,
        yes,
    });
    if exit != ExitCode::SUCCESS {
        return exit;
    }
    // Hand-off to harness start if consent was given.
    if yes {
        let _ = execute_runtime(RuntimeCommand::InferenceStart {
        capability_token: None,
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
    execute_runtime(RuntimeCommand::InferenceStart {
        capability_token: None,
        alias: alias.to_string(),
        json,
        no_wait: false,
        yes,
    })
}
