use cynic_protocol::{
    RUNTIME_PROOF_BUNDLE_SCHEMA, RUNTIME_PROOF_SCHEMA, RuntimeProof, RuntimeProofBundle,
    RuntimeProofEvidence,
};
use serde_json::Value;

use crate::network_runtime_status;

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
        proofs: vec![proof],
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
}
