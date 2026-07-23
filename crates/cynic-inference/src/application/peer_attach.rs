//! Frictionless **peer attach** plane for OpenAI-compat environments.
//!
//! ## Coupling policy (constitution)
//!
//! - **No strict product lock**: peers may be Osmantic ODS, native llama-server,
//!   Ollama, or any compatible `/v1` listener.
//! - **ODS** is the recommended *external* bootstrap product when present
//!   (`https://github.com/Osmantic/ODS`) — install stays in host ecosystem
//!   (`cynic deps upgrade --only ods`), never in this plane.
//! - **This module** only: probe → optional publish into CYNIC routing SSOT.
//!
//! ## Modes
//!
//! | mode | mutates | use |
//! |------|---------|-----|
//! | `probe` | no | status / readiness (default CLI) |
//! | `apply` | yes | merge peers into `inference.toml` |
//! | `apply` + open_webui | yes | also try Open WebUI publisher |

use serde::{Deserialize, Serialize};

use crate::adapters::ods::{ODS_DISCOVERY_SCHEMA, ODS_SURFACE, ODS_UPSTREAM};
use crate::adapters::publishers::inference_toml::InferenceTomlPublisher;
use crate::adapters::publishers::open_webui::OpenWebUiPublisher;
use crate::application::model_sync_pipeline::ModelSyncPipeline;
use crate::domain::model_layout::default_ods_probe_bases;
use crate::ods_discovery::{
    DiscoveredService, DiscoveryReport, discover_ods_services, sync_discovered_backends,
};
use crate::ports::model_publisher_port::PublishResult;

/// Options for a single attach invocation.
#[derive(Debug, Clone)]
pub struct PeerAttachOptions {
    /// Write discovered peers into routing SSOT (`inference.toml`).
    pub apply: bool,
    /// When applying, also attempt Open WebUI connection publish (opt-in).
    pub open_webui: bool,
    /// Node id for multi-publisher pipeline (default `local-node`).
    pub node_id: String,
}

impl Default for PeerAttachOptions {
    fn default() -> Self {
        Self {
            apply: false,
            open_webui: false,
            node_id: "local-node".into(),
        }
    }
}

/// Stable machine-readable attach / discovery report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAttachReport {
    pub schema: &'static str,
    pub surface: &'static str,
    /// Upstream product hint only — not a hard dependency.
    pub upstream_hint: &'static str,
    /// `ok` | `empty` | `degraded`
    pub status: String,
    /// `probe` | `apply`
    pub mode: String,
    pub services: Vec<DiscoveredService>,
    pub probe_bases: Vec<String>,
    pub backends_created: usize,
    pub config_updated: bool,
    pub config_path: Option<String>,
    pub publish_results: Vec<PublishResult>,
    /// At least one running peer with a model id.
    pub ready_for_chat: bool,
    pub preferred_endpoint: Option<String>,
    pub preferred_model: Option<String>,
    pub next_steps: Vec<String>,
    pub notes: Vec<String>,
}

/// Probe local peers (and optionally publish) under a single stable schema.
pub async fn peer_attach(opts: PeerAttachOptions) -> PeerAttachReport {
    let probe_bases = default_ods_probe_bases();
    let services = discover_ods_services().await;

    let mut notes = vec![
        "loose coupling: OpenAI-compat peers only (ODS optional bootstrap product)".into(),
        format!("upstream_hint={ODS_UPSTREAM}"),
    ];

    let preferred = pick_preferred(&services);
    let ready_for_chat = preferred.is_some();

    let mut backends_created = 0usize;
    let mut config_updated = false;
    let mut config_path = None;
    let mut publish_results = Vec::new();
    let mut degraded = false;

    if opts.apply {
        // Primary SSOT path: merge into inference.toml (no Open WebUI side effects).
        match sync_discovered_backends().await {
            Ok(DiscoveryReport {
                services: _,
                backends_created: n,
                config_updated: u,
                config_path: path,
            }) => {
                backends_created = n;
                config_updated = u;
                config_path = Some(path.display().to_string());
            }
            Err(e) => {
                degraded = true;
                notes.push(format!("ssot_publish_error={e}"));
            }
        }

        if opts.open_webui {
            let pipeline = ModelSyncPipeline::new(opts.node_id.clone())
                .with_publisher(std::sync::Arc::new(InferenceTomlPublisher))
                .with_publisher(std::sync::Arc::new(OpenWebUiPublisher::new()));
            let results = pipeline.run_sync().await;
            if results.iter().any(|r| r.error.is_some()) {
                degraded = true;
            }
            publish_results = results;
            notes.push("open_webui publisher opt-in enabled".into());
        } else {
            notes.push("open_webui publisher skipped (pass open_webui=true to enable)".into());
        }
    }

    let mode = if opts.apply { "apply" } else { "probe" };
    let status = if degraded {
        "degraded".into()
    } else if services.is_empty() {
        "empty".into()
    } else {
        "ok".into()
    };

    let next_steps = next_steps_for(&services, opts.apply, config_updated, ready_for_chat);

    PeerAttachReport {
        schema: ODS_DISCOVERY_SCHEMA,
        surface: ODS_SURFACE,
        upstream_hint: ODS_UPSTREAM,
        status,
        mode: mode.into(),
        services,
        probe_bases,
        backends_created,
        config_updated,
        config_path,
        publish_results,
        ready_for_chat,
        preferred_endpoint: preferred.as_ref().map(|(e, _)| e.clone()),
        preferred_model: preferred.map(|(_, m)| m),
        next_steps,
        notes,
    }
}

