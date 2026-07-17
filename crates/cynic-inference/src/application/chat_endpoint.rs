//! Frictionless chat endpoint resolution via **CYNIC SSOT + DNS**, not operator env as law.
//!
//! Order (first usable wins):
//! 1. Breakglass `CYNIC_CHAT_ENDPOINT` (explicit operator override only)
//! 2. CYNIC inference config (`inference.toml` — /etc then principal config)
//! 3. Synapse DNS names (`inference.synapse.cynic`, `llama.synapse.cynic`, …)
//! 4. ODS / local OpenAI-compat discovery (when `--ods` or as last probe)
//!
//! Never silently falls back to mock.

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::adapters::ods::{ODS_SURFACE, ODS_UPSTREAM, discover_ods_services};
use crate::config_loader::{
    InferenceConfigError, LoadedInferenceConfig, load_inference_config_from_path_public,
};
use crate::domain::model_layout::default_ods_probe_bases;
use crate::domain::principal_paths::PrincipalPaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatEndpointSource {
    /// Explicit CYNIC_CHAT_ENDPOINT breakglass.
    Env,
    /// From CYNIC inference.toml SSOT.
    InferenceConfig,
    /// Resolved via synapse DNS network.
    Dns,
    /// Osmantic ODS discovery adapter.
    Ods,
    /// Local OpenAI-compat probe (loopback ports).
    NativeProbe,
    Mock,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEndpointResolution {
    pub schema: &'static str,
    pub ready: bool,
    pub source: ChatEndpointSource,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub backend_name: Option<String>,
    pub detail: String,
    pub candidates: Vec<String>,
    pub dns_zone: String,
    pub config_path: Option<String>,
}

const DEFAULT_DNS_ZONE: &str = "synapse.cynic";

/// Resolve chat backend. `prefer_ods` elevates ODS discovery before loopback-only probe.
pub async fn resolve_chat_endpoint(prefer_ods: bool) -> ChatEndpointResolution {
    let dns_zone = std::env::var("CYNIC_DNS_ZONE").unwrap_or_else(|_| DEFAULT_DNS_ZONE.into());
    let mut candidates = Vec::new();

    // 1. Breakglass env
    if let Ok(ep) = std::env::var("CYNIC_CHAT_ENDPOINT") {
        let ep = ep.trim().trim_end_matches('/').to_string();
        if !ep.is_empty() {
            let model = std::env::var("CYNIC_CHAT_MODEL").ok().filter(|s| !s.is_empty());
            return ChatEndpointResolution {
                schema: "cynic.chat.endpoint.v1",
                ready: true,
                source: ChatEndpointSource::Env,
                endpoint: Some(ep),
                model,
                backend_name: None,
                detail: "breakglass CYNIC_CHAT_ENDPOINT".into(),
                candidates,
                dns_zone,
                config_path: None,
            };
        }
    }

    // 2. CYNIC inference.toml (authority plane)
    if let Some((loaded, path)) = load_chat_inference_config() {
        for b in &loaded.backends {
            candidates.push(format!("config:{} -> {}", b.name, b.base_url));
        }
        // Prefer first sovereign backend; else first backend.
        let backend = loaded
            .backends
            .iter()
            .find(|b| b.sovereign)
            .or_else(|| loaded.backends.first());
        if let Some(b) = backend {
            let ep = b.base_url.trim_end_matches('/').to_string();
            let model = if b.model.is_empty() {
                None
            } else {
                Some(b.model.clone())
            };
            return ChatEndpointResolution {
                schema: "cynic.chat.endpoint.v1",
                ready: true,
                source: ChatEndpointSource::InferenceConfig,
                endpoint: Some(ep),
                model,
                backend_name: Some(b.name.clone()),
                detail: format!("cynic inference SSOT ({})", path.display()),
                candidates,
                dns_zone,
                config_path: Some(path.display().to_string()),
            };
        }
    }

    // 3. Synapse DNS network
    let dns_names = dns_inference_hostnames(&dns_zone);
    for host in &dns_names {
        candidates.push(format!("dns:{host}"));
        if let Some(ip) = resolve_host(host) {
            candidates.push(format!("dns:{host} -> {ip}"));
            // Prefer :8080 (native llama) then :11434 (ODS docker host mapping)
            for port in [8080u16, 11434] {
                let base = format!("http://{host}:{port}");
                if let Some((ep, model)) = probe_openai_base(&base).await {
                    return ChatEndpointResolution {
                        schema: "cynic.chat.endpoint.v1",
                        ready: true,
                        source: ChatEndpointSource::Dns,
                        endpoint: Some(ep),
                        model,
                        backend_name: Some(host.clone()),
                        detail: format!("synapse DNS {host}:{port} (zone={dns_zone})"),
                        candidates,
                        dns_zone,
                        config_path: None,
                    };
                }
                // Also try IP form if hostname resolution works but SNI-less local
                let base_ip = format!("http://{ip}:{port}");
                if let Some((ep, model)) = probe_openai_base(&base_ip).await {
                    return ChatEndpointResolution {
                        schema: "cynic.chat.endpoint.v1",
                        ready: true,
                        source: ChatEndpointSource::Dns,
                        endpoint: Some(ep),
                        model,
                        backend_name: Some(host.clone()),
                        detail: format!("synapse DNS {host} via {ip}:{port}"),
                        candidates,
                        dns_zone,
                        config_path: None,
                    };
                }
            }
        }
    }

    // 4. ODS discovery (elevated if prefer_ods)
    let source_pref = std::env::var("CYNIC_CHAT_SOURCE")
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    let want_ods = prefer_ods || source_pref == "ods";

    if want_ods || source_pref.is_empty() {
        let services = discover_ods_services().await;
        for s in &services {
            candidates.push(format!("ods:{} ({})", s.endpoint, s.name));
        }
        if let Some(s) = services.into_iter().find(|s| s.status == "running") {
            let model = s.models.first().cloned().filter(|m| !m.is_empty());
            return ChatEndpointResolution {
                schema: "cynic.chat.endpoint.v1",
                ready: true,
                source: if want_ods {
                    ChatEndpointSource::Ods
                } else {
                    ChatEndpointSource::NativeProbe
                },
                endpoint: Some(s.endpoint),
                model,
                backend_name: Some(s.name),
                detail: if want_ods {
                    format!("ODS adapter ({ODS_SURFACE} / {ODS_UPSTREAM})")
                } else {
                    "OpenAI-compat probe (ODS-compatible ports)".into()
                },
                candidates,
                dns_zone,
                config_path: None,
            };
        }
    }

    // 5. Last resort loopback bases
    for base in default_ods_probe_bases() {
        candidates.push(base.clone());
        if let Some((ep, model)) = probe_openai_base(&base).await {
            return ChatEndpointResolution {
                schema: "cynic.chat.endpoint.v1",
                ready: true,
                source: ChatEndpointSource::NativeProbe,
                endpoint: Some(ep),
                model,
                backend_name: None,
                detail: "loopback OpenAI-compat probe".into(),
                candidates,
                dns_zone,
                config_path: None,
            };
        }
    }

    ChatEndpointResolution {
        schema: "cynic.chat.endpoint.v1",
        ready: false,
        source: ChatEndpointSource::None,
        endpoint: None,
        model: None,
        backend_name: None,
        detail: format!(
            "no chat endpoint from cynic inference.toml, DNS zone `{dns_zone}`, or ODS/probe — start inference or pass --mock"
        ),
        candidates,
        dns_zone,
        config_path: None,
    }
}

/// Hostnames published (or expected) on the CYNIC synapse DNS plane.
pub fn dns_inference_hostnames(zone: &str) -> Vec<String> {
    vec![
        format!("inference.{zone}"),
        format!("llama.{zone}"),
        format!("llama-server.{zone}"),
        format!("ods.{zone}"),
        format!("chat.{zone}"),
    ]
}

fn load_chat_inference_config() -> Option<(LoadedInferenceConfig, std::path::PathBuf)> {
    for path in inference_config_candidates() {
        match load_inference_config_from_path_public(&path) {
            Ok(loaded) if !loaded.backends.is_empty() => return Some((loaded, path)),
            Ok(_) => continue,
            Err(InferenceConfigError::NotFound { .. }) => continue,
            Err(_) => continue,
        }
    }
    None
}

/// Paths tried for chat endpoint SSOT (not operator-home-first).
pub fn inference_config_candidates() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(p) = std::env::var("CYNIC_INFERENCE_CONFIG") {
        out.push(std::path::PathBuf::from(p));
    }
    out.push(std::path::PathBuf::from("/etc/cynic/inference.toml"));
    let principal = PrincipalPaths::resolve();
    out.push(principal.config_dir.join("inference.toml"));
    out
}

