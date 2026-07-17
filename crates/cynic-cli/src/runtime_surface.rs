use std::process::ExitCode;

use clap::Subcommand;

use crate::bootstrap_surface::ProfileArgs;

mod execution_context;
mod host_checks;
mod inference_surface;
mod network_preflight;
mod paths;
mod report;
mod resolver_plan;
mod suite;
mod validation;

#[derive(Subcommand)]
pub(crate) enum RuntimeCommand {
    Paths {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
    },
    Validate {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        lab_registry: Option<String>,
    },
    Suite {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long, default_value = "fleet")]
        profile: String,
    },
    Plan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long, default_value = "fleet")]
        profile: String,
        #[arg(long)]
        proof: Option<String>,
    },
    Harness {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        kind: Option<String>,
    },
    Conformance {
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        profile: ProfileArgs,
    },
    Metrics {
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        profile: ProfileArgs,
        #[arg(long)]
        proof: Option<String>,
    },
    ExecutionContext {
        #[arg(long)]
        json: bool,
        #[arg(long = "host")]
        hosts: Vec<String>,
    },
    NetworkPreflight {
        #[arg(long)]
        json: bool,
        #[arg(long = "host")]
        hosts: Vec<String>,
    },
    ResolverPlan {
        #[arg(long)]
        json: bool,
        #[arg(long = "host")]
        hosts: Vec<String>,
    },
    RepoAccess {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        profile: Option<String>,
    },
    /// Aggregate runtime phase, report, and inference SSOT status.
    Report {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        output: Option<String>,
        #[arg(long)]
        persist: bool,
    },
    /// Report the sovereign inference configuration as machine-readable JSON.
    /// Native surface: declared routing SSOT (not a live probe).
    /// SSOT path: CYNIC_INFERENCE_CONFIG or /etc/cynic/inference.toml
    Inference {
        #[arg(long)]
        json: bool,
    },
    /// List declared local model profiles in the CYNIC vault (native system).
    /// Not ODS install state — see `inference-discover` to attach a running ODS stack.
    InferenceVault {
        #[arg(long)]
        json: bool,
    },
    /// Status of the native on-demand local inference unit (llama-server@).
    InferenceStatus {
        #[arg(long)]
        json: bool,
    },
    /// Start a native local model profile (systemd --user llama-server@<alias>).
    InferenceStart {
        /// Profile alias from `cynic runtime inference-vault --json`.
        alias: String,
        #[arg(long)]
        json: bool,
    },
    /// Stop the native local inference unit (optional alias; default = running).
    InferenceStop {
        alias: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Integrate with a running [Osmantic ODS](https://github.com/Osmantic/ODS)
    /// (or compatible) environment: probe local endpoints and publish into inference.toml.
    /// ODS bootstraps the AI stack; this does not install ODS or start vault profiles.
    InferenceDiscover {
        #[arg(long)]
        json: bool,
    },
    /// Centralized model store: layout + vault artifacts + profiles (+ optional registry write).
    InferenceModels {
        #[arg(long)]
        json: bool,
        /// Persist scan to CYNIC_MODEL_REGISTRY (default: ~/.config/cynic/model-registry.json).
        #[arg(long)]
        persist: bool,
    },
    /// Hugging Face CLI status (binary, version, auth) relative to model layout.
    InferenceHfStatus {
        #[arg(long)]
        json: bool,
    },
    /// Download a Hub repo into the CYNIC model vault via official `hf` CLI.
    InferenceHfDownload {
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
    /// List HF discovery categories (chat, code, embed, …).
    InferenceHfCategories {
        #[arg(long)]
        json: bool,
    },
    /// Probe host hardware for model sizing recommendations.
    InferenceHardware {
        #[arg(long)]
        json: bool,
    },
    /// Search Hub models by category/query; returns short aliases + hardware fit.
    InferenceHfSearch {
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
    /// Suggest models for this host (category default: chat), ranked by hardware fit.
    InferenceHfSuggest {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "chat")]
        category: String,
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// Probe-driven launch plan for a native profile (agnostic flags — not frozen EXTRA_ARGS).
    InferencePlan {
        alias: String,
        #[arg(long)]
        json: bool,
        /// Write runtime env + systemd drop-in from the plan.
        #[arg(long)]
        apply: bool,
    },
    /// Compatibility alias → `cynic deps` (prefer top-level).
    Deps {
        #[arg(long)]
        json: bool,
    },
    /// Compatibility alias → `cynic deps plan`.
    DepsPlan {
        #[arg(long)]
        json: bool,
    },
    /// Compatibility alias → `cynic deps upgrade`.
    DepsUpgrade {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        confirm: Option<String>,
        #[arg(long = "only")]
        only: Vec<String>,
    },
}

pub(crate) fn execute_runtime(command: RuntimeCommand) -> ExitCode {
    match command {
        RuntimeCommand::Paths { json, registry } => paths::execute_paths(json, registry),
        RuntimeCommand::Validate {
            json,
            registry,
            lab_registry,
        } => validation::execute_validate(json, registry, lab_registry),
        RuntimeCommand::Suite {
            json,
            registry,
            profile,
        } => suite::execute_suite(json, registry, profile),
        RuntimeCommand::Plan {
            json,
            registry,
            profile,
            proof,
        } => suite::execute_plan(json, registry, profile, proof),
        RuntimeCommand::Harness { json, kind } => host_checks::execute_harness(json, kind),
        RuntimeCommand::Conformance { json, profile } => {
            host_checks::execute_conformance(json, profile)
        }
        RuntimeCommand::Metrics {
            json,
            profile,
            proof,
        } => host_checks::execute_metrics(json, profile, proof),
        RuntimeCommand::ExecutionContext { json, hosts } => {
            execution_context::execute_execution_context(json, hosts)
        }
        RuntimeCommand::NetworkPreflight { json, hosts } => {
            network_preflight::execute_network_preflight(json, hosts)
        }
        RuntimeCommand::ResolverPlan { json, hosts } => {
            resolver_plan::execute_resolver_plan(json, hosts)
        }
        RuntimeCommand::RepoAccess { json, profile } => {
            host_checks::execute_repo_access(json, profile)
        }
        RuntimeCommand::Report {
            json,
            output,
            persist,
        } => report::execute_report(json, output, persist),
        RuntimeCommand::Inference { json } => inference_surface::execute_inference(json),
        RuntimeCommand::InferenceVault { json } => inference_surface::execute_inference_vault(json),
        RuntimeCommand::InferenceStatus { json } => {
            inference_surface::execute_inference_status(json)
        }
        RuntimeCommand::InferenceStart { alias, json } => {
            inference_surface::execute_inference_start(&alias, json)
        }
        RuntimeCommand::InferenceStop { alias, json } => {
            inference_surface::execute_inference_stop(alias.as_deref(), json)
        }
        RuntimeCommand::InferenceDiscover { json } => {
            inference_surface::execute_inference_discover(json)
        }
        RuntimeCommand::InferenceModels { json, persist } => {
            inference_surface::execute_inference_models(json, persist)
        }
        RuntimeCommand::InferenceHfStatus { json } => {
            inference_surface::execute_inference_hf_status(json)
        }
        RuntimeCommand::InferenceHfDownload {
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
            revision.as_deref(),
            &include,
            &exclude,
            repo_type.as_deref(),
            cache_only,
            alias.as_deref(),
            register_profile && !no_profile,
        ),
        RuntimeCommand::InferenceHfCategories { json } => {
            inference_surface::execute_inference_hf_categories(json)
        }
        RuntimeCommand::InferenceHardware { json } => {
            inference_surface::execute_inference_hardware(json)
        }
        RuntimeCommand::InferenceHfSearch {
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
        RuntimeCommand::InferenceHfSuggest {
            json,
            category,
            limit,
        } => inference_surface::execute_inference_hf_suggest(json, &category, limit),
        RuntimeCommand::InferencePlan { alias, json, apply } => {
            inference_surface::execute_inference_plan(&alias, json, apply)
        }
        RuntimeCommand::Deps { json } => crate::deps_surface::execute_deps(json),
        RuntimeCommand::DepsPlan { json } => crate::deps_surface::execute_deps_plan(json),
        RuntimeCommand::DepsUpgrade {
            json,
            yes,
            apply,
            confirm,
            only,
        } => crate::deps_surface::execute_deps_upgrade(json, apply, yes, confirm.as_deref(), &only),
    }
}
