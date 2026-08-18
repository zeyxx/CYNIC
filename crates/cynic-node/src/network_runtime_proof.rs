use cynic_protocol::{
    RUNTIME_PROOF_BUNDLE_SCHEMA, RUNTIME_PROOF_SCHEMA, RuntimeProof, RuntimeProofBundle,
    RuntimeProofEvidence,
};
use serde_json::Value;
use std::path::Path;

use crate::network_runtime_status;
use crate::ports::runtime_proof_fetcher_port::RuntimeProofFetcherPort;

/// 2N+2b transport layer — delegate peer `RuntimeProofBundle` fetch to the
/// injected `RuntimeProofFetcherPort` adapter (AGENS.md §7: adapters
/// disposable, port durable). HTTP GET /websocket swap lives only in the
/// adapter impl, never in this policy layer.
pub fn fetch_peer_runtime_proof_bundle(
    fetcher: &dyn RuntimeProofFetcherPort,
    peer_url: &str,
) -> Result<RuntimeProofBundle, Box<dyn std::error::Error>> {
    fetcher.fetch_peer_proof_bundle(peer_url)
}

/// 2N+2b verification policy — the peer node MUST admit `runtime_publisher`
/// (2N+1) before federation trust. Reads the peer's `RuntimeProofBundle`
/// through the injected port and checks the `runtime_publisher_admission`
/// gate is `admitted`. Returns `true` iff admitted, `false` if denied or
/// proof absent (residual unknown → Deny federation, never silent allow).
pub fn verify_peer_runtime_publisher_admission(
    fetcher: &dyn RuntimeProofFetcherPort,
    peer_url: &str,
) -> bool {
    match fetch_peer_runtime_proof_bundle(fetcher, peer_url) {
        Ok(bundle) => bundle.proofs.iter().any(|p| {
            p.gate == "runtime_publisher_admission"
                && p.status == "admitted"
                && p.evidence
                    .iter()
                    .any(|e| e.id == "runtime_publisher_granted" && e.status == "true")
        }),
        Err(_) => false,
    }
}

pub(crate) fn build_network_runtime_proof_bundle(
    registry: &str,
    resolve_dns: bool,
    check_provider_connectivity: bool,
    provider_hosts_path: &str,
    hosts_path: &str,
    corefile_path: &str,
    service_path: &str,
    resolved_dropin_path: &str,
) -> Result<RuntimeProofBundle, Box<dyn std::error::Error>> {
    let report = network_runtime_status::build_network_runtime_status(
        registry,
        resolve_dns,
        check_provider_connectivity,
        provider_hosts_path,
        hosts_path,
        corefile_path,
        service_path,
        resolved_dropin_path,
    )?;
    let report = serde_json::to_value(report)?;
    let proof = RuntimeProof {
        schema: RUNTIME_PROOF_SCHEMA.to_string(),
        gate: "network_runtime_observed".to_string(),
        status: if network_runtime_satisfied(&report) {
            "satisfied"
        } else {
            "blocked"
        }
        .to_string(),
        producer: "cynic-node".to_string(),
        subject: report_string(&report, "hostname").unwrap_or_else(|| "current_node".to_string()),
        observed_at_unix_ms: observed_at_from_generated_at(&report),
        evidence: network_runtime_evidence(&report),
    };
    Ok(RuntimeProofBundle {
        schema: RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
        status: proof.status.clone(),
        proofs: vec![proof]
            .into_iter()
            .chain(runtime_publisher_proof(&resolve_runtime_proof_path(
                hosts_path,
            )))
            .collect(),
    })
}

/// 2N+2 federation: derive the node-local 2N+1 admission proof path from the
/// resolved hosts path's parent (infra/). Returns `infra/runtime_proof.v1`.
/// Path is NOT hardcoded — resolved from the injected `hosts_path` substrate
/// (AGENTS.md: "Dynamic detection before specialization").
fn resolve_runtime_proof_path(hosts_path: &str) -> std::path::PathBuf {
    let p = Path::new(hosts_path);
    p.parent()
        .unwrap_or_else(|| Path::new("."))
        .join("runtime_proof.v1")
}

