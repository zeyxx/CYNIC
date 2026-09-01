//! Canonical contracts for agent hosting outside Cortex (Slice C0.2).
//!
//! Defined in `infra/AGENT_FLEET_HOSTING_DESIGN.md` and `.hermes/plans/2026-09-01-agent-host-proxmox-sandbox-first.md`.
//! Pure types and predicates without I/O.

use serde::{Deserialize, Serialize};

use crate::agent_isolation::SandboxStrategy;

pub const AGENT_HOST_DESIRED_STATE_SCHEMA: &str = "cynic.agent_host.desired_state.v1";
pub const AGENT_SANDBOX_ALLOCATION_SCHEMA: &str = "cynic.agent_sandbox.allocation.v1";
pub const AGENT_SANDBOX_OBSERVATION_SCHEMA: &str = "cynic.agent_sandbox.observation.v1";
pub const AGENT_SANDBOX_RECEIPT_SCHEMA: &str = "cynic.agent_sandbox.receipt.v1";
pub const NOTIFICATION_OUTBOX_ENTRY_SCHEMA: &str = "cynic.notification.outbox_entry.v1";

/// Harness execution class for the dedicated agent host VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentHostHarness {
    /// Conformance testing and validation harness.
    Conformance,
    /// Production persistent agent supervisor harness.
    Production,
}

/// Desired state for the dedicated agent host VM on Proxmox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentHostDesired {
    pub schema: String,
    pub role: String,
    pub harness: AgentHostHarness,
    pub principal: String,
    pub host_node_id: String,
    pub hypervisor_target: String,
    pub vmid: u32,
    pub vcpus: u32,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
    pub storage_pool: String,
    pub network_plane: String,
    pub network_bridge: String,
    pub network_ip: Option<String>,
    pub allowed_sandbox_strategies: Vec<SandboxStrategy>,
    pub default_egress_policy: String,
    pub cortex_autonomous: bool,
}

impl Default for AgentHostDesired {
    fn default() -> Self {
        Self {
            schema: AGENT_HOST_DESIRED_STATE_SCHEMA.to_string(),
            role: "agent_host".to_string(),
            harness: AgentHostHarness::Conformance,
            principal: "cynic".to_string(),
            host_node_id: "cynic-agent-host".to_string(),
            hypervisor_target: "cynic-host-proxmox".to_string(),
            vmid: 102,
            vcpus: 2,
            memory_bytes: 4 * 1024 * 1024 * 1024,
            disk_bytes: 32 * 1024 * 1024 * 1024,
            storage_pool: "local-lvm".to_string(),
            network_plane: "synapse".to_string(),
            network_bridge: "vmbr0".to_string(),
            network_ip: Some("10.10.10.102/24".to_string()),
            allowed_sandbox_strategies: vec![
                SandboxStrategy::BwrapEphemeral,
                SandboxStrategy::LxcUnprivileged,
            ],
            default_egress_policy: "default_deny".to_string(),
            cortex_autonomous: true,
        }
    }
}

/// Mount specification inside an agent sandbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxMountSpec {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

/// Network phase and lifecycle within the sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxNetworkPhase {
    /// Read-only egress to Forgejo during bootstrap fetch of declared SHA.
    BootstrapForgejo,
    /// Absolute zero-egress during agent execution (default-deny).
    ZeroEgressAgent,
    /// Local proxy Unix domain socket for local inference only.
    ProxyInferenceUds,
}

/// Allocation contract for one agent sandbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSandboxAllocation {
    pub schema: String,
    pub allocation_id: String,
    pub workload_id: String,
    pub agent_id: String,
    pub host_vm_id: String,
    pub strategy: SandboxStrategy,
    pub isolation_digest: String,
    pub ttl_seconds: u64,
    pub max_actions: u32,
    pub max_memory_bytes: u64,
    pub max_disk_bytes: u64,
    pub private_git_dir: String,
    pub private_workspace_path: String,
    pub repository_id: String,
    pub forgejo_base_sha: String,
    pub target_branch: String,
    pub mounts: Vec<SandboxMountSpec>,
    pub network_phase: SandboxNetworkPhase,
    pub allocated_at_unix_ms: u64,
}

