use cynic_host::runtime_harness::{
    RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA, RuntimeHarnessContext,
    RuntimeHarnessTargetAttestation,
};
use cynic_lifecycle::ArgvRunner;
use sha2::{Digest, Sha256};

const TARGET_ATTESTATION_TTL_MS: u64 = 5 * 60 * 1_000;
const QGA_ATTESTATION_TIMEOUT_SECONDS: &str = "5";

use crate::ports::harness_target_attestation_port::{
    HarnessTargetAttestationPort, HarnessTargetAttestationRequest,
};

pub(crate) struct LibvirtHarnessTargetAttestor<R> {
    runner: R,
    virsh_bin: String,
    ip_bin: String,
}

impl<R> LibvirtHarnessTargetAttestor<R> {
    pub(crate) fn new(runner: R, virsh_bin: impl Into<String>, ip_bin: impl Into<String>) -> Self {
        Self {
            runner,
            virsh_bin: virsh_bin.into(),
            ip_bin: ip_bin.into(),
        }
    }
}

impl<R: ArgvRunner> HarnessTargetAttestationPort for LibvirtHarnessTargetAttestor<R> {
    fn attest_target(
        &self,
        context: &RuntimeHarnessContext,
        request: &HarnessTargetAttestationRequest,
    ) -> Result<RuntimeHarnessTargetAttestation, String> {
        if request.provider != "qemu-libvirt" {
            return Err(format!(
                "libvirt target attestor cannot observe provider {}",
                request.provider
            ));
        }
        if request.target_id.trim().is_empty()
            || request.expected_network_boundary.trim().is_empty()
        {
            return Err("libvirt target attestation requires target and network identities".into());
        }

        let route_before = self.runner.run(&[&self.ip_bin, "route", "show"]);
        let domain_state = self
            .runner
            .run(&[&self.virsh_bin, "domstate", &request.target_id]);
        let domain_interfaces =
            self.runner
                .run(&[&self.virsh_bin, "domiflist", &request.target_id]);
        let network_xml = self.runner.run(&[
            &self.virsh_bin,
            "net-dumpxml",
            &request.expected_network_boundary,
        ]);
        let transport_observation = observe_transport(
            &self.runner,
            &self.virsh_bin,
            &request.target_id,
            &request.transport,
            &request.transport_target,
        );
        let route_after = self.runner.run(&[&self.ip_bin, "route", "show"]);

        let target_identity_verified = domain_state.success()
            && domain_state.stdout.trim() == "running"
            && domain_interfaces.success();
        let attached_to_expected_network = domain_interfaces
            .stdout
            .split_whitespace()
            .any(|token| token == request.expected_network_boundary);
        let isolated_network_verified = network_xml.success()
            && network_xml.stdout.contains(&format!(
                "<name>{}</name>",
                request.expected_network_boundary
            ))
            && !network_xml.stdout.contains("<forward")
            && attached_to_expected_network;
        let operator_host_network_unchanged = route_before.success()
            && route_after.success()
            && route_before.stdout.trim() == route_after.stdout.trim();
        let transport_verified = transport_observation
            .as_ref()
            .is_some_and(qga_ping_succeeded);

        let observed_at_unix_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
        Ok(RuntimeHarnessTargetAttestation {
            schema: RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA.to_string(),
            harness_kind: context.kind.clone(),
            observed_at_unix_ms,
            expires_at_unix_ms: observed_at_unix_ms.saturating_add(TARGET_ATTESTATION_TTL_MS),
            target_id: request.target_id.clone(),
            provider: request.provider.clone(),
            transport: request.transport.clone(),
            transport_target: request.transport_target.clone(),
            declared_isolation_boundary: context.isolation_boundary.to_string(),
            observed_target_boundary: format!(
                "libvirt-network:{}",
                request.expected_network_boundary
            ),
            target_identity_verified,
            transport_verified,
            isolated_network_verified,
            operator_host_network_unchanged,
            proof_refs: vec![
                proof_ref("operator-route-before", &route_before.stdout),
                proof_ref("libvirt-domain-state", &domain_state.stdout),
                proof_ref("libvirt-domain-interfaces", &domain_interfaces.stdout),
                proof_ref("libvirt-network-xml", &network_xml.stdout),
                proof_ref(
                    "target-transport",
                    transport_observation
                        .as_ref()
                        .map_or("unsupported", |output| output.stdout.as_str()),
                ),
                proof_ref("operator-route-after", &route_after.stdout),
            ],
        })
    }
}

