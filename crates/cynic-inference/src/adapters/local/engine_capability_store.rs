//! Observed llama-server capabilities (version, supported arches, backends).
//!
//! SSOT for what the deployed engine actually supports — learned a posteriori
//! from launch outcomes, not hardcoded. Invalidation on binary change.
//!
//! This is an adapter — it imports domain types and provides I/O.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// ── Domain types (pure, no I/O) ──
pub use crate::domain::engine_caps::{
    ArchStatus, EngineCapabilities, Observation, PreflightDecision,
};

/// Default static fallback — covers llama.cpp b9444+ documented arches.
pub fn static_fallback_arches() -> &'static [&'static str] {
    &[
        "llama",
        "qwen2",
        "qwen2vl",
        "qwen3",
        "qwen35",
        "qwen35moe",
        "phi3",
        "phi2",
        "gemma",
        "gemma2",
        "gemma3",
        "mistral",
        "mixtral",
        "command-r",
        "deepseek2",
        "deepseek",
        "gpt2",
        "gptneox",
        "starcoder",
        "refact",
        "bert",
        "nomic-bert",
        "rwkv",
        "rwkv6",
        "mamba",
        "jamba",
        "cohere",
        "dbrx",
        "olmo",
        "arctic",
        "chatglm",
        "glm4",
        "stablelm",
        "internlm2",
        "minicpm",
        "minicpm3",
        "granite",
        "openelm",
    ]
}

/// Compute binary fingerprint (size:mtime) for invalidation.
pub fn bin_fingerprint(path: &Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    let size = meta.len();
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(format!("size:{size}:mtime:{mtime}"))
}

fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// SSOT persistence for engine capabilities.
#[derive(Debug)]
pub struct EngineCapabilityStore {
    cache_path: PathBuf,
    caps: EngineCapabilities,
}

impl EngineCapabilityStore {
    /// Load the default store from the canonical cache path.
    pub fn load_default() -> Option<EngineCapabilities> {
        let cache_dir = crate::infra::principal_paths::PrincipalPaths::resolve().cache_dir;
        let path = cache_dir.join("engine_capabilities.json");
        if path.exists() {
            Some(Self::load(path).capabilities().clone())
        } else {
            None
        }
    }

    /// Load from disk or create a new empty store.
    pub fn load(cache_path: PathBuf) -> Self {
        let caps = fs::read_to_string(&cache_path)
            .ok()
            .and_then(|s| serde_json::from_str::<EngineCapabilities>(&s).ok())
            .unwrap_or_else(EngineCapabilities::default);
        Self { cache_path, caps }
    }

    /// Check if the binary has changed since last probe (invalidates).
    pub fn is_stale(&self, bin_path: &Path) -> bool {
        match bin_fingerprint(bin_path) {
            Some(fp) => self.caps.bin_fingerprint != fp,
            None => true,
        }
    }

    /// Refresh fingerprint + version from the live binary.
    pub fn refresh_fingerprint(&mut self, bin_path: &Path) {
        if let Some(fp) = bin_fingerprint(bin_path) {
            self.caps.bin_fingerprint = fp;
        }
        // Version is not re-parsed here (requires subprocess); caller sets it.
    }

    /// Update version after a probe.
    pub fn set_version(&mut self, version: impl Into<String>) {
        self.caps.version = version.into();
        self.caps.last_probed_at_unix_s = now_unix_s();
    }

    /// Record a launch outcome.
    pub fn observe(&mut self, arch: &str, success: bool) {
        self.caps.observe(arch, success);
    }

    /// Save to disk atomically (write temp + rename).
    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.caps).map_err(|e| e.to_string())?;
        let tmp = self.cache_path.with_extension("tmp");
        fs::write(&tmp, json).map_err(|e| format!("{}: {}", tmp.display(), e))?;
        fs::rename(&tmp, &self.cache_path)
            .map_err(|e| format!("rename {}: {}", self.cache_path.display(), e))?;
        Ok(())
    }

    pub fn capabilities(&self) -> &EngineCapabilities {
        &self.caps
    }

    pub fn capabilities_mut(&mut self) -> &mut EngineCapabilities {
        &mut self.caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_updates_status() {
        let mut caps = EngineCapabilities::new("/usr/local/bin/llama-server", "b9600");
        caps.observe("qwen35moe", true);
        caps.observe("dspark", false);

        assert_eq!(
            caps.arch_status("qwen35moe"),
            ArchStatus::Observed(Observation::Ok)
        );
        assert_eq!(
            caps.arch_status("dspark"),
            ArchStatus::Observed(Observation::Fail)
        );
        assert_eq!(caps.arch_status("llama"), ArchStatus::Fallback);
        assert_eq!(caps.arch_status("nonexistent"), ArchStatus::Unknown);
    }

    #[test]
    fn preflight_decision_mapping() {
        let mut caps = EngineCapabilities::new("/usr/local/bin/llama-server", "b9600");
        caps.observe("qwen35moe", true);
        caps.observe("dspark", false);

        assert_eq!(
            caps.preflight_decision("qwen35moe"),
            PreflightDecision::Allow
        );
        assert_eq!(
            caps.preflight_decision("llama"),
            PreflightDecision::Warn {
                format: "arch 'llama' unverified (static fallback only); launch to confirm".into()
            }
        );
        assert_eq!(
            caps.preflight_decision("dspark"),
            PreflightDecision::Reject {
                reason: "arch 'dspark' observed as FAIL on /usr/local/bin/llama-server".into(),
            }
        );
        assert_eq!(
            caps.preflight_decision("nonexistent"),
            PreflightDecision::Reject {
                reason: "arch 'nonexistent' never observed and not in fallback; launch required"
                    .into(),
            }
        );
    }

    #[test]
    fn fingerprint_format() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("fake-bin");
        fs::write(&p, b"hello").unwrap();
        let fp = bin_fingerprint(&p).unwrap();
        assert!(fp.starts_with("size:5:mtime:"));
    }
}