/// Observation contract for an active or completed sandbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSandboxObservation {
    pub schema: String,
    pub workload_id: String,
    pub agent_id: String,
    pub vm_identity: String,
    pub supervisor_pid: u32,
    pub cgroup_path: Option<String>,
    pub observed_mounts: Vec<SandboxMountSpec>,
    pub observed_git_dir: String,
    pub observed_workspace_path: String,
    pub observed_branch: String,
    pub observed_head_sha: String,
    pub observed_network_phase: SandboxNetworkPhase,
    pub observed_at_unix_ms: u64,
    pub unknowns: Vec<String>,
}

/// Resource consumption metrics for a sandbox execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxResourceUsage {
    pub cpu_time_ms: u64,
    pub peak_memory_bytes: u64,
    pub disk_written_bytes: u64,
}

/// Final disposition of the sandbox workspace and artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxDisposition {
    Retained,
    Quarantined,
    EligibleForHumanReview,
}

/// Durable receipt produced after sandbox termination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSandboxReceipt {
    pub schema: String,
    pub receipt_id: String,
    pub allocation_digest: String,
    pub workload_id: String,
    pub agent_id: String,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    pub duration_ms: u64,
    pub observed_resource_usage: SandboxResourceUsage,
    pub disposition: SandboxDisposition,
    pub cleanup_evidence_refs: Vec<String>,
    pub no_secrets_verified: bool,
}

/// Target notification channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    Telegram,
    Console,
    File,
}

/// Classification of notification payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationContentClassification {
    PublicRedacted,
    OperatorPrivate,
}

/// Nature of the referenced event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationReferencedEventKind {
    WorkloadAdmitted,
    WorkloadStarted,
    WorkloadCompleted,
    BuildVerified,
    BuildFailed,
    WorkloadBlocked,
}

/// Reference to the runtime event that produced the notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationReferencedEvent {
    pub kind: NotificationReferencedEventKind,
    pub event_id: String,
    pub workload_id: String,
    pub commit_sha: Option<String>,
    pub build_verdict: Option<String>,
    pub proof_ref: Option<String>,
}

/// Delivery status of the outbox entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationDeliveryStatus {
    Pending,
    Delivered,
    Failed,
}

/// Outbox entry for non-authoritative operator notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationOutboxEntry {
    pub schema: String,
    pub notification_id: String,
    pub referenced_event: NotificationReferencedEvent,
    pub channel: NotificationChannel,
    pub classification: NotificationContentClassification,
    pub idempotency_key: String,
    pub summary: String,
    pub delivery_status: NotificationDeliveryStatus,
    pub delivery_receipt: Option<String>,
    pub created_at_unix_ms: u64,
}

// ---------------------------------------------------------------------------
// Predicates and Invariant Validation
// ---------------------------------------------------------------------------

