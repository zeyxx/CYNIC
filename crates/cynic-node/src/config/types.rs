use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use super::defaults;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub(crate) kernel: KernelConfig,
    pub(crate) dog: DogConfig,
    pub(crate) process: ProcessConfig,
    #[serde(default)]
    pub(crate) restart: RestartConfig,
    #[serde(default)]
    pub(crate) health: HealthConfig,
    #[serde(default)]
    pub(crate) federation: Option<FederationConfig>,
    #[serde(default)]
    pub(crate) node_control: Option<NodeControlConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NodeControlConfig {
    pub(crate) kernel_url: String,
    pub(crate) node_id: String,
    pub(crate) signing_key_path: PathBuf,
    pub(crate) enrollment_request_path: PathBuf,
    pub(crate) enrollment_grant_path: PathBuf,
    pub(crate) enrollment_authority_public_key_path: PathBuf,
    pub(crate) enrollment_token_path: PathBuf,
    pub(crate) enrollment_seed_id: String,
    pub(crate) enrollment_role: String,
    #[serde(default = "defaults::default_agent_workload_state_root")]
    pub(crate) agent_workload_state_root: PathBuf,
    #[serde(default = "defaults::default_runtime_artifact_inbox_root")]
    pub(crate) runtime_artifact_inbox_root: PathBuf,
    #[serde(default = "defaults::default_runtime_artifact_max_bytes")]
    pub(crate) runtime_artifact_max_bytes: u64,
    #[serde(default = "defaults::default_runtime_artifact_timeout_secs")]
    pub(crate) runtime_artifact_timeout_secs: u64,
    #[serde(default)]
    pub(crate) enrollment_requested_capabilities: Vec<String>,
    #[serde(default = "defaults::default_enrollment_request_ttl_secs")]
    pub(crate) enrollment_request_ttl_secs: u64,
    #[serde(default = "defaults::default_enrollment_renewal_window_secs")]
    pub(crate) enrollment_renewal_window_secs: u64,
    #[serde(default = "defaults::default_node_control_interval_secs")]
    pub(crate) interval_secs: u64,
    #[serde(default = "defaults::default_node_control_timeout_secs")]
    pub(crate) timeout_secs: u64,
    #[serde(default = "defaults::default_workload_delivery_lease_ttl_secs")]
    pub(crate) workload_delivery_lease_ttl_secs: u64,
    #[serde(default)]
    pub(crate) observed_capabilities: Vec<String>,
    #[serde(default)]
    pub(crate) observed_services: Vec<String>,
    #[serde(default = "defaults::default_runtime_registry_path")]
    pub(crate) registry_path: std::path::PathBuf,
    #[serde(default)]
    pub(crate) build_scheduler: Option<BuildSchedulerConfig>,
    #[serde(default = "defaults::default_builder_listen_addr")]
    pub(crate) builder_listen_addr: std::net::SocketAddr,
    #[serde(default = "defaults::default_workspace_root")]
    pub(crate) workspace_root: std::path::PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BuildSchedulerConfig {
    #[serde(default = "defaults::default_runtime_registry_path")]
    pub(crate) registry_path: PathBuf,
    #[serde(default = "defaults::default_build_repository_id")]
    pub(crate) repository_id: String,
    #[serde(default = "defaults::default_build_scheduler_interval_secs")]
    pub(crate) interval_secs: u64,
    #[serde(default = "defaults::default_build_claim_ttl_secs")]
    pub(crate) claim_ttl_secs: u64,
    #[serde(default = "defaults::default_build_command_timeout_secs")]
    pub(crate) command_timeout_secs: u64,
    #[serde(default)]
    pub(crate) release_publication: Option<ReleasePublicationConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ReleasePublicationConfig {
    pub(crate) artifact_base_url: String,
    pub(crate) authentication_kind: String,
    pub(crate) work_root: PathBuf,
    pub(crate) signing_key_path: PathBuf,
    pub(crate) credential_path: PathBuf,
    pub(crate) signer_identity: String,
    #[serde(default = "defaults::default_release_signature_namespace")]
    pub(crate) signature_namespace: String,
    #[serde(default = "defaults::default_release_publication_max_bytes")]
    pub(crate) max_bytes: u64,
    #[serde(default = "defaults::default_release_publication_timeout_secs")]
    pub(crate) timeout_secs: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NodeControlRuntimeConfig {
    pub(crate) node_control: NodeControlConfig,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WatchdogConfig {
    pub(crate) kernel: KernelConfig,
    pub(crate) node: NodeConfig,
    #[serde(default)]
    pub(crate) watchdog: WatchdogRuntimeConfig,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NodeConfig {
    pub(crate) name: String,
    #[serde(default = "defaults::default_watchdog_role")]
    pub(crate) role: String,
    #[serde(default = "defaults::default_watchdog_constellation")]
    pub(crate) constellation: String,
    #[serde(default)]
    pub(crate) services: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WatchdogRuntimeConfig {
    #[serde(default = "defaults::default_watchdog_interval_secs")]
    pub(crate) interval_secs: u64,
    #[serde(default = "defaults::default_watchdog_http_timeout_secs")]
    pub(crate) http_timeout_secs: u64,
    #[serde(default = "defaults::default_watchdog_registry_path")]
    pub(crate) registry_path: String,
}

impl Default for WatchdogRuntimeConfig {
    fn default() -> Self {
        Self {
            interval_secs: defaults::default_watchdog_interval_secs(),
            http_timeout_secs: defaults::default_watchdog_http_timeout_secs(),
            registry_path: defaults::default_watchdog_registry_path(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct KernelConfig {
    pub(crate) url: String,
    pub(crate) api_key_env: String,
    #[serde(skip)]
    pub(crate) api_key: String,
    #[serde(default = "defaults::default_heartbeat_interval_secs")]
    pub(crate) heartbeat_interval_secs: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DogConfig {
    pub(crate) name: String,
    pub(crate) model: String,
    pub(crate) base_url: String,
    #[serde(default = "defaults::default_context_size")]
    pub(crate) context_size: u32,
    #[serde(default = "defaults::default_dog_timeout_secs")]
    pub(crate) timeout_secs: u64,
    pub(crate) api_key_env: Option<String>,
    #[serde(skip)]
    pub(crate) api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProcessConfig {
    pub(crate) command: Vec<String>,
    pub(crate) working_dir: Option<String>,
    #[serde(default = "defaults::default_stop_timeout_secs")]
    pub(crate) stop_timeout_secs: u64,
    #[serde(default)]
    pub(crate) env: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RestartConfig {
    #[serde(default = "defaults::default_max_attempts")]
    pub(crate) max_attempts: u32,
    #[serde(default = "defaults::default_initial_delay_secs")]
    pub(crate) initial_delay_secs: u64,
    #[serde(default = "defaults::default_max_delay_secs")]
    pub(crate) max_delay_secs: u64,
    #[serde(default = "defaults::default_min_uptime_secs")]
    pub(crate) min_uptime_secs: u64,
}

impl Default for RestartConfig {
    fn default() -> Self {
        Self {
            max_attempts: defaults::default_max_attempts(),
            initial_delay_secs: defaults::default_initial_delay_secs(),
            max_delay_secs: defaults::default_max_delay_secs(),
            min_uptime_secs: defaults::default_min_uptime_secs(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct HealthConfig {
    #[serde(default = "defaults::default_health_interval_secs")]
    pub(crate) interval_secs: u64,
    #[serde(default = "defaults::default_verify_interval_secs")]
    pub(crate) verify_interval_secs: u64,
    #[serde(default = "defaults::default_health_timeout_secs")]
    pub(crate) timeout_secs: u64,
    #[serde(default = "defaults::default_max_failures")]
    pub(crate) max_failures: u32,
    #[serde(default = "defaults::default_startup_timeout_secs")]
    pub(crate) startup_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FederationConfig {
    pub(crate) api_key_env: Option<String>,
    pub(crate) api_key: Option<String>,
    pub(crate) node_public_key_env: Option<String>,
    pub(crate) node_public_key: Option<String>,
    pub(crate) queue_dir: Option<PathBuf>,
    pub(crate) kernel_ws_url: Option<String>,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            interval_secs: defaults::default_health_interval_secs(),
            verify_interval_secs: defaults::default_verify_interval_secs(),
            timeout_secs: defaults::default_health_timeout_secs(),
            max_failures: defaults::default_max_failures(),
            startup_timeout_secs: defaults::default_startup_timeout_secs(),
        }
    }
}
