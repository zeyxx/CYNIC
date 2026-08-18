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

/// Forge a parametrized vault profile (.env) from a benchmark report.
///
/// Closes the gap between a completed benchmark run and a launchable
/// profile: the winning run's `RuntimePlanSummary` (or its `ModelParameters`
/// fallback) is materialized as a `.env` file under `profiles_dir/`, with
/// llama-server flags reconstructed from the plan. The resulting alias can be
/// fed directly to `inference-start --yes <alias>`.
///
/// Selection policy for the "winning" run:
///   1. prefer the first `completed` report that carries a `runtime_launch_plan`
///   2. else the first `completed` report (plan reconstructed from params)
///   3. else the first report with any sample (best throughput)
///
/// Read-back proof: returns `ForgeProfileReport` naming the written path +
/// the alias + the plan that produced it.
#[derive(Debug, Clone, Serialize)]
pub struct ForgeProfileReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub plan_id: String,
    pub alias: String,
    pub profile_path: String,
    pub model_path: String,
    pub model_id: String,
    pub run_id: String,
    pub from_launch_plan: bool,
    pub extra_args: String,
    pub source: String,
    pub message: String,
}

impl ForgeProfileReport {
    pub const SCHEMA: &'static str = "cynic.inference.forge_profile.v1";
}

/// Load a `BenchmarkReport` from disk and forge a vault profile from its
/// winning run. `source_note` is stamped into the profile header.
pub fn forge_profile_from_benchmark_report(
    report_path: &str,
    source_note: &str,
) -> Result<ForgeProfileReport, LocalRuntimeError> {
    let raw = std::fs::read_to_string(report_path)
        .map_err(|e| LocalRuntimeError::Inventory(format!("read report {report_path}: {e}")))?;
    let report: super::agent_harness::BenchmarkReport = serde_json::from_str(&raw)
        .map_err(|e| LocalRuntimeError::Inventory(format!("parse report {report_path}: {e}")))?;
    forge_profile_from_benchmark(report, source_note)
}

