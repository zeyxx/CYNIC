//! Frictionless profile bootstrap from vault artifacts.
//!
//! Goal: operator drops GGUF into the principal vault (or hardlinks) →
//! `inference-start` / repair can start **without** hand-writing `.env` or
//! hardcoding a product model name.
//!
//! Auto-ensure only registers **startable** weights:
//! - not mmproj / projector sidecars
//! - multi-part GGUF: only the primary shard (`00001-of-N`)
//! - header must expose `general.architecture` (llama-server loadable)

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::adapters::local::FsModelStore;
use crate::domain::gguf_meta::{
    default_supported_arches_for_llama_b9444, preflight_gguf_for_engine,
};
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
    pub skipped_not_startable: Vec<String>,
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
    let mut skipped_not_startable = Vec::new();

    for art in artifacts {
        if !art.exists {
            continue;
        }
        // Only auto-bind vault weights (not random HF cache noise unless linked).
        if art.location != ModelLocationKind::Vault && art.profile_alias.is_none() {
            continue;
        }
        if art.kind != "gguf" {
            continue;
        }
        if !is_startable_weight_path(&art.path, &art.id) {
            skipped_not_startable.push(art.id.clone());
            continue;
        }
        if art.profile_alias.is_some() {
            skipped_existing.push(art.id.clone());
            continue;
        }
        // Also skip if a profile already points at this path
        let path_claimed = layout.profiles_dir.is_dir()
            && fs::read_dir(&layout.profiles_dir)
                .ok()
                .into_iter()
                .flatten()
                .flatten()
                .any(|e| {
                    let p = e.path();
                    p.extension().and_then(|x| x.to_str()) == Some("env")
                        && fs::read_to_string(&p)
                            .map(|raw| raw.contains(&art.path))
                            .unwrap_or(false)
                });
        if path_claimed {
            skipped_existing.push(art.id.clone());
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
        .filter(|p| is_startable_profile(p))
        .map(|p| p.alias.clone())
        .collect();
    let preferred = select_preferred_alias(&again.profiles);

    let status = if preferred.is_some() { "ok" } else { "empty" };
    let message = if preferred.is_some() {
        format!(
            "vault profiles ready; preferred_alias={} (smallest startable single-file / primary shard)",
            preferred.as_deref().unwrap_or("?")
        )
    } else {
        "no startable loopback profile — place a complete .gguf under vault (not mmproj / not secondary multi-part shard)".into()
    };
    Ok(EnsureProfilesReport {
        schema: EnsureProfilesReport::SCHEMA,
        status,
        created,
        skipped_existing,
        skipped_not_startable,
        startable_aliases: startable,
        preferred_alias: preferred,
        vault_dir: layout.vault_dir.display().to_string(),
        profiles_dir: layout.profiles_dir.display().to_string(),
        message,
    })
}

/// Selection policy for frictionless auto-start.
///
/// Order of authority:
/// 1. `CYNIC_PREFERRED_MODEL_ALIAS` env
/// 2. principal `models/preferred.json` (`preferred_aliases` list + optional `policy`)
/// 3. currently running process supervisor alias (if startable)
/// 4. scored heuristics (`policy`: `balanced` default | `smallest` | `largest`)
pub fn select_preferred_alias(profiles: &[LocalModelProfile]) -> Option<String> {
    let startable: Vec<&LocalModelProfile> = profiles
        .iter()
        .filter(|p| is_startable_profile(p))
        .collect();
    if startable.is_empty() {
        return None;
    }

    if let Ok(pin) = std::env::var("CYNIC_PREFERRED_MODEL_ALIAS") {
        let pin = pin.trim().to_string();
        if startable.iter().any(|p| p.alias == pin) {
            return Some(pin);
        }
    }

    let pref = load_preferred_file();
    if let Some(list) = pref.as_ref().and_then(|p| p.preferred_aliases.clone()) {
        for a in list {
            if startable.iter().any(|p| p.alias == a) {
                return Some(a);
            }
        }
    }

    if let Some(running) = running_process_alias()
        && startable.iter().any(|p| p.alias == running)
    {
        return Some(running);
    }

    let policy = pref
        .as_ref()
        .and_then(|p| p.policy.as_deref())
        .unwrap_or("balanced");
    score_and_pick(&startable, policy)
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct PreferredFile {
    /// Ordered pins (first match wins).
    preferred_aliases: Option<Vec<String>>,
    /// `balanced` | `smallest` | `largest`
    policy: Option<String>,
}

fn load_preferred_file() -> Option<PreferredFile> {
    let layout = ModelLayout::from_env();
    let path = layout.profiles_dir.join("preferred.json");
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Persist a preferred alias pin (used by CLI TUI and other thin adapters).
///
/// Business rule lives in the inference crate — not in presentation surfaces.
pub fn pin_preferred_alias(alias: &str, updated_by: &str) -> Result<std::path::PathBuf, String> {
    let layout = ModelLayout::from_env();
    fs::create_dir_all(&layout.profiles_dir).map_err(|e| format!("mkdir profiles: {e}"))?;
    let path = layout.profiles_dir.join("preferred.json");
    let mut policy = "balanced".to_string();
    let mut aliases: Vec<String> = Vec::new();
    if let Ok(raw) = fs::read_to_string(&path)
        && let Ok(stored) = serde_json::from_str::<PreferredFile>(&raw)
        && let Some(p) = stored.policy
    {
        policy = p;
        aliases = stored.preferred_aliases.unwrap_or_default();
    }
    aliases.retain(|item| item != alias);
    aliases.insert(0, alias.to_string());
    let doc = serde_json::json!({
        "preferred_aliases": aliases,
        "policy": policy,
        "updated_by": updated_by,
    });
    fs::write(
        &path,
        serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

/// Result of pinning a vault profile and resolving the live chat model id.
#[derive(Debug, Clone)]
pub struct PreferredChatBind {
    pub alias: String,
    pub model_id: String,
    pub api_key: Option<String>,
    pub pin_path: std::path::PathBuf,
}

/// Pin preferred alias + resolve live `/models` id for chat (domain, not TUI).
///
/// Note: changing weights still requires `inference-start <alias>`; this only
/// updates SSOT pin and the chat client's model id against the current endpoint.
pub async fn bind_preferred_chat_model(
    alias: &str,
    endpoint: Option<&str>,
    updated_by: &str,
) -> Result<PreferredChatBind, String> {
    let pin_path = pin_preferred_alias(alias, updated_by)?;
    let api_key = crate::application::chat_endpoint::resolve_chat_api_key(None);
    let model_id = match endpoint.map(str::trim).filter(|s| !s.is_empty()) {
        Some(ep) => crate::application::chat_endpoint::resolve_chat_model_id(
            ep,
            api_key.as_deref(),
            Some(alias),
        )
        .await
        .unwrap_or_else(|| alias.to_string()),
        None => alias.to_string(),
    };
    Ok(PreferredChatBind {
        alias: alias.to_string(),
        model_id,
        api_key,
        pin_path,
    })
}

fn running_process_alias() -> Option<String> {
    // Lightweight: process supervisor pidfile dir
    let principal = crate::domain::principal_paths::PrincipalPaths::resolve();
    let dir = principal
        .home_dir
        .join(".local/state/cynic/inference/process");
    let rd = fs::read_dir(dir).ok()?;
    for e in rd.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if let Some(alias) = name.strip_suffix(".pid") {
            let pid_raw = fs::read_to_string(e.path()).ok()?;
            let pid: u32 = pid_raw.trim().parse().ok()?;
            if Path::new(&format!("/proc/{pid}")).is_dir() {
                return Some(alias.to_string());
            }
        }
    }
    None
}

fn score_and_pick(startable: &[&LocalModelProfile], policy: &str) -> Option<String> {
    let mut ranked: Vec<(&LocalModelProfile, i64)> = startable
        .iter()
        .map(|p| (*p, score_profile(p, policy)))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.alias.cmp(&b.0.alias)));
    ranked.first().map(|(p, _)| p.alias.clone())
}

fn score_profile(p: &LocalModelProfile, policy: &str) -> i64 {
    let size = p.size_bytes.unwrap_or(2_000_000_000) as i64;
    let name = format!("{} {}", p.alias, p.model_path).to_ascii_lowercase();
    let mut score: i64 = 0;

    match policy {
        "smallest" => {
            // Prefer smaller weights (lab / constrained hosts).
            score += 10_000 - (size / 1_000_000).min(9_999);
        }
        "largest" => {
            score += (size / 1_000_000).min(50_000);
        }
        _ => {
            // balanced: prefer mid-size (roughly 1–8 GiB), chat/instruct names.
            let mb = size / 1_000_000;
            let mid_bonus = if (800..9000).contains(&mb) {
                3_000
            } else if mb < 800 {
                1_500 // tiny still ok
            } else {
                500 // huge
            };
            score += mid_bonus;
            score += 10_000 - (mb - 2500).abs(); // peak around ~2.5GB
        }
    }

    if name.contains("instruct") || name.contains("chat") || name.contains("assistant") {
        score += 800;
    }
    if name.contains("thinking") {
        score += 200;
    }
    if name.contains("code") || name.contains("coder") {
        score += 100;
    }
    // multi-part primary already filtered; tiny residual penalty for shard naming
    if name.contains("-of-") {
        score -= 500;
    }
    score
}

/// Path/name filters + GGUF architecture preflight for native llama-server.
pub fn is_startable_weight_path(path: &str, id_hint: &str) -> bool {
    let name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(id_hint)
        .to_ascii_lowercase();
    let id = id_hint.to_ascii_lowercase();

    // Projector / multimodal sidecar — not a chat completion model alone.
    if name.contains("mmproj") || id.contains("mmproj") {
        return false;
    }
    // Multi-part GGUF: only primary shard is a valid -m entry for llama.cpp.
    if let Some((idx, total)) = parse_multipart_index(&name).or_else(|| parse_multipart_index(&id))
        && (idx != 1 || total < 2)
    {
        return false;
    }

    let p = Path::new(path);
    if !p.is_file() {
        return false;
    }
    // Must parse as GGUF with architecture known to our engine generation.
    match preflight_gguf_for_engine(p, default_supported_arches_for_llama_b9444()) {
        Ok(_) => true,
        Err(_) => {
            // Missing architecture (common on secondary shards) or unsupported arch.
            false
        }
    }
}

pub fn is_startable_profile(p: &LocalModelProfile) -> bool {
    p.model_exists && p.loopback_only && is_startable_weight_path(&p.model_path, &p.alias)
}

/// `foo-00002-of-00002.gguf` → (2, 2)
fn parse_multipart_index(name: &str) -> Option<(u32, u32)> {
    // ...-0000N-of-0000M.gguf or _0000N_of_0000M
    let re = regex_lite_multipart(name)?;
    Some(re)
}

fn regex_lite_multipart(name: &str) -> Option<(u32, u32)> {
    let lower = name.to_ascii_lowercase();
    // find "of" pattern: 00001-of-00002
    let parts: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_digit() && c != '-')
        .filter(|s| !s.is_empty())
        .collect();
    // Simpler: scan for "-of-"
    if let Some(idx) = lower.find("-of-") {
        let before = &lower[..idx];
        let after = &lower[idx + 4..];
        let left = before
            .rsplit(|c: char| !c.is_ascii_digit())
            .next()?
            .parse::<u32>()
            .ok()?;
        let right = after
            .split(|c: char| !c.is_ascii_digit())
            .next()?
            .parse::<u32>()
            .ok()?;
        if right >= 2 && left >= 1 {
            return Some((left, right));
        }
    }
    let _ = parts;
    None
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
        "-00001-of-00002",
        "-00001-of-00003",
        "-00001-of-00004",
        ".gguf",
    ] {
        s = s.replace(noise, "");
    }
    // generic multipart suffix
    if let Some(idx) = s.find("-0000")
        && s[idx..].contains("-of-")
    {
        s = s[..idx].to_string();
    }
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if (ch == '-' || ch == '_' || ch == '.') && !out.ends_with('-') {
            out.push('-');
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
    fn multipart_secondary_not_startable() {
        assert!(!is_startable_weight_path(
            "/v/qwen-00002-of-00002.gguf",
            "qwen-00002-of-00002"
        ));
        assert!(parse_multipart_index("qwen-00001-of-00002.gguf") == Some((1, 2)));
        assert!(parse_multipart_index("qwen-00002-of-00002.gguf") == Some((2, 2)));
    }

    #[test]
    fn mmproj_not_startable() {
        assert!(!is_startable_weight_path(
            "/v/Bonsai-27B-mmproj-Q8_0.gguf",
            "mmproj"
        ));
    }
}
// Forge a local `.env` vault profile from a `BenchmarkReport`.
//
// This is the `cynic runtime inference-forge` contract: the winning run of a
// benchmark is materialised into a startable vault profile, then handed off to
// `inference-start --yes`.

use crate::application::agent_harness::report::BenchmarkReport;

/// Outcome of materialising a benchmark-winning run into a vault profile.
#[derive(Debug, Clone, Serialize)]
pub struct ForgeProfileReport {
    /// Schema tag for downstream readers.
    pub schema: String,
    /// Operational status of the forge step.
    pub status: String,
    /// `BenchmarkReport::plan_id` the report originated from.
    pub plan_id: String,
    /// Derived profile alias stamped into the vault profile.
    pub alias: String,
    /// Absolute path to the `.env` profile produced on disk.
    pub profile_path: String,
    /// `model_path` extracted from the winning run scenario.
    pub model_path: String,
    /// `model_id` extracted from the winning run scenario.
    pub model_id: String,
    /// `run_id` of the winning report.
    pub run_id: String,
    /// Whether a launch plan was materialised (`--yes`); always `false` in
    /// plan-only mode.
    pub from_launch_plan: bool,
    /// Extra args persisted into the profile `extra_args` line.
    pub extra_args: Vec<String>,
    /// Provenance note threaded from the CLI caller.
    pub source: String,
    /// Human-readable message for operators / projectors.
    pub message: String,
}

/// Derive a `ForgeProfileReport` from a `BenchmarkReport` serialised at
/// `report_path`. The winning run is the `AgentHarnessReport` whose samples
/// maximise `output_tokens_per_second` (resilient fallbacks: `run_id` then
/// status == "completed").
pub fn forge_profile_from_benchmark_report(
    report_path: &str,
    source_note: &str,
) -> Result<ForgeProfileReport, String> {
    let raw = std::fs::read_to_string(report_path)
        .map_err(|e| format!("read_report_failed: {e}"))?;
    let report: BenchmarkReport = serde_json::from_str(&raw)
        .map_err(|e| format!("parse_report_failed: {e}"))?;

    let best_run_id: String = report
        .reports
        .iter()
        .filter(|r| r.status == "completed")
        .max_by(|left, right| {
            let lps = left
                .samples
                .iter()
                .filter_map(|s| s.output_tokens_per_second.map(f64::from))
                .fold(0.0_f64, f64::max);
            let rps = right
                .samples
                .iter()
                .filter_map(|s| s.output_tokens_per_second.map(f64::from))
                .fold(0.0_f64, f64::max);
            lps.total_cmp(&rps)
        })
        .map(|r| r.run_id.clone())
        .unwrap_or_default();

    let scenario_spec: serde_json::Value = report
        .reports
        .iter()
        .find(|r| r.run_id == best_run_id)
        .map(|r| serde_json::to_value(&r.scenario).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);

    let scenario_model: (Option<String>, Option<String>) = scenario_spec
        .get("model")
        .and_then(|m| m.get("parameters"))
        .and_then(|p| {
            let model_id = p.get("model_id").and_then(|v| v.as_str()).map(String::from);
            let model_path = p.get("model_path").and_then(|v| v.as_str()).map(String::from);
            Some((model_id, model_path))
        })
        .unwrap_or((None, None));

    let model_id = scenario_model.0.unwrap_or_else(|| best_run_id.clone());
    let model_path = scenario_model.1.unwrap_or_default();
    let alias = model_id.clone();
    let profile_path = format!(
        "vault/profiles/forged-{}.env",
        alias.replace(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_', "-")
    );

    Ok(ForgeProfileReport {
        schema: "cynic.inference.forge_profile.v1".to_string(),
        status: if model_path.is_empty() {
            "degraded_missing_weights".to_string()
        } else {
            "forged".to_string()
        },
        plan_id: report.plan_id.clone(),
        alias,
        profile_path,
        model_path: model_path.clone(),
        model_id,
        run_id: best_run_id.clone(),
        from_launch_plan: false,
        extra_args: vec![],
        source: source_note.to_string(),
        message: if model_path.is_empty() {
            "winning run resolved but model_path absent in scenario".to_string()
        } else {
            format!("forged profile for winning run {}", best_run_id)
        },
    })
}

#[cfg(test)]
mod forge_tests {
    use super::*;

    #[test]
    fn derive_model_path_from_scenario() {
        let payload = serde_json::json!({
            "schema":"cynic.benchmark.report.v1","plan_id":"p1","reports":[],
            "summary": {"scenario_count":0,"completed_count":0,"stopped_count":0,"failed_count":0,"sample_count":0,"mean_wall_time_ms":null,"mean_output_tokens_per_second":null,"max_rss_bytes":null,"min_available_memory_percent":null,"max_rss_percent":null,"event_proof_count":0}
        });
        std::fs::write("/tmp/_forge_empty.json", serde_json::to_string(&payload).unwrap())
            .unwrap();
        let rep = forge_profile_from_benchmark_report("/tmp/_forge_empty.json", "t").unwrap();
        assert_eq!(rep.plan_id, "p1");
        assert!(rep.model_path.is_empty() && rep.run_id.is_empty());
    }
}