fn valid_sha256_hex(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn valid_sha1_hex(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Validates the desired state declaration for the agent host VM.
pub fn validate_agent_host_desired(desired: &AgentHostDesired) -> Vec<String> {
    let mut violations = Vec::new();

    if desired.schema != AGENT_HOST_DESIRED_STATE_SCHEMA {
        violations.push("agent_host_desired_schema_invalid".to_string());
    }
    if desired.role != "agent_host" {
        violations.push("agent_host_role_must_be_agent_host".to_string());
    }
    if desired.principal.trim().is_empty() {
        violations.push("agent_host_principal_missing".to_string());
    }
    if desired.host_node_id.trim().is_empty() {
        violations.push("agent_host_node_id_missing".to_string());
    }
    if desired.hypervisor_target.trim().is_empty() {
        violations.push("agent_host_hypervisor_target_missing".to_string());
    }
    if desired.vmid == 0 {
        violations.push("agent_host_vmid_zero".to_string());
    }
    if desired.vcpus == 0 {
        violations.push("agent_host_vcpus_zero".to_string());
    }
    if desired.memory_bytes < 1024 * 1024 * 1024 {
        violations.push("agent_host_memory_below_1gib_minimum".to_string());
    }
    if desired.disk_bytes < 10 * 1024 * 1024 * 1024 {
        violations.push("agent_host_disk_below_10gib_minimum".to_string());
    }
    if desired.storage_pool.trim().is_empty() {
        violations.push("agent_host_storage_pool_missing".to_string());
    }
    if desired.network_bridge.trim().is_empty() {
        violations.push("agent_host_network_bridge_missing".to_string());
    }
    if desired.allowed_sandbox_strategies.is_empty() {
        violations.push("agent_host_allowed_sandbox_strategies_empty".to_string());
    }
    if !desired.cortex_autonomous {
        violations.push("agent_host_must_be_cortex_autonomous".to_string());
    }

    violations
}

/// Validates an agent sandbox allocation contract.
pub fn validate_agent_sandbox_allocation(allocation: &AgentSandboxAllocation) -> Vec<String> {
    let mut violations = Vec::new();

    if allocation.schema != AGENT_SANDBOX_ALLOCATION_SCHEMA {
        violations.push("agent_sandbox_allocation_schema_invalid".to_string());
    }
    if allocation.allocation_id.trim().is_empty() {
        violations.push("agent_sandbox_allocation_id_missing".to_string());
    }
    if allocation.workload_id.trim().is_empty() {
        violations.push("agent_sandbox_workload_id_missing".to_string());
    }
    if allocation.agent_id.trim().is_empty() {
        violations.push("agent_sandbox_agent_id_missing".to_string());
    }
    if !valid_sha256_hex(&allocation.isolation_digest) {
        violations.push("agent_sandbox_isolation_digest_invalid".to_string());
    }
    if allocation.ttl_seconds == 0 {
        violations.push("agent_sandbox_ttl_zero".to_string());
    }
    if allocation.max_actions == 0 {
        violations.push("agent_sandbox_max_actions_zero".to_string());
    }
    if allocation.max_memory_bytes == 0 {
        violations.push("agent_sandbox_max_memory_zero".to_string());
    }
    if allocation.max_disk_bytes == 0 {
        violations.push("agent_sandbox_max_disk_zero".to_string());
    }

    // Git and Workspace isolation invariants
    if !valid_sha1_hex(&allocation.forgejo_base_sha) {
        violations.push("agent_sandbox_forgejo_base_sha_invalid".to_string());
    }

    let branch = allocation.target_branch.trim();
    if branch.is_empty() {
        violations.push("agent_sandbox_target_branch_missing".to_string());
    } else if branch == "main" || branch == "master" || branch == "v1" || branch == "HEAD" {
        violations.push("agent_sandbox_target_branch_forbidden_protected_branch".to_string());
    }

    let git_dir = allocation.private_git_dir.trim();
    let workspace = allocation.private_workspace_path.trim();

    if git_dir.is_empty() {
        violations.push("agent_sandbox_private_git_dir_missing".to_string());
    }
    if workspace.is_empty() {
        violations.push("agent_sandbox_private_workspace_path_missing".to_string());
    }
    if git_dir == workspace {
        violations.push("agent_sandbox_git_dir_equals_workspace_path".to_string());
    }

    // Shared git admin detection (reject shared checkout paths)
    let shared_markers = [
        "/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC",
        "/var/lib/cynic/source/repositories/CYNIC",
        "/.git",
    ];
    for marker in shared_markers {
        if git_dir.starts_with(marker) || workspace.starts_with(marker) {
            violations.push("blocked_shared_git_admin".to_string());
            break;
        }
    }

    // Mount security: non-read-only mounts must strictly target inside private workspace
    let forbidden_rw_prefixes = [
        "/etc",
        "/usr",
        "/var/lib/cynic/source",
        "/var/lib/cynic/identity",
        "/root",
        "/home",
        "/bin",
        "/sbin",
        "/lib",
    ];

    for mount in &allocation.mounts {
        if !mount.read_only {
            for prefix in forbidden_rw_prefixes {
                if mount.target == prefix || mount.target.starts_with(&format!("{prefix}/")) {
                    violations.push("blocked_excessive_rw_mount".to_string());
                    break;
                }
            }
        }
    }

    violations
}

/// Validates an observation of a running or terminated agent sandbox.
pub fn validate_agent_sandbox_observation(
    observation: &AgentSandboxObservation,
    allocation: &AgentSandboxAllocation,
) -> Vec<String> {
    let mut violations = Vec::new();

    if observation.schema != AGENT_SANDBOX_OBSERVATION_SCHEMA {
        violations.push("agent_sandbox_observation_schema_invalid".to_string());
    }
    if observation.workload_id != allocation.workload_id {
        violations.push("agent_sandbox_observation_workload_id_mismatch".to_string());
    }
    if observation.agent_id != allocation.agent_id {
        violations.push("agent_sandbox_observation_agent_id_mismatch".to_string());
    }
    if observation.supervisor_pid == 0 {
        violations.push("agent_sandbox_observation_supervisor_pid_zero".to_string());
    }
    if observation.observed_git_dir != allocation.private_git_dir {
        violations.push("agent_sandbox_observation_git_dir_mismatch".to_string());
    }
    if observation.observed_workspace_path != allocation.private_workspace_path {
        violations.push("agent_sandbox_observation_workspace_path_mismatch".to_string());
    }
    if observation.observed_branch != allocation.target_branch {
        violations.push("agent_sandbox_observation_branch_mismatch".to_string());
    }
    if !valid_sha1_hex(&observation.observed_head_sha) {
        violations.push("agent_sandbox_observation_head_sha_invalid".to_string());
    }

    violations
}

/// Validates a durable sandbox completion receipt.
pub fn validate_agent_sandbox_receipt(receipt: &AgentSandboxReceipt) -> Vec<String> {
    let mut violations = Vec::new();

    if receipt.schema != AGENT_SANDBOX_RECEIPT_SCHEMA {
        violations.push("agent_sandbox_receipt_schema_invalid".to_string());
    }
    if receipt.receipt_id.trim().is_empty() {
        violations.push("agent_sandbox_receipt_id_missing".to_string());
    }
    if !valid_sha256_hex(&receipt.allocation_digest) {
        violations.push("agent_sandbox_receipt_allocation_digest_invalid".to_string());
    }
    if receipt.workload_id.trim().is_empty() {
        violations.push("agent_sandbox_receipt_workload_id_missing".to_string());
    }
    if receipt.agent_id.trim().is_empty() {
        violations.push("agent_sandbox_receipt_agent_id_missing".to_string());
    }
    if receipt.completed_at_unix_ms < receipt.started_at_unix_ms {
        violations.push("agent_sandbox_receipt_completion_precedes_start".to_string());
    }
    if !receipt.no_secrets_verified {
        violations.push("agent_sandbox_receipt_secrets_not_verified_absent".to_string());
    }

    violations
}

/// Validates an outbox entry destined for notifications (e.g. Telegram).
pub fn validate_notification_outbox_entry(entry: &NotificationOutboxEntry) -> Vec<String> {
    let mut violations = Vec::new();

    if entry.schema != NOTIFICATION_OUTBOX_ENTRY_SCHEMA {
        violations.push("notification_outbox_entry_schema_invalid".to_string());
    }
    if entry.notification_id.trim().is_empty() {
        violations.push("notification_id_missing".to_string());
    }
    if entry.idempotency_key.trim().is_empty() {
        violations.push("notification_idempotency_key_missing".to_string());
    }
    if entry.summary.trim().is_empty() {
        violations.push("notification_summary_empty".to_string());
    }

    // Ensure no secrets or credentials leaked in notification summaries
    let secret_patterns = [
        "token",
        "secret",
        "password",
        "BEGIN OPENSSH PRIVATE KEY",
        "BEGIN RSA PRIVATE KEY",
        "id_ed25519",
        "api_key",
        "Bearer",
    ];
    for pattern in secret_patterns {
        if entry
            .summary
            .to_lowercase()
            .contains(&pattern.to_lowercase())
        {
            violations.push("blocked_unredacted_notification_summary".to_string());
            break;
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_desired() -> AgentHostDesired {
        AgentHostDesired::default()
    }

    fn valid_allocation() -> AgentSandboxAllocation {
        AgentSandboxAllocation {
            schema: AGENT_SANDBOX_ALLOCATION_SCHEMA.to_string(),
            allocation_id: "alloc-001".to_string(),
            workload_id: "workload-c0-test".to_string(),
            agent_id: "codex:node-a:1".to_string(),
            host_vm_id: "cynic-agent-host".to_string(),
            strategy: SandboxStrategy::BwrapEphemeral,
            isolation_digest: format!("sha256:{}", "a".repeat(64)),
            ttl_seconds: 3600,
            max_actions: 50,
            max_memory_bytes: 2 * 1024 * 1024 * 1024,
            max_disk_bytes: 10 * 1024 * 1024 * 1024,
            private_git_dir: "/var/lib/cynic/sandboxes/workload-c0-test/repo.git".to_string(),
            private_workspace_path: "/var/lib/cynic/sandboxes/workload-c0-test/workspace"
                .to_string(),
            repository_id: "cynic".to_string(),
            forgejo_base_sha: "0123456789abcdef0123456789abcdef01234567".to_string(),
            target_branch: "agent/codex/c0-test-slice".to_string(),
            mounts: vec![
                SandboxMountSpec {
                    source: "/var/lib/cynic/sandboxes/workload-c0-test/workspace".to_string(),
                    target: "/workspace".to_string(),
                    read_only: false,
                },
                SandboxMountSpec {
                    source: "/usr".to_string(),
                    target: "/usr".to_string(),
                    read_only: true,
                },
            ],
            network_phase: SandboxNetworkPhase::ZeroEgressAgent,
            allocated_at_unix_ms: 1788267750000,
        }
    }

    fn valid_observation(alloc: &AgentSandboxAllocation) -> AgentSandboxObservation {
        AgentSandboxObservation {
            schema: AGENT_SANDBOX_OBSERVATION_SCHEMA.to_string(),
            workload_id: alloc.workload_id.clone(),
            agent_id: alloc.agent_id.clone(),
            vm_identity: "cynic-agent-host".to_string(),
            supervisor_pid: 12345,
            cgroup_path: Some("/sys/fs/cgroup/cynic.slice/cynic-sandbox-001.scope".to_string()),
            observed_mounts: alloc.mounts.clone(),
            observed_git_dir: alloc.private_git_dir.clone(),
            observed_workspace_path: alloc.private_workspace_path.clone(),
            observed_branch: alloc.target_branch.clone(),
            observed_head_sha: alloc.forgejo_base_sha.clone(),
            observed_network_phase: alloc.network_phase,
            observed_at_unix_ms: 1788267751000,
            unknowns: vec![],
        }
    }

    fn valid_receipt(alloc: &AgentSandboxAllocation) -> AgentSandboxReceipt {
        AgentSandboxReceipt {
            schema: AGENT_SANDBOX_RECEIPT_SCHEMA.to_string(),
            receipt_id: "rcpt-001".to_string(),
            allocation_digest: alloc.isolation_digest.clone(),
            workload_id: alloc.workload_id.clone(),
            agent_id: alloc.agent_id.clone(),
            started_at_unix_ms: 1788267750000,
            completed_at_unix_ms: 1788267850000,
            duration_ms: 100000,
            observed_resource_usage: SandboxResourceUsage {
                cpu_time_ms: 1250,
                peak_memory_bytes: 512 * 1024 * 1024,
                disk_written_bytes: 4096000,
            },
            disposition: SandboxDisposition::Retained,
            cleanup_evidence_refs: vec!["bwrap:proc:exit_0".to_string()],
            no_secrets_verified: true,
        }
    }

    fn valid_outbox_entry() -> NotificationOutboxEntry {
        NotificationOutboxEntry {
            schema: NOTIFICATION_OUTBOX_ENTRY_SCHEMA.to_string(),
            notification_id: "notif-001".to_string(),
            referenced_event: NotificationReferencedEvent {
                kind: NotificationReferencedEventKind::WorkloadCompleted,
                event_id: "evt-001".to_string(),
                workload_id: "workload-c0-test".to_string(),
                commit_sha: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
                build_verdict: Some("passed".to_string()),
                proof_ref: Some("proof://build/001".to_string()),
            },
            channel: NotificationChannel::Telegram,
            classification: NotificationContentClassification::PublicRedacted,
            idempotency_key: "notif-idem-001".to_string(),
            summary: "Workload workload-c0-test completed successfully. Build verdict: passed."
                .to_string(),
            delivery_status: NotificationDeliveryStatus::Pending,
            delivery_receipt: None,
            created_at_unix_ms: 1788267855000,
        }
    }

    #[test]
    fn valid_desired_state_passes_validation() {
        let desired = valid_desired();
        assert!(validate_agent_host_desired(&desired).is_empty());
    }

    #[test]
    fn valid_sandbox_allocation_passes_validation() {
        let alloc = valid_allocation();
        assert!(validate_agent_sandbox_allocation(&alloc).is_empty());
    }

    #[test]
    fn valid_observation_passes_validation() {
        let alloc = valid_allocation();
        let obs = valid_observation(&alloc);
        assert!(validate_agent_sandbox_observation(&obs, &alloc).is_empty());
    }

    #[test]
    fn valid_receipt_passes_validation() {
        let alloc = valid_allocation();
        let rcpt = valid_receipt(&alloc);
        assert!(validate_agent_sandbox_receipt(&rcpt).is_empty());
    }

    #[test]
    fn valid_outbox_entry_passes_validation() {
        let entry = valid_outbox_entry();
        assert!(validate_notification_outbox_entry(&entry).is_empty());
    }

    #[test]
    fn rejects_shared_git_admin_path() {
        let mut alloc = valid_allocation();
        alloc.private_workspace_path =
            "/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/worktree-1".to_string();
        let violations = validate_agent_sandbox_allocation(&alloc);
        assert!(violations.contains(&"blocked_shared_git_admin".to_string()));
    }

    #[test]
    fn rejects_main_and_master_branches() {
        let mut alloc = valid_allocation();
        alloc.target_branch = "main".to_string();
        let violations = validate_agent_sandbox_allocation(&alloc);
        assert!(
            violations
                .contains(&"agent_sandbox_target_branch_forbidden_protected_branch".to_string())
        );

        alloc.target_branch = "master".to_string();
        let violations = validate_agent_sandbox_allocation(&alloc);
        assert!(
            violations
                .contains(&"agent_sandbox_target_branch_forbidden_protected_branch".to_string())
        );
    }

    #[test]
    fn rejects_invalid_forgejo_base_sha() {
        let mut alloc = valid_allocation();
        alloc.forgejo_base_sha = "short_sha".to_string();
        let violations = validate_agent_sandbox_allocation(&alloc);
        assert!(violations.contains(&"agent_sandbox_forgejo_base_sha_invalid".to_string()));
    }

    #[test]
    fn rejects_excessive_rw_mounts_on_system_dirs() {
        let mut alloc = valid_allocation();
        alloc.mounts.push(SandboxMountSpec {
            source: "/etc".to_string(),
            target: "/etc".to_string(),
            read_only: false,
        });
        let violations = validate_agent_sandbox_allocation(&alloc);
        assert!(violations.contains(&"blocked_excessive_rw_mount".to_string()));
    }

    #[test]
    fn rejects_observation_with_mismatched_workload_or_branch() {
        let alloc = valid_allocation();
        let mut obs = valid_observation(&alloc);
        obs.observed_branch = "agent/other/diverged".to_string();
        let violations = validate_agent_sandbox_observation(&obs, &alloc);
        assert!(violations.contains(&"agent_sandbox_observation_branch_mismatch".to_string()));
    }

    #[test]
    fn rejects_outbox_entry_leaking_secrets() {
        let mut entry = valid_outbox_entry();
        entry.summary = "Workload done. Forgejo API token: secret_tok_12345".to_string();
        let violations = validate_notification_outbox_entry(&entry);
        assert!(violations.contains(&"blocked_unredacted_notification_summary".to_string()));
    }

    #[test]
    fn rejects_receipt_without_secrets_verified_absent() {
        let alloc = valid_allocation();
        let mut rcpt = valid_receipt(&alloc);
        rcpt.no_secrets_verified = false;
        let violations = validate_agent_sandbox_receipt(&rcpt);
        assert!(
            violations.contains(&"agent_sandbox_receipt_secrets_not_verified_absent".to_string())
        );
    }
}
