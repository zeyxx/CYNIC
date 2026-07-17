//! Frictionless profile bootstrap from vault artifacts.
//!
//! Goal: operator drops GGUF into the principal vault (or hardlinks) →
//! `inference-start` / repair can start **without** hand-writing `.env` or
//! hardcoding a product model name.

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::adapters::local::FsModelStore;
use crate::domain::local_runtime::LocalModelProfile;
use crate::domain::model_layout::ModelLayout;
use crate::domain::model_store::{ModelArtifact, ModelLocationKind};
use crate::ports::local_runtime_port::LocalRuntimeError;
use crate::ports::model_store_port::ModelStorePort;

#[derive(Debug, Clone, Serialize)]
pub struct EnsureProfilesReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub created: Vec<String>,
    pub skipped_existing: Vec<String>,
    pub startable_aliases: Vec<String>,
    pub preferred_alias: Option<String>,
    pub vault_dir: String,
    pub profiles_dir: String,
    pub message: String,
}

impl EnsureProfilesReport {
    pub const SCHEMA: &'static str = "cynic.inference.ensure_profiles.v1";
}

/// Scan vault, auto-register missing loopback profiles, pick preferred start alias.
pub fn ensure_profiles_from_vault() -> Result<EnsureProfilesReport, LocalRuntimeError> {
    let layout = ModelLayout::from_env();
    let store = FsModelStore::with_layout(layout.clone());
    let report = store.scan()?;
    ensure_profiles_from_scan(&layout, &report.artifacts, &report.profiles)
}

pub fn ensure_profiles_from_scan(
    layout: &ModelLayout,
    artifacts: &[ModelArtifact],
    _profiles: &[LocalModelProfile],
) -> Result<EnsureProfilesReport, LocalRuntimeError> {
    fs::create_dir_all(&layout.profiles_dir).map_err(|e| {
        LocalRuntimeError::Inventory(format!(
            "create profiles dir {}: {e}",
            layout.profiles_dir.display()
        ))
    })?;

    let mut created = Vec::new();
    let mut skipped_existing = Vec::new();

    for art in artifacts {
        if !art.exists {
            continue;
        }
        // Only auto-bind vault weights (not random HF cache noise unless linked).
        if art.location != ModelLocationKind::Vault && art.profile_alias.is_none() {
            continue;
        }
        if art.profile_alias.is_some() {
            skipped_existing.push(art.id.clone());
            continue;
        }
        // Skip non-gguf for native llama path (safetensors needs different engine).
        if art.kind != "gguf" {
            continue;
        }
        let alias = unique_alias(layout, &alias_from_artifact_id(&art.id));
        write_loopback_profile(layout, &alias, &art.path, "auto-from-vault")?;
        created.push(alias);
    }

    // Re-scan profiles after writes
    let store = FsModelStore::with_layout(layout.clone());
    let again = store.scan()?;
    let startable: Vec<String> = again
        .profiles
        .iter()
        .filter(|p| p.model_exists && p.loopback_only)
        .map(|p| p.alias.clone())
        .collect();
    let preferred = select_preferred_alias(&again.profiles);

    let status = if preferred.is_some() { "ok" } else { "empty" };
    let message = if preferred.is_some() {
        "vault profiles ready; use preferred_alias for frictionless start".into()
    } else {
        "no startable loopback profile — place a .gguf under vault or download via inference-hf-download".into()
    };
    Ok(EnsureProfilesReport {
        schema: EnsureProfilesReport::SCHEMA,
        status,
        created,
        skipped_existing,
        startable_aliases: startable,
        preferred_alias: preferred,
        vault_dir: layout.vault_dir.display().to_string(),
        profiles_dir: layout.profiles_dir.display().to_string(),
        message,
    })
}

/// Prefer smallest existing loopback profile (fits more hosts); stable tie-break by alias.
pub fn select_preferred_alias(profiles: &[LocalModelProfile]) -> Option<String> {
    let mut candidates: Vec<&LocalModelProfile> = profiles
        .iter()
        .filter(|p| p.model_exists && p.loopback_only)
        .collect();
    if candidates.is_empty() {
        // any existing weight, even non-loopback (report only; start may reject)
        candidates = profiles.iter().filter(|p| p.model_exists).collect();
    }
    candidates.sort_by(|a, b| {
        let sa = a.size_bytes.unwrap_or(u64::MAX);
        let sb = b.size_bytes.unwrap_or(u64::MAX);
        sa.cmp(&sb).then_with(|| a.alias.cmp(&b.alias))
    });
    candidates.first().map(|p| p.alias.clone())
}

