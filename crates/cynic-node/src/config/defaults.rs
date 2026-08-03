pub(super) fn default_heartbeat_interval_secs() -> u64 {
    40
}

pub(super) fn default_context_size() -> u32 {
    4096
}

pub(super) fn default_dog_timeout_secs() -> u64 {
    60
}

pub(super) fn default_max_attempts() -> u32 {
    5
}

pub(super) fn default_initial_delay_secs() -> u64 {
    2
}

pub(super) fn default_max_delay_secs() -> u64 {
    120
}

pub(super) fn default_min_uptime_secs() -> u64 {
    10
}

pub(super) fn default_health_interval_secs() -> u64 {
    15
}

pub(super) fn default_verify_interval_secs() -> u64 {
    60
}

pub(super) fn default_health_timeout_secs() -> u64 {
    5
}

pub(super) fn default_node_control_interval_secs() -> u64 {
    30
}

pub(super) fn default_node_control_timeout_secs() -> u64 {
    10
}

pub(super) fn default_workload_delivery_lease_ttl_secs() -> u64 {
    120
}

pub(super) fn default_agent_workload_state_root() -> std::path::PathBuf {
    "/var/lib/cynic".into()
}

pub(super) fn default_runtime_artifact_inbox_root() -> std::path::PathBuf {
    "/var/lib/cynic/runtime-artifacts/inbox".into()
}

pub(super) fn default_runtime_artifact_max_bytes() -> u64 {
    4 * 1024 * 1024 * 1024
}

pub(super) fn default_runtime_artifact_timeout_secs() -> u64 {
    900
}

pub(super) fn default_runtime_registry_path() -> std::path::PathBuf {
    "/etc/cynic/registry.json".into()
}

pub(super) fn default_build_repository_id() -> String {
    "cynic".into()
}

pub(super) fn default_build_scheduler_interval_secs() -> u64 {
    5
}

pub(super) fn default_build_claim_ttl_secs() -> u64 {
    60
}

pub(super) fn default_build_command_timeout_secs() -> u64 {
    900
}

pub(super) fn default_release_signature_namespace() -> String {
    cynic_protocol::RUNTIME_RELEASE_SIGNATURE_NAMESPACE.to_string()
}

pub(super) fn default_release_publication_max_bytes() -> u64 {
    4 * 1024 * 1024 * 1024
}

pub(super) fn default_release_publication_timeout_secs() -> u64 {
    900
}

pub(super) fn default_enrollment_request_ttl_secs() -> u64 {
    600
}

pub(super) fn default_enrollment_renewal_window_secs() -> u64 {
    300
}

pub(super) fn default_max_failures() -> u32 {
    3
}

pub(super) fn default_startup_timeout_secs() -> u64 {
    120
}

pub(super) fn default_stop_timeout_secs() -> u64 {
    10
}

pub(super) fn default_watchdog_interval_secs() -> u64 {
    30
}

pub(super) fn default_watchdog_http_timeout_secs() -> u64 {
    5
}

pub(super) fn default_watchdog_registry_path() -> String {
    DEFAULT_REGISTRY_PATH.to_string()
}

pub(super) fn default_watchdog_constellation() -> String {
    "unassigned".to_string()
}

pub(super) fn default_watchdog_role() -> String {
    "node_watchdog".to_string()
}
use cynic_host::registry::DEFAULT_REGISTRY_PATH;
