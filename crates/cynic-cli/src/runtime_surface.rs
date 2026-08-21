use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::harness_progression::{HarnessProgressionVerdict, RuntimeHarnessKind};
use cynic_host::registry::DEFAULT_REGISTRY_PATH;

use crate::bootstrap_surface::ProfileArgs;
use crate::fail;

mod agent_matrix;
mod compat_aliases;
mod disk;
mod execution_context;
mod host_checks;
mod inference_dispatch;
mod inference_surface;
mod operator_models;

pub(crate) use operator_models::OperatorModelsCommand;
mod network_preflight;
mod paths;
mod persistence;
mod registry_observation;
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
    RegistryObservation {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        observation: String,
        #[arg(long)]
        apply: bool,
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
    HarnessProgress {
        #[arg(long, value_enum)]
        current_kind: RuntimeHarnessKindArg,
        #[arg(long, value_enum)]
        target_kind: RuntimeHarnessKindArg,
        #[arg(long, value_enum)]
        satisfied: Vec<ProofCategoryArg>,
        #[arg(long, value_enum)]
        unknown: Vec<ProofCategoryArg>,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    Conformance {
        // No local `json` here: ProfileArgs (bootstrap_surface/selection.rs:22)
        // already declares `--json`, and clap panics at startup on a duplicate
        // argument id ("Argument names must be unique, but 'json' is in use by
        // more than one argument or group"). Read it from the flattened group.
        #[command(flatten)]
        profile: ProfileArgs,
    },
    Metrics {
        // Same duplicate-id collision as Conformance: --json comes from
        // ProfileArgs, not from a second local declaration.
        #[command(flatten)]
        profile: ProfileArgs,
        #[arg(long)]
        proof: Option<String>,
    },
    /// Discover the active SurrealDB runtime, engine, transport, and capability proof levels.
    StorageCapabilities {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        url: Option<String>,
    },
    /// Evaluate storage credential sync status.
    StorageCredential {
        #[arg(long, default_value_t = false)]
        json: bool,
        #[arg(long, default_value = "http://10.10.10.103:8000")]
        endpoint: String,
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
        #[arg(long)]
        persist: bool,
        #[arg(long)]
        endpoint: Option<String>,
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
    /// Persist an existing runtime proof record through the kernel and require read-back.
    ProofPersist {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        proof: String,
        #[arg(long)]
        endpoint: Option<String>,
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
    /// Report which runtime/bootstrap surfaces are safe to expose to bounded agent autonomy now vs later.
    AgentMatrix {
        #[arg(long)]
        json: bool,
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
        /// Show the last N log lines for the running local process (requires process supervisor).
        #[arg(long, default_value_t = 0)]
        tail: usize,
    },
    /// Start a native local model profile (process|systemd supervisor).
    /// Omit alias / pass `auto` to ensure vault profiles and start the preferred model.
    InferenceStart {
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
    /// Stop the native local inference unit (optional alias; default = running).
    InferenceStop {
        alias: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Forge a vault profile from a BenchmarkReport, write it, then launch.
    InferenceForge {
        #[arg(long)]
        report: String,
        #[arg(long, default_value = "auto")]
        alias: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Peer / optional ODS attach plane (loose coupling).
    ///
    /// Default: **probe-only** status (`cynic.inference.ods_discovery.v1`).
    /// `--apply` publishes peers into inference.toml (mutation).
    /// `--open-webui` (with `--apply`) also tries Open WebUI publisher.
    ///
    /// Does **not** install ODS — use `cynic deps upgrade --yes --only ods`.
    /// Native vault remains independent: `inference-start`.
    InferenceDiscover {
        #[arg(long)]
        json: bool,
        /// Publish discovered peers into routing SSOT (inference.toml).
        #[arg(long)]
        apply: bool,
        /// With `--apply`, also attempt Open WebUI connection publish (opt-in).
        #[arg(long = "open-webui")]
        open_webui: bool,
    },
    /// Centralized model store: layout + vault artifacts + profiles (+ optional registry write).
    InferenceModels {
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
    /// Default is plan-only; pass `--yes` to apply physical moves.
    InferenceModelsConsolidate {
        #[arg(long)]
        json: bool,
        /// Apply moves (default: plan only).
        #[arg(long)]
        yes: bool,
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
    /// Compatibility alias → `cynic deps audit`.
    DepsAudit {
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
    /// Probe host storage capacity, filesystems, caches and large files.
    Disk {
        #[arg(long)]
        json: bool,
    },
    /// Plan or execute host disk space cleanup.
    DiskClean {
        #[arg(long)]
        json: bool,
        /// Apply the cleanup plan (mutation).
        #[arg(long)]
        apply: bool,
        /// Reconcile only the registry-declared build-runner cache profile.
        #[arg(long)]
        build_runner: bool,
        #[arg(long, default_value = DEFAULT_REGISTRY_PATH)]
        registry: String,
        /// Build-runner node id; defaults to the observed hostname.
        #[arg(long)]
        node: Option<String>,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
    },
}

pub(crate) fn execute_runtime(command: RuntimeCommand) -> ExitCode {
    let command = match inference_dispatch::try_execute_inference(command) {
        Ok(exit) => return exit,
        Err(command) => command,
    };
    let command = match compat_aliases::try_execute_compat_alias(command) {
        Ok(exit) => return exit,
        Err(command) => command,
    };
    match command {
        RuntimeCommand::Paths { json, registry } => paths::execute_paths(json, registry),
        RuntimeCommand::RegistryObservation {
            json,
            registry,
            observation,
            apply,
        } => {
            registry_observation::execute_registry_observation(json, registry, &observation, apply)
        }
        RuntimeCommand::Validate {
            json,
            registry,
            lab_registry,
        } => validation::execute_validate(json, registry, lab_registry),
        RuntimeCommand::Suite {
            json,
            registry,
            profile,
        } => suite::execute_suite(json, registry.as_deref(), &profile),
        RuntimeCommand::Plan {
            json,
            registry,
            profile,
            proof,
        } => suite::execute_plan(json, registry.as_deref(), &profile, proof.as_deref()),
        RuntimeCommand::HarnessProgress {
            current_kind,
            target_kind,
            satisfied,
            unknown,
            json,
        } => {
            match execute_harness_progress(
                current_kind.clone(),
                target_kind.clone(),
                satisfied.clone(),
                unknown.clone(),
                json,
            ) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => fail(e),
            }
        }
        RuntimeCommand::Conformance { profile } => {
            let json = profile.json;
            host_checks::execute_conformance(json, &profile)
        }
        RuntimeCommand::Metrics { profile, proof } => {
            let json = profile.json;
            host_checks::execute_metrics(json, &profile, proof.as_deref())
        }
        RuntimeCommand::StorageCapabilities { json, url } => {
            host_checks::execute_storage_capabilities(json, url.as_deref())
        }
        RuntimeCommand::StorageCredential { json, endpoint } => {
            match execute_storage_credential_sync(json, &endpoint) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => fail(e),
            }
        }
        RuntimeCommand::ExecutionContext { json, hosts } => {
            execution_context::execute_execution_context(json, &hosts)
        }
        RuntimeCommand::NetworkPreflight { json, hosts, persist, endpoint } => {
            network_preflight::execute_network_preflight(json, &hosts, persist, endpoint.as_deref())
        }
        RuntimeCommand::ResolverPlan { json, hosts } => {
            resolver_plan::execute_resolver_plan(json, &hosts)
        }
        RuntimeCommand::RepoAccess { json, profile } => {
            host_checks::execute_repo_access(json, profile)
        }
        RuntimeCommand::ProofPersist {
            json,
            proof,
            endpoint,
        } => persistence::execute_persistence(json, &proof, endpoint.as_deref()),
        RuntimeCommand::Report {
            json,
            output,
            persist,
        } => report::execute_report(json, output.as_deref(), persist),
        RuntimeCommand::AgentMatrix { json } => agent_matrix::execute_agent_matrix(json),
        RuntimeCommand::Inference { .. }
        | RuntimeCommand::InferenceVault { .. }
        | RuntimeCommand::InferenceStatus { .. }
        | RuntimeCommand::InferenceStart { .. }
        | RuntimeCommand::InferenceStop { .. }
        | RuntimeCommand::InferenceDiscover { .. }
        | RuntimeCommand::InferenceModels { .. }
        | RuntimeCommand::InferenceModelsConsolidate { .. }
        | RuntimeCommand::InferenceHfStatus { .. }
        | RuntimeCommand::InferenceHfDownload { .. }
        | RuntimeCommand::InferenceHfCategories { .. }
        | RuntimeCommand::InferenceHardware { .. }
        | RuntimeCommand::InferenceHfSearch { .. }
        | RuntimeCommand::InferenceHfSuggest { .. }
        | RuntimeCommand::InferencePlan { .. }
        | RuntimeCommand::InferenceForge { .. }
        | RuntimeCommand::Deps { .. }
        | RuntimeCommand::DepsPlan { .. }
        | RuntimeCommand::DepsAudit { .. }
        | RuntimeCommand::DepsUpgrade { .. }
        | RuntimeCommand::Disk { .. }
        | RuntimeCommand::DiskClean { .. } => unreachable!("pre-dispatched runtime command"),
        RuntimeCommand::Harness { json, kind } => {
            // This arm used to sit in the `unreachable!("pre-dispatched runtime
            // command")` group, but nothing ever pre-dispatched it: no early
            // return in execute_runtime, inference_dispatch, or compat_aliases
            // claims Harness, so `cynic runtime harness --json` panicked on the
            // documented happy path.
            //
            // The handler already existed in host_checks.rs — a module carrying
            // `#![allow(dead_code)]` because none of its three functions had a
            // caller. Calling it is the repair; reimplementing it here would
            // have duplicated the projection and left the fossil in place.
            host_checks::execute_harness(json, kind.as_deref())
        }
    }
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub(crate) enum RuntimeHarnessKindArg {
    Lab,
    Conformance,
    Staging,
    Canary,
    Production,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub(crate) enum ProofCategoryArg {
    Bootstrap,
    Hardware,
    HostConformance,
    BuildReproduction,
    Network,
    TargetAttestation,
    RollbackIntent,
}

impl From<RuntimeHarnessKindArg> for RuntimeHarnessKind {
    fn from(arg: RuntimeHarnessKindArg) -> Self {
        match arg {
            RuntimeHarnessKindArg::Lab => RuntimeHarnessKind::ClosedLocalLab,
            RuntimeHarnessKindArg::Conformance => RuntimeHarnessKind::Conformance,
            RuntimeHarnessKindArg::Staging => RuntimeHarnessKind::Staging,
            RuntimeHarnessKindArg::Canary => RuntimeHarnessKind::Canary,
            RuntimeHarnessKindArg::Production => RuntimeHarnessKind::Production,
        }
    }
}

impl From<ProofCategoryArg> for cynic_host::harness_progression::ProofCategory {
    fn from(arg: ProofCategoryArg) -> Self {
        use cynic_host::harness_progression::ProofCategory;
        match arg {
            ProofCategoryArg::Bootstrap => ProofCategory::Bootstrap,
            ProofCategoryArg::Hardware => ProofCategory::Hardware,
            ProofCategoryArg::HostConformance => ProofCategory::HostConformance,
            ProofCategoryArg::BuildReproduction => ProofCategory::BuildReproduction,
            ProofCategoryArg::Network => ProofCategory::Network,
            ProofCategoryArg::TargetAttestation => ProofCategory::TargetAttestation,
            ProofCategoryArg::RollbackIntent => ProofCategory::RollbackIntent,
        }
    }
}

pub(crate) fn execute_harness_progress(
    current_kind: RuntimeHarnessKindArg,
    target_kind: RuntimeHarnessKindArg,
    satisfied: Vec<ProofCategoryArg>,
    unknown: Vec<ProofCategoryArg>,
    json: bool,
) -> Result<(), String> {
    let evidence = cynic_host::harness_progression::HarnessProgressionEvidence {
        current_kind: current_kind.into(),
        target_kind: target_kind.into(),
        satisfied: satisfied.into_iter().map(Into::into).collect(),
        pending: vec![],
        unknown: unknown.into_iter().map(Into::into).collect(),
    };
    let verdict = cynic_host::harness_progression::decide_harness_progression(&evidence);
    if json {
        let s =
            serde_json::to_string_pretty(&verdict).map_err(|e| format!("serialize failed: {e}"))?;
        println!("{s}");
    } else {
        match &verdict {
            HarnessProgressionVerdict::Allowed {
                current_kind,
                target_kind,
                authority,
            } => {
                println!("ALLOWED: {current_kind:?} -> {target_kind:?} (authority: {authority:?})");
            }
            HarnessProgressionVerdict::Blocked { reason, detail } => {
                println!("BLOCKED: {reason} — {detail}");
            }
        }
    }
    Ok(())
}

pub(crate) fn execute_storage_credential_sync(json: bool, endpoint: &str) -> Result<(), String> {
    use cynic_host::storage_credential_sync::{
        StorageCredentialSyncAdapter, StorageCredentialSyncPort,
    };
    use cynic_protocol::storage_credential_sync::{StorageCredentialSpec, decide_credential_sync};

    let spec = StorageCredentialSpec {
        username: "root".into(),
        password: "cynic".into(),
        endpoint: endpoint.into(),
        namespace: "cynic".into(),
        database: "main".into(),
    };
    let adapter = StorageCredentialSyncAdapter::default();
    let obs = adapter.observe(&spec);
    let verdict = decide_credential_sync(&spec, &obs);

    if json {
        let s =
            serde_json::to_string_pretty(&verdict).map_err(|e| format!("serialize failed: {e}"))?;
        println!("{s}");
    } else {
        use cynic_protocol::storage_credential_sync::CredentialSyncVerdict;
        match &verdict {
            CredentialSyncVerdict::Synchronized => {
                println!("SYNCHRONIZED: storage credentials healthy");
            }
            CredentialSyncVerdict::OutOfSync { reason, detail, .. } => {
                println!("OUT OF SYNC: {reason} — {detail}");
            }
            CredentialSyncVerdict::Unknown { reason, detail } => {
                println!("UNKNOWN: {reason} — {detail}");
            }
        }
    }
    Ok(())
}
