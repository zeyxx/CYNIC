//! Node subcommand families — the noun level the node surface was missing.
//!
//! WHY: `contracts/foundation.json` forbids a hyphen in any CLI command
//! name, and `no_cli_command_carries_a_hyphen` is the gate that says so.
//! The node surface had eighteen flat verb-noun commands
//! (`node cert-inspect`, `node csr-create`, `node source-bootstrap`), each
//! one a missing noun level expressed as a dash.
//!
//! The split is by DOMAIN, not by word. The families here mirror the
//! execution modules that already exist under `node_surface/`, so the
//! command tree and the code tree say the same thing:
//!
//! | before                | after                     |
//! |-----------------------|---------------------------|
//! | `node identity-init`  | `node identity init`      |
//! | `node node-identity`  | `node identity observe`   |
//! | `node csr-create`     | `node identity csr`       |
//! | `node cert-inspect`   | `node cert inspect`       |
//! | `node cert-verify`    | `node cert verify`        |
//! | `node host-audit`     | `node host audit`         |
//! | `node consensus-status` | `node consensus status`  |
//! | `node registry-observation` | `node registry observe` |
//! | `node agent-workspace` | `node agent workspace`    |
//! | `node agent-workspace-lifecycle` | `node agent lifecycle` |
//! | `node agent-scheduler-step` | `node agent scheduler` |
//! | `node agent-workload-status` | `node workload status` |
//! | `node agent-workload-transition` | `node workload transition` |
//! | `node build-workload-grant-verify` | `node grant verify` |
//! | `node storage-reconcile` | `node storage reconcile` |
//! | `node source-admission-plan` | `node source admission` |
//! | `node source-materialize` | `node source materialize` |
//! | `node source-bootstrap` | `node source bootstrap`   |
//!
//! HOW: each family is a `Subcommand` enum holding the options its
//! commands had; the dispatch in `node_surface/mod.rs` forwards to the
//! same execution functions as before. Only the command PATH changed —
//! the flags, the defaults, the behaviour and the exit codes are the
//! ones the previous flat arms had.
//! NEGATIVE BOUNDARY: this module declares the command tree only. It
//! performs no I/O and holds no policy; the execution lives in the
//! domain modules it dispatches to.

use clap::Subcommand;

/// `cynic node identity ...` — node-local Ed25519 identity and enrollment.
#[derive(Subcommand)]
pub enum NodeIdentityCommand {
    /// Initialize or display the local node Ed25519 signing keypair.
    Init {
        #[arg(long)]
        key_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Observe or create the node-local Ed25519 identity used for machine enrollment.
    Observe {
        #[arg(long)]
        node_id: String,
        #[arg(long, default_value = "/var/lib/cynic/identity/node-signing.key")]
        signing_key_path: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Create a self-signed Certificate Signing Request (CSR) for node enrollment.
    Csr {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        role: String,
        #[arg(long)]
        key_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

/// `cynic node cert ...` — node enrollment certificates.
#[derive(Subcommand)]
pub enum NodeCertCommand {
    /// Inspect a local Node Enrollment Certificate.
    Inspect {
        #[arg(long)]
        cert_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Cryptographically verify a Node Enrollment Certificate against a Kernel authority public key.
    Verify {
        #[arg(long)]
        kernel_pubkey_hex: String,
        #[arg(long)]
        cert_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

/// `cynic node consensus ...` — fleet epoch consensus and clock skew.
#[derive(Subcommand)]
pub enum NodeConsensusCommand {
    /// Evaluate fleet epoch consensus and clock skew across cluster nodes.
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 1)]
        expected_epoch: u64,
        #[arg(long, default_value = "HEAD")]
        expected_commit: String,
        #[arg(long, default_value_t = 5000)]
        max_skew_ms: u64,
    },
}

/// `cynic node registry ...` — registry observation and projection.
#[derive(Subcommand)]
pub enum NodeRegistryCommand {
    /// Emit the node observation envelope consumed by registry observation projection.
    Observe {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "/etc/cynic/registry.json")]
        registry: String,
        #[arg(long)]
        output: Option<String>,
    },
}

/// `cynic node agent ...` — sovereign agent worktree and lifecycle surfaces.
#[derive(Subcommand)]
pub enum NodeAgentCommand {
    /// Plan or apply one lease-fenced source change in a sovereign agent worktree.
    Workspace {
        #[arg(long)]
        request: String,
        #[arg(long)]
        result_file: Option<String>,
        #[arg(long, default_value = "/etc/cynic/registry.json")]
        registry: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long, default_value = "http://127.0.0.1:3030")]
        kernel_endpoint: String,
        #[arg(long, default_value_t = 3_600_000)]
        max_claim_ttl_ms: u64,
        #[arg(long, default_value_t = 604_800_000)]
        max_retention_ms: u64,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Observe, plan, or collect one completed sovereign agent workspace.
    Lifecycle {
        #[arg(long)]
        workload_id: String,
        #[arg(long, default_value = "/etc/cynic/registry.json")]
        registry: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long, default_value = "http://127.0.0.1:3030")]
        kernel_endpoint: String,
        #[arg(long)]
        agent_id: String,
        #[arg(long, default_value_t = 300_000)]
        renewal_window_ms: u64,
        #[arg(long, default_value_t = 3_600_000)]
        renewal_ttl_ms: u64,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Reconcile and durably journal one scheduler step without executing it.
    Scheduler {
        #[arg(long)]
        observation: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long)]
        json: bool,
    },
}

