//! Native CYNIC local inference runtime types.
//!
//! CYNIC builds its **own** sovereign inference control plane: vault weights,
//! operator profiles, and process lifecycle for declared profiles.
//!
//! [Osmantic ODS](https://github.com/Osmantic/ODS) is a separate, maintained
//! framework used to bootstrap a full local AI environment on the host. Native
//! types here do **not** model ODS install/lifecycle. Integration with a
//! running ODS stack is `adapters::ods` (observe + publish into routing SSOT).

use serde::{Deserialize, Serialize};

/// Where a local model weight lives under CYNIC control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalModelSource {
    /// File under the sovereign model vault.
    Vault,
    /// Profile points at a path outside the vault (still operator-declared).
    ExternalPath,
}

/// Declared local model profile (operator intent, not a live probe).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelProfile {
    /// Stable alias used as systemd template instance and CLI key.
    pub alias: String,
    /// Absolute path to the GGUF (or other engine weight) file.
    pub model_path: String,
    pub source: LocalModelSource,
    pub model_exists: bool,
    pub size_bytes: Option<u64>,
    pub host: String,
    pub port: u16,
    pub ctx_size: u32,
    /// Extra argv fragment for the engine (opaque to domain).
    pub extra_args: String,
    pub memory_max: Option<String>,
    pub cpu_quota: Option<String>,
    /// True when profile is intended for loopback-only binding.
    pub loopback_only: bool,
}

/// Lifecycle state of the native local inference process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalRuntimeState {
    Inactive,
    Activating,
    Active,
    Failed,
    Unknown,
}

impl LocalRuntimeState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::Activating => "activating",
            Self::Active => "active",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

/// Observed native runtime instance (one slot on the default local port).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalRuntimeStatus {
    pub schema: &'static str,
    pub surface: &'static str,
    pub alias: Option<String>,
    pub state: LocalRuntimeState,
    pub unit: Option<String>,
    pub main_pid: Option<u32>,
    pub memory_current_bytes: Option<u64>,
    pub endpoint: Option<String>,
    pub listen_host: Option<String>,
    pub listen_port: Option<u16>,
    pub message: Option<String>,
}

impl LocalRuntimeStatus {
    pub const SCHEMA: &'static str = "cynic.inference.local_runtime_status.v1";
    pub const SURFACE: &'static str = "native";

    pub fn inactive(message: impl Into<String>) -> Self {
        Self {
            schema: Self::SCHEMA,
            surface: Self::SURFACE,
            alias: None,
            state: LocalRuntimeState::Inactive,
            unit: None,
            main_pid: None,
            memory_current_bytes: None,
            endpoint: None,
            listen_host: None,
            listen_port: None,
            message: Some(message.into()),
        }
    }
}

/// Machine-readable inventory of declared local models.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelInventory {
    pub schema: &'static str,
    pub surface: &'static str,
    pub vault_dir: String,
    pub profiles_dir: String,
    pub unit_template: String,
    pub models: Vec<LocalModelProfile>,
}

impl LocalModelInventory {
    pub const SCHEMA: &'static str = "cynic.inference.local_model_inventory.v1";
    pub const SURFACE: &'static str = "native";
}

/// Result of a native start/stop mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalRuntimeMutationReport {
    pub schema: &'static str,
    pub surface: &'static str,
    pub operation: String,
    pub alias: String,
    pub ok: bool,
    pub unit: String,
    pub status: LocalRuntimeStatus,
    pub message: Option<String>,
}

impl LocalRuntimeMutationReport {
    pub const SCHEMA: &'static str = "cynic.inference.local_runtime_mutation.v1";
    pub const SURFACE: &'static str = "native";
}