/// 2N+2 layer 1: self-attest the node-local 2N+1 `runtime_publisher` admission.
/// Reads `infra/runtime_proof.v1` (kernel/node-signed) and emits a 2nd proof in
/// the bundle so federation peers can verify `runtime_publisher.granted == true`
/// without HumanAdmin. Returns `None` when no proof file exists →
/// `residual_unknowns` (never silent guess).
fn runtime_publisher_proof(proof_path: &std::path::Path) -> Option<RuntimeProof> {
    let content = std::fs::read_to_string(proof_path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let granted = v.get("granted").and_then(|g| g.as_bool())?;

    Some(RuntimeProof {
        schema: RUNTIME_PROOF_SCHEMA.to_string(),
        gate: "runtime_publisher_admission".to_string(),
        status: if granted {
            "admitted"
        } else {
            "denied_by_2N_seed"
        }
        .to_string(),
        producer: "cynic-node".to_string(),
        subject: v
            .get("roles")
            .and_then(|r| r.get("runtime_publisher"))
            .and_then(|rp| rp.get("authority_class"))
            .and_then(|a| a.as_str())
            .unwrap_or("runtime_publisher")
            .to_string(),
        observed_at_unix_ms: observed_at_from_generated_at(&v).max(1),
        evidence: vec![RuntimeProofEvidence {
            id: "runtime_publisher_granted".to_string(),
            status: if granted { "true" } else { "false" }.to_string(),
            detail: format!(
                "granted={} via={}",
                granted,
                v.get("roles")
                    .and_then(|r| r.get("runtime_publisher"))
                    .and_then(|rp| rp.get("granted_via"))
                    .and_then(|g| g.as_str())
                    .unwrap_or("unknown")
            ),
        }],
    })
}

fn network_runtime_satisfied(report: &Value) -> bool {
    matches!(
        report.get("status").and_then(Value::as_str),
        Some("mesh_active" | "provider_fallback_active")
    ) && !matches!(
        report.get("severity").and_then(Value::as_str),
        Some("critical")
    )
}

fn network_runtime_evidence(report: &Value) -> Vec<RuntimeProofEvidence> {
    let blockers = report
        .get("blockers")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    vec![
        RuntimeProofEvidence {
            id: "network_status".to_string(),
            status: report_string(report, "status").unwrap_or_else(|| "unknown".to_string()),
            detail: format!(
                "severity={} blockers={blockers}",
                report_string(report, "severity").unwrap_or_else(|| "unknown".to_string())
            ),
        },
        RuntimeProofEvidence {
            id: "actuator_activation".to_string(),
            status: report
                .pointer("/actuator_activation/status")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            detail: format!(
                "bundle_complete={} root_authority={}",
                report
                    .pointer("/actuator_activation/bundle_complete")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                report
                    .pointer("/actuator_activation/root_authority/sudo_noninteractive_available")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            ),
        },
    ]
}

fn report_string(report: &Value, key: &str) -> Option<String> {
    report.get(key).and_then(Value::as_str).map(str::to_string)
}

fn observed_at_from_generated_at(report: &Value) -> u128 {
    report
        .get("generated_at")
        .and_then(Value::as_str)
        .map(stable_timestamp_hint)
        .unwrap_or(0)
}

fn stable_timestamp_hint(value: &str) -> u128 {
    value.bytes().map(u128::from).sum()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{network_runtime_evidence, network_runtime_satisfied};

    #[test]
    fn network_runtime_gate_requires_non_critical_live_status() {
        assert!(network_runtime_satisfied(&json!({
            "status": "mesh_active",
            "severity": "clear"
        })));
        assert!(network_runtime_satisfied(&json!({
            "status": "provider_fallback_active",
            "severity": "warning"
        })));
        assert!(!network_runtime_satisfied(&json!({
            "status": "blocked",
            "severity": "critical"
        })));
    }

    #[test]
    fn evidence_keeps_status_and_blocker_count() {
        let evidence = network_runtime_evidence(&json!({
            "status": "blocked",
            "severity": "critical",
            "blockers": [{}, {}],
            "actuator_activation": {
                "status": "missing_activation",
                "bundle_complete": false,
                "root_authority": {"sudo_noninteractive_available": false}
            }
        }));
        assert_eq!(evidence[0].id, "network_status");
        assert_eq!(evidence[0].status, "blocked");
        assert!(evidence[0].detail.contains("blockers=2"));
    }

    #[test]
    fn resolve_runtime_proof_path_uses_hosts_parent() {
        let hosts = std::env::temp_dir().join("subnet/hosts.txt");
        std::fs::create_dir_all(hosts.parent().unwrap()).ok();
        let got = super::resolve_runtime_proof_path(hosts.to_str().unwrap());
        assert_eq!(
            got.file_name().and_then(|n| n.to_str()),
            Some("runtime_proof.v1")
        );
        assert_eq!(got.parent().unwrap(), hosts.parent().unwrap());
    }

    #[test]
    fn runtime_publisher_proof_admitted_when_granted() {
        // 2N+2 RED→GREEN: a granted infra/runtime_proof.v1 yields an
        // `admitted` runtime_publisher_admission proof in the bundle.
        let dir = std::env::temp_dir().join(format!("cynic-2N2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok();
        let proof = serde_json::json!({
            "schema": "cynic.runtime_proof.v1",
            "granted": true,
            "roles": { "runtime_publisher": {
                "authority_class": "RuntimeMachineIdentity",
                "granted_via": "kernel/node-signed runtime_proof.v1 (2N+1 admission)"
            } }
        });
        let proof_path = dir.join("runtime_proof.v1");
        std::fs::write(&proof_path, serde_json::to_string(&proof).unwrap()).unwrap();

        let p = super::runtime_publisher_proof(&proof_path)
            .expect("proof file exists with granted=true");
        assert_eq!(p.gate, "runtime_publisher_admission");
        assert_eq!(p.status, "admitted");
        assert_eq!(p.producer, "cynic-node");
        assert!(
            p.evidence
                .iter()
                .any(|e| e.id == "runtime_publisher_granted")
        );
        let ev = p
            .evidence
            .iter()
            .find(|e| e.id == "runtime_publisher_granted")
            .unwrap();
        assert!(ev.detail.starts_with("granted=true via="));
    }

    #[test]
    fn runtime_publisher_proof_none_when_file_absent() {
        // 2N+2 invariant: missing proof => residual unknown (None), never silent guess.
        let missing = std::env::temp_dir().join("cynic-2N2-missing-proof.v1");
        std::fs::remove_file(&missing).ok();
        assert!(super::runtime_publisher_proof(&missing).is_none());
    }
}
