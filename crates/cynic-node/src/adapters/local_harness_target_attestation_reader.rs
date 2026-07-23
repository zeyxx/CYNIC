use std::path::Path;

use cynic_host::runtime_harness::RuntimeHarnessTargetAttestation;
use serde::Deserialize;

use crate::ports::harness_target_attestation_reader_port::HarnessTargetAttestationReaderPort;

pub(crate) struct LocalHarnessTargetAttestationReader;

#[derive(Deserialize)]
struct AttestationEnvelope {
    attestation: RuntimeHarnessTargetAttestation,
}

impl HarnessTargetAttestationReaderPort for LocalHarnessTargetAttestationReader {
    fn read_attestation(&self, path: &Path) -> Result<RuntimeHarnessTargetAttestation, String> {
        let bytes = std::fs::read(path).map_err(|error| {
            format!(
                "failed to read harness target attestation {}: {error}",
                path.display()
            )
        })?;
        if let Ok(attestation) = serde_json::from_slice(&bytes) {
            return Ok(attestation);
        }
        serde_json::from_slice::<AttestationEnvelope>(&bytes)
            .map(|envelope| envelope.attestation)
            .map_err(|error| {
                format!(
                    "invalid harness target attestation JSON {}: {error}",
                    path.display()
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use cynic_host::runtime_harness::{
        RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA, RuntimeHarnessKind,
        RuntimeHarnessTargetAttestation, closed_local_lab_context,
    };

    use super::LocalHarnessTargetAttestationReader;
    use crate::ports::harness_target_attestation_reader_port::HarnessTargetAttestationReaderPort;

    fn attestation() -> RuntimeHarnessTargetAttestation {
        RuntimeHarnessTargetAttestation {
            schema: RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA.to_string(),
            harness_kind: RuntimeHarnessKind::ClosedLocalLab,
            observed_at_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            target_id: "lab-kernel".to_string(),
            provider: "qemu-libvirt".to_string(),
            transport: "qemu-guest-agent".to_string(),
            transport_target: "lab-kernel".to_string(),
            declared_isolation_boundary: closed_local_lab_context().isolation_boundary.to_string(),
            observed_target_boundary: "libvirt-network:cynic-lab-isolated".to_string(),
            target_identity_verified: true,
            transport_verified: true,
            isolated_network_verified: true,
            operator_host_network_unchanged: true,
            proof_refs: vec!["sha256:test:proof".to_string()],
        }
    }

    #[test]
    fn reads_direct_attestation_json() {
        let temp = tempfile::NamedTempFile::new().expect("tempfile");
        std::fs::write(
            temp.path(),
            serde_json::to_vec(&attestation()).expect("serialize"),
        )
        .expect("write");

        let loaded = LocalHarnessTargetAttestationReader
            .read_attestation(temp.path())
            .expect("attestation");

        assert_eq!(loaded.target_id, "lab-kernel");
    }

    #[test]
    fn reads_attestation_report_envelope() {
        let temp = tempfile::NamedTempFile::new().expect("tempfile");
        let envelope = serde_json::json!({
            "schema": "cynic.network.harness_target_attestation.v1",
            "status": "attested",
            "attestation": attestation(),
        });
        std::fs::write(
            temp.path(),
            serde_json::to_vec(&envelope).expect("serialize"),
        )
        .expect("write");

        let loaded = LocalHarnessTargetAttestationReader
            .read_attestation(temp.path())
            .expect("attestation");

        assert_eq!(loaded.transport, "qemu-guest-agent");
    }
}
