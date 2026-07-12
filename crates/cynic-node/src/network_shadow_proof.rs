use std::error::Error;

use chrono::Utc;
use serde::Serialize;
use serde_json::Value;

use crate::adapters::dns_renderer::SynapseDnsRenderer;
use crate::adapters::registry::LocalRegistryAdapter;
use crate::observe;
use crate::ports::network_interfaces_port::{DnsBundleRendererPort, RegistryTopologyPort};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkShadowProofReport {
    schema: &'static str,
    generated_at: String,
    registry: String,
    legacy_python: RetiredLegacyPython,
    rust: ShadowProofRust,
    drift: Vec<ShadowDrift>,
    pub(crate) status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct RetiredLegacyPython {
    status: &'static str,
    reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct ShadowProofRust {
    dns_bundle: RustDnsBundle,
    mesh_status: RustMeshStatus,
}

#[derive(Debug, Clone, Serialize)]
struct RustDnsBundle {
    activation_plan: Value,
    synapse_hosts: String,
    corefile: String,
    resolved_dropin: String,
}

#[derive(Debug, Clone, Serialize)]
struct RustMeshStatus {
    runtime_status: String,
    dns_status: String,
    blocker_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ShadowDrift {
    kind: &'static str,
    detail: String,
}

#[derive(Debug, Clone)]
struct ShadowProofReportInputs {
    registry: String,
    rust: ShadowProofRust,
    drift: Vec<ShadowDrift>,
}

pub(crate) fn build_shadow_proof(
    registry: &str,
    resolve_dns: bool,
    observe_runtime: bool,
) -> Result<NetworkShadowProofReport, Box<dyn Error>> {
    let registry_adapter = LocalRegistryAdapter::new(registry);
    let topology = registry_adapter.fetch_static_topology()?;
    let zone = registry_adapter
        .fetch_network_policy_summary()?
        .synapse_dns_zone
        .unwrap_or_else(crate::dns_contract::default_dns_zone);
    let renderer = SynapseDnsRenderer::new(zone);
    let activation_plan = renderer.activation_plan()?;
    let synapse_hosts = renderer.render_hosts(&topology)?;
    let corefile = renderer.render_corefile()?;
    let resolved_dropin = renderer.render_resolved_dropin()?;
    let rust_observation = observe::observe_payload(Some(registry));

    let rust = ShadowProofRust {
        dns_bundle: RustDnsBundle {
            activation_plan: serde_json::to_value(activation_plan)?,
            synapse_hosts,
            corefile,
            resolved_dropin,
        },
        mesh_status: RustMeshStatus {
            runtime_status:
                serde_json::to_value(&rust_observation)?["network_mesh_status"]["runtime_status"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
            dns_status:
                serde_json::to_value(&rust_observation)?["network_mesh_status"]["dns_status"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
            blocker_count: rust_observation.blockers.len(),
        },
    };

    let drift = evaluate_rust_network_shadow(&rust, resolve_dns, observe_runtime);
    Ok(build_shadow_proof_report(ShadowProofReportInputs {
        registry: registry.to_string(),
        rust,
        drift,
    }))
}

fn evaluate_rust_network_shadow(
    rust: &ShadowProofRust,
    resolve_dns: bool,
    observe_runtime: bool,
) -> Vec<ShadowDrift> {
    let mut drift = Vec::new();

    if normalize_text(&rust.dns_bundle.synapse_hosts).is_empty() {
        drift.push(ShadowDrift {
            kind: "dns_hosts_empty",
            detail: "Rust DNS host projection is empty for the current registry.".to_string(),
        });
    }
    if normalize_text(&rust.dns_bundle.corefile).is_empty() {
        drift.push(ShadowDrift {
            kind: "dns_corefile_empty",
            detail: "Rust CoreDNS projection is empty.".to_string(),
        });
    }
    if normalize_text(&rust.dns_bundle.resolved_dropin).is_empty() {
        drift.push(ShadowDrift {
            kind: "dns_resolved_dropin_empty",
            detail: "Rust resolved drop-in projection is empty.".to_string(),
        });
    }
    if observe_runtime && rust.mesh_status.runtime_status == "unknown" {
        drift.push(ShadowDrift {
            kind: "runtime_status_unknown",
            detail: "Runtime observation did not produce a network mesh status.".to_string(),
        });
    }
    if resolve_dns && rust.mesh_status.dns_status == "unknown" {
        drift.push(ShadowDrift {
            kind: "dns_status_unknown",
            detail: "DNS resolution was requested but no DNS status was observed.".to_string(),
        });
    }

    drift
}

fn normalize_text(value: &str) -> String {
    value
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn build_shadow_proof_report(inputs: ShadowProofReportInputs) -> NetworkShadowProofReport {
    let status = if inputs.drift.is_empty() {
        "aligned"
    } else {
        "drift_detected"
    };

    NetworkShadowProofReport {
        schema: "cynic.node_network_shadow_proof.v1",
        generated_at: Utc::now().to_rfc3339(),
        registry: inputs.registry,
        legacy_python: RetiredLegacyPython {
            status: "retired",
            reason: "Python organ shadow proof was removed from the minimal low-layer baseline.",
        },
        rust: inputs.rust,
        drift: inputs.drift,
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_host::registry::DEFAULT_REGISTRY_PATH;
    use serde_json::json;

    #[test]
    fn normalize_text_ignores_trailing_space_and_blank_edges() {
        assert_eq!(normalize_text("a \n b\n\n"), "a\n b");
    }

    #[test]
    fn report_builder_marks_drift_status_from_inputs() {
        let report = build_shadow_proof_report(ShadowProofReportInputs {
            registry: DEFAULT_REGISTRY_PATH.to_string(),
            rust: ShadowProofRust {
                dns_bundle: RustDnsBundle {
                    activation_plan: json!({}),
                    synapse_hosts: String::new(),
                    corefile: String::new(),
                    resolved_dropin: String::new(),
                },
                mesh_status: RustMeshStatus {
                    runtime_status: "mesh_active".to_string(),
                    dns_status: "active".to_string(),
                    blocker_count: 0,
                },
            },
            drift: vec![ShadowDrift {
                kind: "dns_status_unknown",
                detail: "mismatch".to_string(),
            }],
        });

        assert_eq!(report.status, "drift_detected");
        assert_eq!(report.legacy_python.status, "retired");
    }
}
