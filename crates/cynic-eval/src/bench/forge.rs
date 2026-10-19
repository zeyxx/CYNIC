//! Forge a startable vault profile from a benchmark report.
//!
//! Moved from `cynic-inference::profile_bootstrap`: the decision (pick the
//! winning run from a `BenchmarkReport`) is eval-owned. The only serving
//! input is `profiles_dir` — a plain path supplied by the caller. This module
//! never reads serving env, layouts, or ports, so no `eval → inference` edge.

use std::fs;
use std::path::Path;

use crate::harness::BenchmarkReport;

/// Stable schema id for the forge profile report projection.
/// Kept byte-identical to the inference-era value (wire compat).
pub const FORGE_PROFILE_SCHEMA: &str = "cynic.inference.forge_profile.v1";

/// Report emitted by `forge_profile_from_benchmark_report`: the winning run
/// distilled into a single startable vault profile, ready for read-back proof
/// and a bounded hand-off to `inference-start`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ForgeProfileReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub plan_id: String,
    pub run_id: String,
    pub alias: String,
    pub model_id: String,
    pub model_path: String,
    pub profile_path: String,
    pub from_launch_plan: Option<String>,
    pub extra_args: Vec<String>,
    pub source: String,
    pub message: String,
}

/// Error channel for forge profile derivation.
#[derive(Debug, Clone)]
pub struct ForgeProfileError(pub String);

impl std::fmt::Display for ForgeProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ForgeProfileError {}

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

/// Derive the winning run from a `BenchmarkReport` (JSON file) and forge a
/// loopback vault `.env` profile for it under `profiles_dir`.
///
/// Pure decision + bounded action: read benchmark report → pick the sample with
/// the best `output_tokens_per_second` → write a loopback profile → return the
/// proof report (never auto-start the inference supervisor; that requires an
/// explicit `--yes` hand-off at the CLI surface).
pub fn forge_profile_from_benchmark_report(
    report_path: &str,
    source_note: &str,
    profiles_dir: &Path,
) -> Result<ForgeProfileReport, ForgeProfileError> {
    let raw = std::fs::read_to_string(report_path)
        .map_err(|e| ForgeProfileError(format!("read {report_path}: {e}")))?;
    let report: BenchmarkReport = serde_json::from_str(&raw)
        .map_err(|e| ForgeProfileError(format!("parse BenchmarkReport: {e}")))?;

    // Pick the winning run: the AgentHarnessReport with the best output rate.
    let winning = report
        .reports
        .iter()
        .filter_map(|hr| {
            let best = hr
                .samples
                .iter()
                .filter_map(|s| s.output_tokens_per_second)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            match best {
                Some(rate) => Some((hr, rate)),
                None => None,
            }
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let (winning_report, _) = winning.ok_or_else(|| {
        ForgeProfileError("no harness report with output_tokens_per_second samples".into())
    })?;

    let model_id = winning_report.scenario.model.parameters.model_id.clone();
    let model_path = winning_report.scenario.model.parameters.model_path.clone();
    let run_id = winning_report.run_id.clone();
    let alias = alias_from_artifact_id(&model_id);
    let profile_path = write_loopback_profile(profiles_dir, &alias, &model_path, source_note)
        .map_err(|e| ForgeProfileError(format!("write loopback profile: {e}")))?;

    let msg = if source_note.is_empty() {
        format!(
            "forged profile {alias} from winning run {run_id} of plan {}",
            report.plan_id
        )
    } else {
        format!(
            "{source_note}: forged profile {alias} from winning run {run_id} of plan {}",
            report.plan_id
        )
    };

    Ok(ForgeProfileReport {
        schema: FORGE_PROFILE_SCHEMA,
        status: "forged",
        plan_id: report.plan_id.clone(),
        run_id,
        alias,
        model_id,
        model_path,
        profile_path,
        from_launch_plan: None,
        extra_args: vec![],
        source: source_note.to_string(),
        message: msg,
    })
}

/// Write a loopback `.env` profile under `profiles_dir` (idempotent: an
/// existing profile is kept and its path returned).
pub fn write_loopback_profile(
    profiles_dir: &Path,
    alias: &str,
    model_path: &str,
    source_note: &str,
) -> Result<String, ForgeProfileError> {
    let path = profiles_dir.join(format!("{alias}.env"));
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
        ForgeProfileError(format!("write profile {}: {e}", path.display()))
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
}