fn observe_transport(
    runner: &impl ArgvRunner,
    virsh_bin: &str,
    target_id: &str,
    transport: &str,
    transport_target: &str,
) -> Option<cynic_lifecycle::ArgvOutput> {
    if transport != "qemu-guest-agent" || transport_target != target_id {
        return None;
    }
    Some(runner.run(&[
        virsh_bin,
        "qemu-agent-command",
        target_id,
        "--timeout",
        QGA_ATTESTATION_TIMEOUT_SECONDS,
        r#"{"execute":"guest-ping"}"#,
    ]))
}

fn qga_ping_succeeded(output: &cynic_lifecycle::ArgvOutput) -> bool {
    output.success()
        && serde_json::from_str::<serde_json::Value>(&output.stdout)
            .ok()
            .and_then(|value| value.get("return").cloned())
            .is_some_and(|value| value.is_object())
}

fn proof_ref(kind: &str, observed: &str) -> String {
    let digest = Sha256::digest(observed.as_bytes());
    format!("sha256:{kind}:{}", hex::encode(digest))
}

#[cfg(test)]
mod tests {
    use cynic_host::runtime_harness::{
        closed_local_lab_context, validate_runtime_harness_target_attestation,
    };
    use cynic_lifecycle::{ArgvOutput, MapArgvRunner};

    use super::LibvirtHarnessTargetAttestor;
    use crate::ports::harness_target_attestation_port::{
        HarnessTargetAttestationPort, HarnessTargetAttestationRequest,
    };

    fn output(stdout: &str) -> ArgvOutput {
        ArgvOutput {
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
            spawn_error: None,
        }
    }

    fn request() -> HarnessTargetAttestationRequest {
        HarnessTargetAttestationRequest {
            target_id: "lab-kernel".to_string(),
            provider: "qemu-libvirt".to_string(),
            transport: "qemu-guest-agent".to_string(),
            transport_target: "lab-kernel".to_string(),
            expected_network_boundary: "cynic-lab-isolated".to_string(),
        }
    }

    fn runner(network_xml: &str) -> MapArgvRunner {
        MapArgvRunner {
            responses: vec![
                (
                    vec!["/portable/ip".into(), "route".into(), "show".into()],
                    output("default via 192.0.2.1 dev eth0\n"),
                ),
                (
                    vec![
                        "/portable/virsh".into(),
                        "domstate".into(),
                        "lab-kernel".into(),
                    ],
                    output("running\n"),
                ),
                (
                    vec![
                        "/portable/virsh".into(),
                        "domiflist".into(),
                        "lab-kernel".into(),
                    ],
                    output("vnet0 network cynic-lab-isolated virtio\n"),
                ),
                (
                    vec![
                        "/portable/virsh".into(),
                        "net-dumpxml".into(),
                        "cynic-lab-isolated".into(),
                    ],
                    output(network_xml),
                ),
                (
                    vec![
                        "/portable/virsh".into(),
                        "qemu-agent-command".into(),
                        "lab-kernel".into(),
                        "--timeout".into(),
                        "5".into(),
                        r#"{"execute":"guest-ping"}"#.into(),
                    ],
                    output(r#"{"return":{}}"#),
                ),
            ],
        }
    }

    #[test]
    fn observes_attested_libvirt_target_without_mutation() {
        let context = closed_local_lab_context();
        let attestor = LibvirtHarnessTargetAttestor::new(
            runner("<network><name>cynic-lab-isolated</name></network>"),
            "/portable/virsh",
            "/portable/ip",
        );

        let attestation = attestor
            .attest_target(&context, &request())
            .expect("attestation");

        assert!(attestation.target_identity_verified);
        assert!(attestation.transport_verified);
        assert!(attestation.isolated_network_verified);
        assert!(attestation.operator_host_network_unchanged);
        assert_eq!(attestation.proof_refs.len(), 6);
        assert!(validate_runtime_harness_target_attestation(&context, &attestation).is_empty());
    }

    #[test]
    fn network_with_forwarding_is_not_attested_as_isolated() {
        let context = closed_local_lab_context();
        let attestor = LibvirtHarnessTargetAttestor::new(
            runner("<network><name>cynic-lab-isolated</name><forward mode='nat'/></network>"),
            "/portable/virsh",
            "/portable/ip",
        );

        let attestation = attestor
            .attest_target(&context, &request())
            .expect("observation report");
        let violations = validate_runtime_harness_target_attestation(&context, &attestation);

        assert!(!attestation.isolated_network_verified);
        assert!(
            violations
                .iter()
                .any(|violation| { violation.code == "harness_target_isolation_unverified" })
        );
    }
}