/// Probe-only convenience (no mutation).
pub async fn peer_attach_status() -> PeerAttachReport {
    peer_attach(PeerAttachOptions::default()).await
}

/// Apply convenience: publish into SSOT (toml only).
pub async fn peer_attach_apply() -> PeerAttachReport {
    peer_attach(PeerAttachOptions {
        apply: true,
        open_webui: false,
        node_id: "local-node".into(),
    })
    .await
}

fn pick_preferred(services: &[DiscoveredService]) -> Option<(String, String)> {
    // Prefer peers that advertise at least one model (truthful chat id).
    for s in services.iter().filter(|s| s.status == "running") {
        if let Some(m) = s.models.iter().find(|m| !m.is_empty()) {
            return Some((s.endpoint.clone(), m.clone()));
        }
    }
    // Fallback: first running endpoint (client may still work with server default).
    services
        .iter()
        .find(|s| s.status == "running" && !s.endpoint.is_empty())
        .map(|s| (s.endpoint.clone(), "default".into()))
}

fn next_steps_for(
    services: &[DiscoveredService],
    applied: bool,
    config_updated: bool,
    ready: bool,
) -> Vec<String> {
    let mut steps = Vec::new();
    if services.is_empty() {
        steps.push(
            "no peer listening — start native: cynic runtime inference-start auto --json".into(),
        );
        steps.push(
            "or bootstrap ODS product (optional): cynic deps upgrade --yes --only ods".into(),
        );
        steps.push("then re-probe: cynic runtime inference-discover --json".into());
        steps.push(
            "override bases: CYNIC_ODS_PROBE_URLS=http://127.0.0.1:11434,http://127.0.0.1:8080/v1"
                .into(),
        );
        return steps;
    }
    if !applied {
        steps.push("probe-only (no mutation). attach SSOT: cynic runtime inference-discover --json --apply".into());
    } else if config_updated {
        steps.push("SSOT updated — cynic runtime inference --json".into());
    } else {
        steps.push("SSOT already had these peers".into());
    }
    if ready {
        steps.push("chat via peer preference: cynic chat --ods".into());
        steps.push("or one-shot: cynic chat --ods \"hello\" --json".into());
    }
    steps
        .push("native vault remains independent: cynic runtime inference-start auto --json".into());
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferred_picks_first_running_with_model() {
        let services = vec![
            DiscoveredService {
                name: "a".into(),
                endpoint: "http://127.0.0.1:9/v1".into(),
                status: "down".into(),
                models: vec!["x".into()],
            },
            DiscoveredService {
                name: "b".into(),
                endpoint: "http://127.0.0.1:8080/v1".into(),
                status: "running".into(),
                models: vec!["m1".into(), "m2".into()],
            },
        ];
        let p = pick_preferred(&services).unwrap();
        assert_eq!(p.0, "http://127.0.0.1:8080/v1");
        assert_eq!(p.1, "m1");
    }

    #[test]
    fn empty_next_steps_mention_optional_ods() {
        let steps = next_steps_for(&[], false, false, false);
        assert!(steps.iter().any(|s| s.contains("inference-start")));
        assert!(steps.iter().any(|s| s.contains("only ods")));
    }
}