/// Resolve alias for start: explicit → auto ensure + preferred.
pub fn resolve_start_alias(explicit: Option<&str>) -> Result<String, LocalRuntimeError> {
    let explicit = explicit.map(str::trim).filter(|s| !s.is_empty());
    match explicit {
        Some(a) if !is_auto_token(a) => Ok(a.to_string()),
        _ => {
            let ensured = ensure_profiles_from_vault()?;
            ensured.preferred_alias.ok_or_else(|| {
                LocalRuntimeError::UnknownProfile(
                    "no startable model in vault — add a GGUF under the principal vault or pass an explicit alias".into(),
                )
            })
        }
    }
}

fn is_auto_token(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "auto" | "default" | "any" | "_" | "-"
    )
}

/// Stable alias from filename stem (agnostic — no product model list).
pub fn alias_from_artifact_id(id: &str) -> String {
    let mut s = id.to_ascii_lowercase();
    // strip common quant noise for shorter aliases
    for noise in [
        "-instruct",
        "-chat",
        "-q4_k_m",
        "-q5_k_m",
        "-q6_k",
        "-q8_0",
        "-q4_0",
        "-q4_1",
        "-q3_k_m",
        "-q2_k",
        "-f16",
        "-bf16",
        ".gguf",
    ] {
        s = s.replace(noise, "");
    }
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if ch == '-' || ch == '_' || ch == '.' {
            if !out.ends_with('-') {
                out.push('-');
            }
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "model".into()
    } else if out.len() > 48 {
        out.chars().take(48).collect()
    } else {
        out
    }
}

fn unique_alias(layout: &ModelLayout, base: &str) -> String {
    let mut alias = base.to_string();
    let mut n = 2u32;
    while layout.profiles_dir.join(format!("{alias}.env")).is_file() {
        alias = format!("{base}-{n}");
        n += 1;
        if n > 999 {
            break;
        }
    }
    alias
}

pub fn write_loopback_profile(
    layout: &ModelLayout,
    alias: &str,
    model_path: &str,
    source_note: &str,
) -> Result<String, LocalRuntimeError> {
    let path = layout.profiles_dir.join(format!("{alias}.env"));
    if path.is_file() {
        return Ok(path.display().to_string());
    }
    let body = format!(
        r#"# CYNIC native profile — {source_note}
# agnostic defaults; probe rewrites runtime/ on start
LLAMA_SERVER_BIN=/usr/local/bin/llama-server
LLAMA_MODEL_PATH={model_path}
LLAMA_HOST=127.0.0.1
LLAMA_PORT=8080
LLAMA_CTX_SIZE=8192
LLAMA_EXTRA_ARGS=--parallel 1 -ctk q4_0 -ctv q4_0 --flash-attn on
LLAMA_MEMORY_MAX=8G
LLAMA_CPU_QUOTA=400%
CYNIC_MODEL_ALIAS={alias}
"#
    );
    fs::write(&path, body).map_err(|e| {
        LocalRuntimeError::Inventory(format!("write profile {}: {e}", path.display()))
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    let _ = Path::new(model_path);
    Ok(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_from_id_is_stable_and_short() {
        let a = alias_from_artifact_id("Qwen_Qwen3-4B-Instruct-2507-Q4_K_M");
        assert!(a.contains("qwen"));
        assert!(!a.contains("q4_k_m"));
        assert!(a.len() <= 48);
    }

    #[test]
    fn preferred_picks_smallest_existing() {
        let profiles = vec![
            LocalModelProfile {
                alias: "big".into(),
                model_path: "/v/big.gguf".into(),
                source: crate::domain::local_runtime::LocalModelSource::Vault,
                model_exists: true,
                size_bytes: Some(9_000_000_000),
                host: "127.0.0.1".into(),
                port: 8080,
                ctx_size: 4096,
                extra_args: String::new(),
                memory_max: None,
                cpu_quota: None,
                loopback_only: true,
            },
            LocalModelProfile {
                alias: "small".into(),
                model_path: "/v/small.gguf".into(),
                source: crate::domain::local_runtime::LocalModelSource::Vault,
                model_exists: true,
                size_bytes: Some(1_000_000_000),
                host: "127.0.0.1".into(),
                port: 8080,
                ctx_size: 4096,
                extra_args: String::new(),
                memory_max: None,
                cpu_quota: None,
                loopback_only: true,
            },
        ];
        assert_eq!(select_preferred_alias(&profiles).as_deref(), Some("small"));
    }
}
