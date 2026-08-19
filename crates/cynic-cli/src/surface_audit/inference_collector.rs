//! Inference status proof collector for surface-audit.
//!
//! Read-only observation of the local inference runtime via
//! LocalInferenceRuntimePort. Produces an `InferenceStatusProof`
//! (schema: `cynic.inference.status.v1`) that feeds the
//! surface-audit `SurfaceAuditCheck` entry for the inference surface.
//!
//! Observation is a port call; no hardcoded substrate.

#![allow(dead_code)]
use cynic_inference::domain::local_runtime::LocalRuntimeStatus;
use cynic_inference::LocalRuntimeServiceAny;
use cynic_protocol::runtime::InferenceStatusProof;

use crate::surface_audit::types::{SurfaceAuditCheck, SurfaceAuditEntry};

/// Collect an `InferenceStatusProof` from the live local runtime port.
///
/// Returns `None` when the inference surface is not wired (no
/// LocalRuntimeServiceAny variant available); the surface-audit
/// check then reports `unknown` coverage rather than a spurious
/// `satisfied`.
pub(crate) fn collect_inference_status_proof() -> Option<InferenceStatusProof> {
    let runtime = LocalRuntimeServiceAny::from_env();

    // Read-back through the port — no shell, no hardcoded paths.
    let status: LocalRuntimeStatus = runtime.status().ok()?;

    let observed_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u128;

    Some(InferenceStatusProof {
        schema: InferenceStatusProof::SCHEMA,
        surface: status.surface.to_string(),
        alias: status.alias,
        state: status.state.as_str().to_string(),
        endpoint: status.endpoint,
        listen_host: status.listen_host,
        listen_port: status.listen_port,
        main_pid: status.main_pid,
        memory_current_bytes: status.memory_current_bytes,
        message: status
            .message
            .unwrap_or_else(|| "no llama-server process observed".to_string()),
        observed_at_unix_ms: observed_at,
    })
}

/// Project the inference-status proof into a SurfaceAuditCheck so the
/// surface-audit JSON surface exposes live inference coverage + state.
pub(crate) fn inference_status_check() -> SurfaceAuditCheck {
    match collect_inference_status_proof() {
        Some(proof) if proof.satisfied() => SurfaceAuditCheck {
            name: "inference_runtime_status",
            status: "satisfied",
            detail: format!(
                "local inference runtime active on {} (pid={:?})",
                proof.surface, proof.main_pid
            ),
        },
        Some(proof) => SurfaceAuditCheck {
            name: "inference_runtime_status",
            status: "blocked",
            detail: format!(
                "inference runtime state={} (surface={}, message={})",
                proof.state, proof.surface, proof.message
            ),
        },
        None => SurfaceAuditCheck {
            name: "inference_runtime_status",
            status: "unknown",
            detail: "inference runtime adapter unavailable (coverage null)".to_string(),
        },
    }
}

/// Attach the inference status proof as a runtime proof within a
/// surface audit entry. Caller is responsible for emitting it into
/// the RuntimeProofBundle.
pub(crate) fn inference_status_audit_entry(proof: InferenceStatusProof) -> SurfaceAuditEntry {
    SurfaceAuditEntry {
        id: "cynic.runtime.inference_status".to_string(),
        status: if proof.satisfied() { "satisfied" } else { "blocked" }.into(),
        owner_crate: "cynic-inference".into(),
        domain_owner: "inference".into(),
        adapter_module: "runtime/inference_collector".into(),
        adapter_path: "crates/cynic-cli/src/surface_audit/inference_collector.rs".into(),
        owner_module_paths: vec![
            "cynic_inference::application::local_runtime_service".into(),
            "cynic_inference::ports::local_runtime_port".into(),
            "cynic_protocol::runtime::InferenceStatusProof".into(),
        ],
        mutation_policy: "read_only_observation".into(),
        mutation_dimensions: cynic_protocol::SurfaceMutationDimensions {
            // Mapped from the legacy `mutates_host/mutates_state/mutates_network=false`
            // triple onto the canonical surface_registry.rs:46 shape.
            // "read_only_observation" policy ⇒ read_only_default=true + all admits off.
            read_only_default: true,
            requires_explicit_apply: false,
            allows_admitted_dispatch: false,
            allows_artifact_generation: false,
            allows_temporary_files: false,
            allows_process_execution: false,
            create_once: false,
            human_admin_stdin: false,
            requires_compensation_proof: false,
            requires_signature_digest_epoch_node_slot_preflight: false,
        },
        checks: vec![SurfaceAuditCheck {
            name: "inference_status_read_back",
            status: if proof.satisfied() { "satisfied" } else { "blocked" }.into(),
            detail: format!(
                "observed_at={} surface={} state={} alias={:?}",
                proof.observed_at_unix_ms,
                proof.surface,
                proof.state,
                proof.alias
            ),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_schema_constant_matches() {
        assert_eq!(InferenceStatusProof::SCHEMA, "cynic.inference.status.v1");
    }

    #[test]
    fn satisfied_is_active_state() {
        let proof = InferenceStatusProof {
            schema: InferenceStatusProof::SCHEMA,
            surface: "process".into(),
            alias: Some("qwen-2b".into()),
            state: "active".into(),
            endpoint: Some("http://127.0.0.1:8080".into()),
            listen_host: Some("127.0.0.1".into()),
            listen_port: Some(8080),
            main_pid: Some(4242),
            memory_current_bytes: Some(2048),
            message: "running".into(),
            observed_at_unix_ms: 1,
        };
        assert!(proof.satisfied());
    }

    #[test]
    fn inactive_state_not_satisfied() {
        let proof = InferenceStatusProof {
            schema: InferenceStatusProof::SCHEMA,
            surface: "process".into(),
            alias: None,
            state: "inactive".into(),
            endpoint: None,
            listen_host: None,
            listen_port: None,
            main_pid: None,
            memory_current_bytes: None,
            message: "no process".into(),
            observed_at_unix_ms: 1,
        };
        assert!(!proof.satisfied());
    }
}
