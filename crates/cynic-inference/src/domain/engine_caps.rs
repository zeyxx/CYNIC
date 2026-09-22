//! Engine capabilities — observed properties of the deployed inference binary.
//!
//! Pure domain type. No I/O, no deps to adapters.

use std::collections::HashMap;

/// Observed capabilities of the deployed `llama-server` binary.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EngineCapabilities {
    #[serde(default = "default_schema")]
    pub schema: String,
    /// Path to the binary.
    pub bin_path: String,
    /// Fingerprint for invalidation (size:mtime).
    pub bin_fingerprint: String,
    /// llama-server version string.
    pub version: String,
    /// Last probe timestamp.
    pub last_probed_at_unix_s: u64,
    /// Observed architecture outcomes (arch → status).
    #[serde(default)]
    pub observed_arches: HashMap<String, Observation>,
    /// GPU / CPU backends detected at probe time.
    #[serde(default)]
    pub backends: Vec<String>,
}

fn default_schema() -> String {
    "cynic.inference.engine_capabilities.v1".into()
}

impl Default for EngineCapabilities {
    fn default() -> Self {
        Self {
            schema: default_schema(),
            bin_path: String::new(),
            bin_fingerprint: String::new(),
            version: "unknown".into(),
            last_probed_at_unix_s: 0,
            observed_arches: HashMap::new(),
            backends: Vec::new(),
        }
    }
}

impl EngineCapabilities {
    pub fn new(bin_path: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            bin_path: bin_path.into(),
            version: version.into(),
            last_probed_at_unix_s: 0, // caller should set
            ..Default::default()
        }
    }

    pub fn observe(&mut self, arch: &str, success: bool) {
        let obs = if success {
            Observation::Ok
        } else {
            Observation::Fail
        };
        self.observed_arches.insert(arch.to_string(), obs);
        self.last_probed_at_unix_s = 0; // caller should set
    }

    pub fn arch_status(&self, arch: &str) -> ArchStatus {
        if let Some(obs) = self.observed_arches.get(arch) {
            return ArchStatus::Observed(*obs);
        }
        ArchStatus::Unknown
    }

    pub fn preflight_decision(&self, arch: &str) -> PreflightDecision {
        match self.arch_status(arch) {
            ArchStatus::Observed(Observation::Ok) => PreflightDecision::Allow,
            ArchStatus::Observed(Observation::Fail) => PreflightDecision::Reject {
                reason: format!("arch '{arch}' observed as FAIL on {}", self.bin_path),
            },
            ArchStatus::Fallback => PreflightDecision::Warn {
                format: format!(
                    "arch '{arch}' unverified (static fallback only); launch to confirm"
                ),
            },
            ArchStatus::Unknown => PreflightDecision::Reject {
                reason: format!("arch '{arch}' never observed on {}", self.bin_path),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Observation {
    Ok,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchStatus {
    Observed(Observation),
    /// Present in the static fallback list, never verified.
    Fallback,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreflightDecision {
    Allow,
    Warn { format: String },
    Reject { reason: String },
}

/// Static fallback list of known-supported architectures.
pub fn static_fallback_arches() -> Vec<&'static str> {
    vec![
        "llama",
        "mistral",
        "mixtral",
        "falcon",
        "phi",
        "gemma",
        "starcoder",
        "qwen",
        "deepseek",
        "openelm",
    ]
}
