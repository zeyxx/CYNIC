//! `cynic inference` — one object, its verbs, and no hyphen.
//!
//! Until 2026-09-28 these 16 commands lived under `cynic runtime
//! inference-*`: a verb (`runtime`) wrapping an object (`inference`) wrapping a
//! verb (`status`). `contracts/foundation.json` →
//! `no_hyphen_in_cli_commands` forbids that shape, and `clap` cannot express a
//! subcommand containing a space, so the fix is the two levels the old name was
//! asking for.
//!
//! The handlers are not moved: `runtime_surface::inference_surface` still owns
//! the behaviour, this module owns the name. A surface projects; it does not
//! own. `tests/command_taxonomy.rs` reflects the whole clap tree and fails on
//! any remaining hyphen.

use std::process::ExitCode;

use clap::Subcommand;

use crate::runtime_surface::inference_surface;

#[derive(Subcommand)]
pub(crate) enum InferenceCommand {
    /// Report the sovereign inference configuration as machine-readable JSON.
    Config {
        #[arg(long)]
        json: bool,
    },
    /// List declared local model profiles in the CYNIC vault.
    Vault {
        #[arg(long)]
        json: bool,
    },
    /// Status of the native on-demand local inference unit.
    Status {
        #[arg(long)]
        json: bool,
        /// Show the last N log lines for the running local process (requires process supervisor).
        #[arg(long, default_value_t = 0)]
        tail: usize,
    },
    /// Start a native local model profile.
    Start {
        /// Profile alias, or `auto` (default) for frictionless inventory pick.
        #[arg(default_value = "auto")]
        alias: String,
        #[arg(long)]
        json: bool,
        /// Return immediately after process spawn, and keep probing for readiness async.
        /// Useful when models are large and you want to keep the TUI responsive.
        #[arg(long)]
        no_wait: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Stop the native local inference unit.
    Stop {
        alias: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Forge a vault profile from a BenchmarkReport, then launch.
    Forge {
        #[arg(long)]
        report: String,
        #[arg(long, default_value = "auto")]
        alias: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Peer / optional ODS attach plane (probe-only by default).
    Discover {
        #[arg(long)]
        json: bool,
        /// Publish discovered peers into routing SSOT (inference.toml).
        #[arg(long)]
        apply: bool,
        /// With `--apply`, also attempt Open WebUI connection publish (opt-in).
        #[arg(long = "open-webui")]
        open_webui: bool,
    },
    /// Hardware profile observation for the local inference plane.
    Hardware {
        #[arg(long)]
        json: bool,
    },
    /// Plan the runtime binding for a model profile.
    Plan {
        alias: String,
        #[arg(long)]
        json: bool,
        /// Write runtime env + systemd drop-in from the plan.
        #[arg(long)]
        apply: bool,
    },
    /// Centralized model store: layout, vault artifacts and profiles.
    Models {
        #[command(subcommand)]
        command: ModelsCommand,
    },
    /// Hugging Face operations against the model vault, via the official `hf` CLI.
    Hf {
        #[command(subcommand)]
        command: HfCommand,
    },
    /// Inspect ChatTrace prompt+generation tokens, context meter observation, and JSONL log.
    Trace {
        #[command(subcommand)]
        command: TraceCommand,
    },
}

#[derive(Subcommand)]
pub(crate) enum TraceCommand {
    /// Status of the ChatTrace JSONL sink and cumulative tokens observed.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Tail the most recent ChatTrace records.
    Tail {
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum ModelsCommand {
    /// Layout + vault artifacts + profiles (+ optional registry write).
    Show {
        #[arg(long)]
        json: bool,
        /// Persist scan to CYNIC_MODEL_REGISTRY (default: principal model-registry.json).
        #[arg(long)]
        persist: bool,
        /// Auto-create loopback profiles for vault GGUFs that have none (frictionless).
        #[arg(long = "ensure-profiles")]
        ensure_profiles: bool,
    },
    /// Move stray GGUF weights into the principal vault and rewrite profile paths.
    Consolidate {
        #[arg(long)]
        json: bool,
        /// Apply moves (default: plan only).
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum HfCommand {
    /// Status of the Hugging Face CLI relative to model layout.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Download a Hub repo into the CYNIC model vault.
    Download {
        /// Hugging Face repo id, e.g. `org/model` (use search/suggest for short aliases).
        repo: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        revision: Option<String>,
        /// Glob include (repeatable), e.g. `*.gguf` (default `*.gguf` when registering).
        #[arg(long = "include")]
        include: Vec<String>,
        #[arg(long = "exclude")]
        exclude: Vec<String>,
        /// Repo type: model|dataset|space (default model).
        #[arg(long = "type")]
        repo_type: Option<String>,
        /// Download into hub cache only (default: into CYNIC vault).
        #[arg(long)]
        cache_only: bool,
        /// Short local alias for profile (default: derived from repo name).
        #[arg(long)]
        alias: Option<String>,
        /// Write native llama-server@ profile after download (default: true unless --no-profile).
        #[arg(long, default_value_t = true)]
        register_profile: bool,
        #[arg(long, conflicts_with = "register_profile")]
        no_profile: bool,
    },
    /// List Hub categories relevant to local models.
    Categories {
        #[arg(long)]
        json: bool,
    },
    /// Search the Hub for models.
    Search {
        #[arg(long)]
        json: bool,
        /// Category: chat|code|embedding|reasoning|vision|small|multilingual|uncensored|gguf
        #[arg(long)]
        category: Option<String>,
        /// Free-text search (optional; combined with category).
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        author: Option<String>,
        #[arg(long, default_value_t = 15)]
        limit: u32,
    },
    /// Suggest a model for the local hardware profile.
    Suggest {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "chat")]
        category: String,
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
}
pub(crate) fn execute_inference_command(command: InferenceCommand) -> ExitCode {
    match command {
        InferenceCommand::Config { json } => inference_surface::execute_inference(json),
        InferenceCommand::Vault { json } => inference_surface::execute_inference_vault(json),
        InferenceCommand::Status { json, tail } => {
            inference_surface::execute_inference_status(json, tail)
        }
        InferenceCommand::Start {
            alias,
            json,
            yes,
            no_wait,
        } => inference_surface::execute_inference_start(Some(alias.as_str()), json, yes, no_wait),
        InferenceCommand::Stop { alias, json, yes } => {
            inference_surface::execute_inference_stop(alias.as_deref(), json, yes)
        }
        InferenceCommand::Forge {
            report,
            alias: _,
            json,
            yes,
        } => inference_surface::execute_inference_forge(&report, "", json, yes),
        InferenceCommand::Discover {
            json,
            apply,
            open_webui,
        } => inference_surface::execute_inference_discover(json, apply, open_webui),
        InferenceCommand::Hardware { json } => inference_surface::execute_inference_hardware(json),
        InferenceCommand::Plan { alias, json, apply } => {
            inference_surface::execute_inference_plan(&alias, json, apply)
        }
        InferenceCommand::Models { command } => match command {
            ModelsCommand::Show {
                json,
                persist,
                ensure_profiles,
            } => inference_surface::execute_inference_models(json, persist, ensure_profiles),
            ModelsCommand::Consolidate { json, yes } => {
                inference_surface::execute_inference_models_consolidate(json, yes)
            }
        },
        InferenceCommand::Hf { command } => match command {
            HfCommand::Status { json } => inference_surface::execute_inference_hf_status(json),
            HfCommand::Download {
                repo,
                json,
                revision,
                include,
                exclude,
                repo_type,
                cache_only,
                alias,
                register_profile,
                no_profile,
            } => inference_surface::execute_inference_hf_download(
                &repo,
                json,
                &inference_surface::HfDownloadArgs {
                    revision: revision.as_deref(),
                    include: &include,
                    exclude: &exclude,
                    repo_type: repo_type.as_deref(),
                    cache_only,
                    alias: alias.as_deref(),
                    register_profile: register_profile && !no_profile,
                },
            ),
            HfCommand::Categories { json } => {
                inference_surface::execute_inference_hf_categories(json)
            }
            HfCommand::Search {
                json,
                category,
                query,
                author,
                limit,
            } => inference_surface::execute_inference_hf_search(
                json,
                category.as_deref(),
                query.as_deref(),
                author.as_deref(),
                limit,
            ),
            HfCommand::Suggest {
                json,
                category,
                limit,
            } => inference_surface::execute_inference_hf_suggest(json, &category, limit),
        },
        InferenceCommand::Trace { command } => execute_trace_command(command),
    }
}

#[derive(Debug, serde::Serialize)]
struct TracePostureReport {
    schema: &'static str,
    status: &'static str,
    ledger_path: String,
    total_turns: usize,
    ok_rate: f64,
    mean_elapsed_ms: f64,
    total_prompt_tokens: u64,
    total_gen_tokens: u64,
    total_tokens: u64,
}

fn execute_trace_command(command: TraceCommand) -> ExitCode {
    let ledger_path = std::env::var(cynic_inference::domain::chat_trace::ChatTrace::LEDGER_ENV_KEY)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let local_evidence = std::path::PathBuf::from("evidence/inference_traces.jsonl");
            if local_evidence.exists() {
                local_evidence
            } else {
                std::path::PathBuf::from(
                    cynic_inference::domain::chat_trace::ChatTrace::DEFAULT_LEDGER_PATH,
                )
            }
        });

    let records: Vec<cynic_inference::domain::chat_trace::ChatTrace> = if ledger_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&ledger_path) {
            content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    match command {
        TraceCommand::Status { json } => {
            let summary = cynic_inference::domain::chat_trace::summarize_traces(&records);
            let mut total_prompt = 0u64;
            let mut total_gen = 0u64;
            for r in &records {
                if let Some(p) = r.prompt_tokens {
                    total_prompt += p;
                }
                if let Some(g) = r.gen_tokens {
                    total_gen += g;
                }
            }

            let report = TracePostureReport {
                schema: "cynic.inference.trace_posture.v1",
                status: if records.is_empty() { "empty_ledger" } else { "active" },
                ledger_path: ledger_path.display().to_string(),
                total_turns: summary.turns,
                ok_rate: summary.ok_rate,
                mean_elapsed_ms: summary.mean_elapsed_ms,
                total_prompt_tokens: total_prompt,
                total_gen_tokens: total_gen,
                total_tokens: total_prompt + total_gen,
            };

            if json {
                crate::emit_json(&report, false)
            } else {
                println!("CYNIC Inference ChatTrace Posture");
                println!("===============================");
                println!("Ledger:  {}", report.ledger_path);
                println!("Status:  {}", report.status);
                println!(
                    "Turns:   {} total (ok_rate={:.1}%, mean_elapsed={:.1}ms)",
                    report.total_turns,
                    report.ok_rate * 100.0,
                    report.mean_elapsed_ms
                );
                println!(
                    "Tokens:  {} total (prompt={}, gen={})",
                    report.total_tokens, report.total_prompt_tokens, report.total_gen_tokens
                );
                ExitCode::SUCCESS
            }
        }
        TraceCommand::Tail { limit, json } => {
            let start = records.len().saturating_sub(limit);
            let slice = &records[start..];

            if json {
                crate::emit_json(&slice, false)
            } else if slice.is_empty() {
                println!("No inference traces recorded in {}.", ledger_path.display());
                ExitCode::SUCCESS
            } else {
                println!("Last {} inference trace(s) from {}:", slice.len(), ledger_path.display());
                for (idx, trace) in slice.iter().enumerate() {
                    let status_str = match trace.status {
                        cynic_inference::domain::chat_trace::TraceStatus::Ok => "OK",
                        cynic_inference::domain::chat_trace::TraceStatus::Blocked => "BLOCKED",
                        cynic_inference::domain::chat_trace::TraceStatus::Error => "ERROR",
                    };
                    println!(
                        "  [{}] status={:<9} session={:<20} elapsed={}ms tokens={}+{}",
                        idx + 1,
                        status_str,
                        trace.session_id,
                        trace.elapsed_ms,
                        trace
                            .prompt_tokens
                            .map_or_else(|| "unknown".to_string(), |v| v.to_string()),
                        trace
                            .gen_tokens
                            .map_or_else(|| "unknown".to_string(), |v| v.to_string()),
                    );
                }
                ExitCode::SUCCESS
            }
        }
    }
}