fn resolve_host(name: &str) -> Option<String> {
    let out = Command::new("getent").args(["hosts", name]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&out.stdout);
    let ip = line.split_whitespace().next()?;
    if ip.is_empty() {
        None
    } else {
        Some(ip.to_string())
    }
}

async fn probe_openai_base(base: &str) -> Option<(String, Option<String>)> {
    let base = base.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(900))
        .build()
        .ok()?;

    // Accept either base already ends with /v1 or plain host root.
    let models_urls = if base.ends_with("/v1") {
        vec![format!("{base}/models")]
    } else {
        vec![
            format!("{base}/v1/models"),
            format!("{base}/models"),
        ]
    };

    for url in models_urls {
        let resp = client.get(&url).send().await.ok()?;
        if !resp.status().is_success() {
            continue;
        }
        let body = resp.json::<serde_json::Value>().await.ok()?;
        let model = body
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|m| m.get("id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string());
        let endpoint = if url.contains("/v1/models") {
            url.trim_end_matches("/models").to_string()
        } else if url.ends_with("/models") && !url.contains("/v1/") {
            // non-v1 models path — still expose /v1 for chat
            format!("{base}/v1")
        } else {
            format!("{base}/v1")
        };
        return Some((endpoint, model));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dns_hostnames_use_zone() {
        let h = dns_inference_hostnames("synapse.cynic");
        assert!(h.iter().any(|x| x == "inference.synapse.cynic"));
        assert!(h.iter().any(|x| x == "llama.synapse.cynic"));
    }

    #[test]
    fn inference_candidates_include_etc_and_principal() {
        let c = inference_config_candidates();
        assert!(c.iter().any(|p| p.ends_with("inference.toml")));
        assert!(c.iter().any(|p| p.to_string_lossy().contains("/etc/cynic")));
    }

    #[test]
    fn resolution_schema_stable() {
        let r = ChatEndpointResolution {
            schema: "cynic.chat.endpoint.v1",
            ready: false,
            source: ChatEndpointSource::None,
            endpoint: None,
            model: None,
            backend_name: None,
            detail: "x".into(),
            candidates: vec![],
            dns_zone: DEFAULT_DNS_ZONE.into(),
            config_path: None,
        };
        assert_eq!(r.schema, "cynic.chat.endpoint.v1");
    }
}
