//! Shared model-readiness probing for local supervisors.
//!
//! `process` and `systemd --user` supervisors poll the same
//! `GET /v1/models` endpoint with the same matching policy; only the
//! transport (argv spawn vs unit lifecycle) differs. These pure helpers
//! are the single definition — adapters delegate, never duplicate.

use std::path::Path;
use std::time::Duration;

use crate::domain::local_runtime::LocalModelProfile;

/// Candidate identities a loaded model may report (alias, file name, stem),
/// lowercased and deduplicated.
pub(crate) fn expected_model_ids(profile: &LocalModelProfile) -> Vec<String> {
    let mut ids = Vec::new();
    ids.push(profile.alias.clone());

    let model_path = Path::new(&profile.model_path);
    if let Some(name) = model_path.file_name().and_then(|p| p.to_str()) {
        let stem = Path::new(name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(name);
        ids.push(name.to_string());
        ids.push(stem.to_string());
    }

    ids.into_iter()
        .map(|s| s.to_lowercase())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

/// Parse reported model ids from an OpenAI-style (`data[].id`) or
/// Ollama-style (`models[].{id,name,model}`) list body. Unknown shapes
/// yield an empty vec (unobservable), never an error.
pub(crate) fn parse_model_ids(body: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };

    let mut ids = Vec::new();
    if let Some(models) = v.get("data").and_then(|d| d.as_array()) {
        for m in models {
            if let Some(id) = m
                .get("id")
                .and_then(|x| x.as_str())
                .filter(|x| !x.is_empty())
            {
                ids.push(id.to_string());
            }
        }
    }

    if let Some(models) = v.get("models").and_then(|d| d.as_array()) {
        for m in models {
            let Some(id) = m
                .get("id")
                .or_else(|| m.get("name"))
                .or_else(|| m.get("model"))
                .and_then(|x| x.as_str())
                .filter(|x| !x.is_empty())
            else {
                continue;
            };
            ids.push(id.to_string());
        }
    }

    ids.sort();
    ids.dedup();
    ids
}

/// Fuzzy target match: exact or substring either way, case-insensitive.
/// Empty inputs never match.
pub(crate) fn model_matches_target(loaded: &[String], expected: &[String]) -> bool {
    if loaded.is_empty() || expected.is_empty() {
        return false;
    }
    let loaded_lc: Vec<String> = loaded.iter().map(|s| s.to_lowercase()).collect();
    for e in expected {
        let e_lc = e.to_lowercase();
        for l in &loaded_lc {
            if l == &e_lc || l.contains(&e_lc) || e_lc.contains(l) {
                return true;
            }
        }
    }
    false
}

/// Readiness timeout scales with weight size: 180 s base + 45 s/GiB,
/// capped at 1200 s.
pub(crate) fn estimate_model_ready_timeout(profile: &LocalModelProfile) -> Duration {
    let size_gb = profile.size_bytes.unwrap_or(0) as f64 / (1024.0 * 1024.0 * 1024.0);
    let extra = (size_gb * 45.0).round() as u64;
    let timeout_secs = 180u64 + extra;
    Duration::from_secs(timeout_secs.min(1200))
}
