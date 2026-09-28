use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::harness_progression::{HarnessProgressionVerdict, RuntimeHarnessKind};
use cynic_host::registry::DEFAULT_REGISTRY_PATH;

use crate::fail;
use crate::host_profile_selection::ProfileArgs;

mod agent_matrix;
mod capacity;
mod compat_aliases;
mod cpu;
mod disk;
mod execution_context;
mod forgejo_token_cli;
mod host_checks;
pub(crate) mod inference_surface;
mod memory;
mod network;
mod operator_models;

pub(crate) use operator_models::{OperatorModelsCommand, execute_operator_models};
mod build_artefact;
mod network_preflight;
mod paths;
mod persistence;
mod queue;
mod registry_observation;
mod report;
mod resolver_plan;
mod storage_topology;
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
    /// Declared and observed registry state.
    Registry {
        #[command(subcommand)]
        command: RuntimeRegistryCommand,
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
    /// Assess local capacity: load tier + RAM/CPU headroom (read-only).
    /// Policy provenance: CYNIC_METABOLIC_POLICY (JSON file) or legacy defaults.
    Capacity {
        #[arg(long)]
        json: bool,
    },
    /// Host harness: observed state, and transition progress.
    Harness {
        #[command(subcommand)]
        command: RuntimeHarnessCommand,
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
    /// Active storage runtime, engine, transport and capability proof levels.
    Storage {
        #[command(subcommand)]
        command: RuntimeStorageCommand,
    },
    /// Evaluate storage credential sync status.
    /// Set a bounded storage client policy; requires --apply and read-back proof.
    /// The enrolled machine principal; usable only after adapter selection is proven.
    Execution {
        #[command(subcommand)]
        command: RuntimeExecutionCommand,
    },
    /// Resolver resolution plan.
    Resolver {
        #[command(subcommand)]
        command: RuntimeResolverCommand,
    },
    /// Check repository access for the declared hosts.
    Repo {
        #[command(subcommand)]
        command: RuntimeRepoCommand,
    },
    /// Persist a runtime proof record and require read-back.
    Proof {
        #[command(subcommand)]
        command: RuntimeProofCommand,
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
    /// Which runtime surfaces are safe for bounded agent autonomy now vs later.
    Agent {
        #[command(subcommand)]
        command: RuntimeAgentCommand,
    },
    /// Probe host storage capacity, filesystems, caches and large files.
    /// Host disk: observed state, and cleanup.
    Disk {
        #[command(subcommand)]
        command: RuntimeDiskCommand,
    },
    /// Probe host memory capacity, swap, hugepages, and pressure stalls.
    Memory {
        #[arg(long)]
        json: bool,
    },
    /// Probe host CPU model, topology, jiffies, and pressure stalls.
    Cpu {
        #[arg(long)]
        json: bool,
    },
    /// Probe host network interfaces, addresses, routes, usage, and drops.
    /// Host network: observed state, and preflight before a fleet change.
    Network {
        #[command(subcommand)]
        command: RuntimeNetworkCommand,
    },
    /// Observe or provision the Forgejo runtime token.
    Forgejo {
        #[command(subcommand)]
        command: RuntimeForgejoCommand,
    },
    /// 4-layer build artefact observation and GC (checkouts pressure).
    Build {
        #[command(subcommand)]
        command: RuntimeBuildCommand,
    },
    /// Queue plan — branch+discovery+GC composed via LocalQueueAdapter (thin).
    Queue {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeStorageCommand {
    /// Active storage runtime, engine, transport and capability proof levels.
    Capabilities {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        url: Option<String>,
    },
    /// Evaluate storage credential sync status.
    Credential {
        #[arg(long, default_value_t = false)]
        json: bool,
        #[arg(long)]
        endpoint: Option<String>,
    },
    /// Storage clients and the policy they are bound to.
    Client {
        #[command(subcommand)]
        command: RuntimeStorageClientCommand,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeHarnessCommand {
    /// Observed harness state for one kind.
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        kind: Option<String>,
    },
    /// Progress between harness kinds, per proof category.
    Progress {
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
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeNetworkCommand {
    /// Observed host network state.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Preflight the network before a fleet change.
    Preflight {
        #[arg(long)]
        json: bool,
        #[arg(long = "host")]
        hosts: Vec<String>,
        #[arg(long)]
        persist: bool,
        #[arg(long)]
        endpoint: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeDiskCommand {
    /// Observed host disk capacity, filesystems, caches and large files.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Plan or execute host disk space cleanup.
    Clean {
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

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeRegistryCommand {
    /// Discover the active runtime, engine, transport and capability proof levels.
    Observe {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        observation: String,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeStorageClientCommand {
    /// The client policy itself.
    Policy {
        #[command(subcommand)]
        command: RuntimeStorageClientPolicyCommand,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeStorageClientPolicyCommand {
    /// Set a bounded client policy; requires --apply and read-back proof.
    Set {
        #[arg(long)]
        policy_path: String,
        #[arg(long = "endpoint", required = true)]
        endpoints: Vec<String>,
        #[arg(long, default_value_t = 5000)]
        timeout_ms: u64,
        #[arg(long, default_value_t = 1)]
        write_quorum: u32,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeResolverCommand {
    /// Resolver resolution plan.
    Plan {
        #[arg(long)]
        json: bool,
        #[arg(long = "host")]
        hosts: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeRepoCommand {
    /// Check repository access for the declared hosts.
    Access {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        profile: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeProofCommand {
    /// Persist a runtime proof record and require read-back.
    Persist {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        proof: String,
        #[arg(long)]
        endpoint: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeExecutionCommand {
    /// The enrolled machine principal; usable only after adapter selection is proven.
    Context {
        #[arg(long)]
        json: bool,
        #[arg(long = "host")]
        hosts: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeAgentCommand {
    /// Which runtime surfaces are safe for bounded agent autonomy now vs later.
    Matrix {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeBuildCommand {
    /// 4-layer build artefact observation and GC (checkouts pressure).
    Artefact {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RuntimeForgejoCommand {
    /// Observe or provision the Forgejo runtime token.
    Token {
        #[arg(long, value_enum, default_value = "status")]
        mode: ForgejoTokenMode,
        #[arg(long)]
        json: bool,
        /// Apply mutation (create/rotate). Required for write operations.
        #[arg(long)]
        yes: bool,
        /// Override the canonical token secret path (default /etc/cynic/secrets/forgejo-token).
        #[arg(long)]
        token_path: Option<String>,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub(crate) enum ForgejoTokenMode {
    Status,
    Create,
    Rotate,
}

/// Select an adapter only after a future capability resolver proves both
/// transport reachability and credential injection. Until then this surface is
/// deliberately fail-closed rather than pretending local-only is dynamic.
pub(crate) fn forgejo_token_adapter_selector() -> Box<dyn cynic_vcs_forgejo::ForgejoTokenApiPort> {
    let api_url = std::env::var("CYNIC_FORGEJO_API_URL")
        .or_else(|_| std::env::var("FORGEJO_API_URL"))
        .or_else(|_| std::env::var("FORGEJO_URL"))
        .unwrap_or_else(|_| {
            let reg_path = crate::registry_path(None);
            if let Ok(content) = std::fs::read_to_string(&reg_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(nodes) = json.get("nodes").and_then(|n| n.as_array()) {
                        for node in nodes {
                            let role = node
                                .get("role")
                                .and_then(|r| r.as_str())
                                .unwrap_or_default();
                            let id = node.get("id").and_then(|r| r.as_str()).unwrap_or_default();
                            if role == "forge_primary" || id == "forgejo" {
                                let ip = node
                                    .get("network")
                                    .and_then(|n| n.get("synapse"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("127.0.0.1");
                                let port = node
                                    .get("services")
                                    .and_then(|s| s.as_array())
                                    .and_then(|svcs| {
                                        svcs.iter()
                                            .find(|s| {
                                                s.get("name").and_then(|n| n.as_str())
                                                    == Some("forgejo")
                                            })
                                            .and_then(|s| s.get("port"))
                                            .and_then(|p| p.as_u64())
                                    })
                                    .unwrap_or(3000);
                                return format!("http://{ip}:{port}");
                            }
                        }
                    }
                }
            }
            "http://127.0.0.1:3000".to_string()
        });

    Box::new(
        cynic_vcs_forgejo::forgejo_token::HttpForgejoTokenApiAdapter {
            api_base: api_url,
            principal: "jeanterre13".to_string(),
            authorization_token: cynic_secrets::env_var("forgejo-token", "FORGEJO_TOKEN"),
        },
    )
}

pub(crate) fn execute_runtime(command: RuntimeCommand) -> ExitCode {
    let command = match compat_aliases::try_execute_compat_alias(command) {
        Ok(exit) => return exit,
        Err(command) => command,
    };
    match command {
        RuntimeCommand::Paths { json, registry } => paths::execute_paths(json, registry),
        RuntimeCommand::Registry {
            command:
                RuntimeRegistryCommand::Observe {
                    json,
                    registry,
                    observation,
                    apply,
                },
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
        RuntimeCommand::Harness {
            command: RuntimeHarnessCommand::Status { json, kind },
        } => host_checks::execute_harness(json, kind.as_deref()),
        RuntimeCommand::Harness {
            command:
                RuntimeHarnessCommand::Progress {
                    current_kind,
                    target_kind,
                    satisfied,
                    unknown,
                    json,
                },
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
        RuntimeCommand::Storage { command } => match command {
            RuntimeStorageCommand::Capabilities { json, url } => {
                host_checks::execute_storage_capabilities(json, url.as_deref())
            }
            RuntimeStorageCommand::Credential { json, endpoint } => {
                match execute_storage_credential_sync(json, endpoint.as_deref()) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => fail(e),
                }
            }
            RuntimeStorageCommand::Client {
                command:
                    RuntimeStorageClientCommand::Policy {
                        command:
                            RuntimeStorageClientPolicyCommand::Set {
                                policy_path,
                                endpoints,
                                timeout_ms,
                                write_quorum,
                                apply,
                                json,
                            },
                    },
            } => storage_topology::execute_client_policy_set(
                &policy_path,
                &endpoints,
                timeout_ms,
                write_quorum,
                apply,
                json,
            ),
        },
        RuntimeCommand::Execution {
            command: RuntimeExecutionCommand::Context { json, hosts },
        } => execution_context::execute_execution_context(json, &hosts),
        RuntimeCommand::Resolver {
            command: RuntimeResolverCommand::Plan { json, hosts },
        } => resolver_plan::execute_resolver_plan(json, &hosts),
        RuntimeCommand::Repo {
            command: RuntimeRepoCommand::Access { json, profile },
        } => host_checks::execute_repo_access(json, profile),
        RuntimeCommand::Proof {
            command:
                RuntimeProofCommand::Persist {
                    json,
                    proof,
                    endpoint,
                },
        } => persistence::execute_persistence(json, &proof, endpoint.as_deref()),
        RuntimeCommand::Report {
            json,
            output,
            persist,
        } => report::execute_report(json, output.as_deref(), persist),
        RuntimeCommand::Agent {
            command: RuntimeAgentCommand::Matrix { json },
        } => agent_matrix::execute_agent_matrix(json),
        RuntimeCommand::Capacity { json } => capacity::execute_capacity(json),
        RuntimeCommand::Memory { json } => memory::execute_memory(json),
        RuntimeCommand::Cpu { json } => cpu::execute_cpu(json),
        RuntimeCommand::Forgejo {
            command:
                RuntimeForgejoCommand::Token {
                    mode,
                    json,
                    yes,
                    token_path,
                },
        } => forgejo_token_cli::execute_forgejo_token(mode, json, yes, token_path.clone()),
        RuntimeCommand::Build {
            command: RuntimeBuildCommand::Artefact { json, apply },
        } => build_artefact::execute_build_artefact(json, apply),
        RuntimeCommand::Queue { json, apply } => queue::execute_queue(json, apply),
        // Disk and Network are pre-dispatched in compat_aliases: that module
        // already owned their handlers, and moving them here would have split
        // one object across two dispatch chains for no gain.
        RuntimeCommand::Disk { .. } | RuntimeCommand::Network { .. } => {
            unreachable!("pre-dispatched runtime command")
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

pub(crate) fn execute_storage_credential_sync(
    json: bool,
    endpoint: Option<&str>,
) -> Result<(), String> {
    use cynic_host::storage_credential_sync::{
        StorageCredentialSyncAdapter, StorageCredentialSyncPort,
    };
    use cynic_storage::credential_sync::{StorageCredentialSpec, decide_credential_sync};

    let resolved_endpoint = endpoint
        .map(|s| s.to_string())
        .or_else(|| std::env::var("CYNIC_SURREALDB_URL").ok())
        .or_else(|| std::env::var("SURREALDB_URL").ok())
        .unwrap_or_else(|| {
            let reg_path = crate::registry_path(None);
            if let Ok(content) = std::fs::read_to_string(&reg_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(nodes) = val.get("nodes").and_then(|n| n.as_array()) {
                        for node in nodes {
                            let role = node
                                .get("role")
                                .and_then(|r| r.as_str())
                                .unwrap_or_default();
                            let id = node.get("id").and_then(|r| r.as_str()).unwrap_or_default();
                            let services = node.get("services").and_then(|s| s.as_array());
                            let has_surreal = services
                                .map(|svcs| {
                                    svcs.iter().any(|s| {
                                        s.get("name").and_then(|n| n.as_str()) == Some("surrealdb")
                                    })
                                })
                                .unwrap_or(false);
                            if role == "storage_primary" || id.contains("surrealdb") || has_surreal
                            {
                                let ip = node
                                    .get("network")
                                    .and_then(|n| n.get("synapse"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("127.0.0.1");
                                let port = services
                                    .and_then(|svcs| {
                                        svcs.iter()
                                            .find(|s| {
                                                s.get("name").and_then(|n| n.as_str())
                                                    == Some("surrealdb")
                                            })
                                            .and_then(|s| s.get("port"))
                                            .and_then(|p| p.as_u64())
                                    })
                                    .unwrap_or(8000);
                                return format!("http://{ip}:{port}");
                            }
                        }
                    }
                }
            }
            "http://127.0.0.1:8000".to_string()
        });

    let spec = StorageCredentialSpec {
        username: "root".into(),
        password: "cynic".into(),
        endpoint: resolved_endpoint,
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
        use cynic_storage::credential_sync::CredentialSyncVerdict;
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