/// `cynic node workload ...` — node-local workload records and transitions.
#[derive(Subcommand)]
pub enum NodeWorkloadCommand {
    /// Read one append-only node-local workload record.
    Status {
        #[arg(long)]
        workload_id: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long)]
        json: bool,
    },
    /// Plan or apply one revision-fenced workload state transition.
    Transition {
        #[arg(long)]
        request: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
}

/// `cynic node grant ...` — machine-signed build grants.
#[derive(Subcommand)]
pub enum NodeGrantCommand {
    /// Verify a machine-signed build grant against its live durable workload record.
    Verify {
        #[arg(long)]
        grant: String,
        #[arg(long)]
        workload_record: String,
        #[arg(long)]
        authority_public_key: String,
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        source_commit: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        json: bool,
    },
}

/// `cynic node storage ...` — node-local storage services.
#[derive(Subcommand)]
pub enum NodeStorageCommand {
    /// Observe or reconcile one node-local storage service with a bounded restart budget.
    Reconcile {
        #[arg(long, default_value = "/var/lib/cynic/storage-lifecycle")]
        state_dir: String,
        #[arg(long, default_value_t = 30)]
        warmup_seconds: u64,
        #[arg(long, default_value_t = 300)]
        restart_window_seconds: u64,
        #[arg(long, default_value_t = 1)]
        max_restarts_per_window: u32,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
}

/// `cynic node source ...` — source admission, materialization and bootstrap.
#[derive(Subcommand)]
pub enum NodeSourceCommand {
    /// Compose a non-mutating source-to-build-runner admission plan.
    Admission {
        #[arg(long)]
        request: String,
        #[arg(long)]
        observation: String,
        #[arg(long)]
        json: bool,
    },
    /// Plan or apply one digest-addressed source materialization on a build runner.
    Materialize {
        #[arg(long)]
        request: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long, default_value = "/var/lib/cynic/source/snapshots")]
        snapshot_root: String,
        #[arg(long)]
        snapshot_base_url: Option<String>,
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Observe, plan or materialize one registry-declared source repository.
    Bootstrap {
        #[arg(long)]
        node: String,
        #[arg(long)]
        repository: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        revision: String,
        #[arg(long, default_value = "/etc/cynic/registry.json")]
        registry: String,
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        #[arg(long)]
        agent_id: String,
        #[arg(long, default_value = "http://127.0.0.1:3030")]
        kernel_endpoint: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
}