/// In-memory entry point: pick the winning run and write its profile.
pub fn forge_profile_from_benchmark(
    report: super::agent_harness::BenchmarkReport,
    source_note: &str,
) -> Result<ForgeProfileReport, LocalRuntimeError> {
    let layout = ModelLayout::from_env();
    std::fs::create_dir_all(&layout.profiles_dir).map_err(|e| {
        LocalRuntimeError::Inventory(format!(
            "create profiles dir {}: {e}",
            layout.profiles_dir.display()
        ))
    })?;

    // Select champion run.
    let champion = select_champion(&report);
    let Some(champion) = champion else {
        return Err(LocalRuntimeError::Inventory(format!(
            "no usable report in plan {} — need at least one completed or sampled run",
            report.plan_id
        )));
    };

    let run_id = champion.run_id.clone();
    let params = &champion.scenario.model.parameters;
    let model_id = params.model_id.clone();
    let model_path = params.model_path.clone();
    // alias from model_id (stable, de-quantized); de-dup against existing profiles
    let alias = unique_alias(&layout, &alias_from_artifact_id(&model_id));

    // Build the .env body from the launch plan when available, else reconstruct
    // from ModelParameters (probe-agnostic fallback).
    let (extra_args, body) = match &champion.runtime_launch_plan {
        Some(plan) => {
            let extra_args = reconstruct_extra_args_from_plan(plan, params);
            (
                extra_args,
                build_profile_body(&alias, &model_path, plan, &extra_args, source_note),
            )
        }
        None => {
            let extra_args = reconstruct_extra_args_from_params(params);
            (
                extra_args,
                build_profile_body_from_params(
                    &alias,
                    &model_path,
                    params,
                    &extra_args,
                    source_note,
                ),
            )
        }
    };

    let path = layout.profiles_dir.join(format!("{alias}.env"));
    // Never silently overwrite an existing profile — surface for explicit review.
    if path.is_file() {
        return Err(LocalRuntimeError::Inventory(format!(
            "profile already exists: {}/{alias}.env — rename or remove before forging",
            layout.profiles_dir.display()
        )));
    }
    std::fs::write(&path, &body).map_err(|e| {
        LocalRuntimeError::Inventory(format!(
            "forge profile {}/{}: {e}",
            layout.profiles_dir.display(),
            path.file_name().unwrap().to_string_lossy()
        ))
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(ForgeProfileReport {
        schema: ForgeProfileReport::SCHEMA,
        status: "forged",
        plan_id: report.plan_id,
        alias,
        profile_path: path.display().to_string(),
        model_path,
        model_id,
        run_id,
        from_launch_plan: champion.runtime_launch_plan.is_some(),
        extra_args,
        source: source_note.to_string(),
        message: format!(
            "forged parametrized profile '{}' from benchmark plan '{}', run '{}' — ready for 'inference-start --yes {}",
            alias, report.plan_id, run_id, alias
        ),
    })
}

/// Choose the champion report: first completed w/ launch_plan, then first
/// completed, then highest-mean-output-throughput sample-bearing report.
fn select_champion(
    report: &super::agent_harness::BenchmarkReport,
) -> Option<super::agent_harness::AgentHarnessReport> {
    use super::agent_harness::AgentHarnessReport;
    // 1. first completed with a launch plan
    if let Some(champ) = report
        .reports
        .iter()
        .find(|r: &&AgentHarnessReport| r.status == "completed" && r.runtime_launch_plan.is_some())
    {
        return Some(champ.clone());
    }
    // 2. first completed
    if let Some(champ) = report
        .reports
        .iter()
        .find(|r: &&AgentHarnessReport| r.status == "completed")
    {
        return Some(champ.clone());
    }
    // 3. highest mean tokens/sec among reports with samples
    report
        .reports
        .iter()
        .filter(|r: &&AgentHarnessReport| !r.samples.is_empty())
        .max_by(|left: &&AgentHarnessReport, right: &&AgentHarnessReport| {
            let l = mean_output_rate(left);
            let r = mean_output_rate(right);
            l.partial_cmp(&r).unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}

fn mean_output_rate(report: &super::agent_harness::AgentHarnessReport) -> f64 {
    let rates: Vec<f64> = report
        .samples
        .iter()
        .filter_map(|s| s.output_tokens_per_second.map(|v| v as f64))
        .collect();
    if rates.is_empty() {
        0.0
    } else {
        rates.iter().sum::<f64>() / rates.len() as f64
    }
}

/// Reconstruct `--cache`, `--threads`, `--threads-batch`, `--n-gpu-layers`,
/// kv-cache quantization, `--flash-attn`, memory + CPU quota from a plan.
fn reconstruct_extra_args_from_plan(
    plan: &super::agent_harness::RuntimePlanSummary,
    params: &super::agent_harness::ModelParameters,
) -> String {
    let mut args: Vec<String> = Vec::new();
    if plan.threads > 0 {
        args.push(format!("--threads {}", plan.threads));
    }
    if plan.threads_batch > 0 {
        args.push(format!("--threads-batch {}", plan.threads_batch));
    }
    if plan.n_gpu_layers != 0 {
        args.push(format!("--n-gpu-layers {}", plan.n_gpu_layers));
    }
    if plan.flash_attn == "on" || plan.flash_attn == "true" {
        args.push("--flash-attn".to_string());
    }
    if let Some(k) = &plan.kv_cache_type_k {
        args.push(format!("--cache-type-k {k}"));
    }
    if let Some(v) = &plan.kv_cache_type_v {
        args.push(format!("--cache-type-v {v}"));
    }
    if let Some(bs) = plan.batch_size {
        if bs > 0 {
            args.push(format!("--batch-size {bs}"));
        }
    }
    if let Some(ub) = plan.ubatch_size {
        if ub > 0 {
            args.push(format!("--ubatch-size {ub}"));
        }
    }
    // KV-dims context: if a reasoning_budget / fit_mode surfaces, append as comment-safe
    if let Some(rb) = &params.reasoning_budget {
        args.push(format!("--reasoning-budget {rb}"));
    }
    if let Some(fit) = &params.fit_mode {
        args.push(format!("--fit-mode {fit}"));
    }
    if params.parallel_slots > 1 {
        args.push(format!("--parallel {}", params.parallel_slots));
    } else {
        args.push("--parallel 1".to_string());
    }
    args.join(" ")
}

/// Fallback reconstruction when no launch plan was captured (probe-less).
fn reconstruct_extra_args_from_params(params: &super::agent_harness::ModelParameters) -> String {
    let mut args: Vec<String> = Vec::new();
    if params.context_requested > 0 {
        args.push(format!("--ctx-size {}", params.context_requested));
    }
    if params.parallel_slots > 1 {
        args.push(format!("--parallel {}", params.parallel_slots));
    } else {
        args.push("--parallel 1".to_string());
    }
    if let Some(t) = params.threads.filter(|t| *t > 0) {
        args.push(format!("--threads {t}"));
    }
    if let Some(gpu) = params.gpu_layers.filter(|g| *g > 0) {
        args.push(format!("--n-gpu-layers {gpu}"));
    }
    if params.flash_attention == Some(true) {
        args.push("--flash-attn".to_string());
    }
    if let Some(k) = &params.kv_cache_type_k {
        args.push(format!("--cache-type-k {k}"));
    }
    if let Some(v) = &params.kv_cache_type_v {
        args.push(format!("--cache-type-v {v}"));
    }
    if let Some(rb) = &params.reasoning_budget {
        args.push(format!("--reasoning-budget {rb}"));
    }
    if let Some(fit) = &params.fit_mode {
        args.push(format!("--fit-mode {fit}"));
    }
    args.join(" ")
}

fn build_profile_body(
    alias: &str,
    model_path: &str,
    plan: &super::agent_harness::RuntimePlanSummary,
    extra_args: &str,
    source_note: &str,
) -> String {
    let memory_max = if plan.memory_max.is_empty() {
        "8G".to_string()
    } else {
        plan.memory_max.clone()
    };
    let cpu_quota = if plan.cpu_quota.is_empty() {
        "400%".to_string()
    } else {
        plan.cpu_quota.clone()
    };
    format!(
        r#"# CYNIC native profile — {source_note}
# forged from benchmark report: alias={alias}
# engine args derived from RuntimePlanSummary (run_id captured at forge time)
LLAMA_SERVER_BIN=/usr/local/bin/llama-server
LLAMA_MODEL_PATH={model_path}
LLAMA_HOST={host}
LLAMA_PORT={port}
LLAMA_CTX_SIZE={ctx}
LLAMA_THREADS={threads}
LLAMA_THREADS_BATCH={threads_batch}
LLAMA_N_GPU_LAYERS={n_gpu_layers}
LLAMA_MEMORY_MAX={memory_max}
LLAMA_CPU_QUOTA={cpu_quota}
LLAMA_EXTRA_ARGS={extra_args}
CYNIC_MODEL_ALIAS={alias}
"#,
        host = plan.host,
        port = plan.port,
        ctx = plan.ctx_size,
        threads = plan.threads,
        threads_batch = plan.threads_batch,
        n_gpu_layers = plan.n_gpu_layers,
        memory_max = memory_max,
        cpu_quota = cpu_quota,
        extra_args = extra_args,
    )
}

fn build_profile_body_from_params(
    alias: &str,
    model_path: &str,
    params: &super::agent_harness::ModelParameters,
    extra_args: &str,
    source_note: &str,
) -> String {
    let memory_max = std::env::var("LLAMA_MEMORY_MAX").unwrap_or_else(|_| "8G".to_string());
    let cpu_quota = std::env::var("LLAMA_CPU_QUOTA").unwrap_or_else(|_| "400%".to_string());
    format!(
        r#"# CYNIC native profile — {source_note}
# forged from benchmark report (no launch plan captured): alias={alias}
# engine args derived from ModelParameters fallback
LLAMA_SERVER_BIN=/usr/local/bin/llama-server
LLAMA_MODEL_PATH={model_path}
LLAMA_HOST=127.0.0.1
LLAMA_PORT=8080
LLAMA_CTX_SIZE={ctx}
LLAMA_THREADS={threads}
LLAMA_MEMORY_MAX={memory_max}
LLAMA_CPU_QUOTA={cpu_quota}
LLAMA_EXTRA_ARGS={extra_args}
CYNIC_MODEL_ALIAS={alias}
"#,
        ctx = params.context_requested,
        threads = params.threads.map(|t| t.to_string()).unwrap_or_default(),
        memory_max = memory_max,
        cpu_quota = cpu_quota,
        extra_args = extra_args,
    )
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
