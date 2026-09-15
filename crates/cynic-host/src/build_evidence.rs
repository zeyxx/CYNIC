use std::fs;

use cynic_protocol::{
    KernelEndpointPolicy, LeaseGuard, LeaseVerification, coord_verify_path, kernel_url,
};
use sha2::{Digest, Sha256};

use crate::update_lifecycle::ArgvRunner;

pub trait BuildRunnerObservationPort {
    fn observe(&self) -> Result<cynic_build::BuildRunnerObservation, String>;
}

pub trait BuildLeaseVerificationPort {
    fn verify(&self, guard: &LeaseGuard) -> LeaseVerification;
}

pub struct JsonBuildRunnerObservationAdapter<'a> {
    path: &'a str,
}

impl<'a> JsonBuildRunnerObservationAdapter<'a> {
    pub fn new(path: &'a str) -> Self {
        Self { path }
    }
}

impl BuildRunnerObservationPort for JsonBuildRunnerObservationAdapter<'_> {
    fn observe(&self) -> Result<cynic_build::BuildRunnerObservation, String> {
        let raw = fs::read_to_string(self.path)
            .map_err(|error| format!("failed to read build runner observation: {error}"))?;
        parse_observation(&raw)
    }
}

pub struct SshBuildRunnerObservationAdapter<'a, R: ArgvRunner> {
    runner: &'a R,
    target: &'a str,
}

impl<'a, R: ArgvRunner> SshBuildRunnerObservationAdapter<'a, R> {
    pub fn new(runner: &'a R, target: &'a str) -> Result<Self, String> {
        if target.trim().is_empty() || target.starts_with('-') {
            return Err("build runner SSH target is invalid".to_string());
        }
        Ok(Self { runner, target })
    }
}

impl<R: ArgvRunner> BuildRunnerObservationPort for SshBuildRunnerObservationAdapter<'_, R> {
    fn observe(&self) -> Result<cynic_build::BuildRunnerObservation, String> {
        let argv = [
            "ssh",
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "ConnectTimeout=10",
            self.target,
            "cynic-node",
            "registry-observation",
            "--json",
            "--registry",
            "/etc/cynic/registry.json",
        ];
        let output = self.runner.run(&argv);
        if !output.success() {
            return Err(output
                .spawn_error
                .unwrap_or_else(|| "build runner SSH observation failed".to_string()));
        }
        parse_observation(&output.stdout)
    }
}

fn parse_observation(raw: &str) -> Result<cynic_build::BuildRunnerObservation, String> {
    let mut observation = serde_json::from_str::<cynic_build::BuildRunnerObservation>(raw)
        .map_err(|error| format!("failed to parse build runner observation: {error}"))?;
    observation.evidence_refs.push(format!(
        "sha256:{}",
        hex::encode(Sha256::digest(raw.as_bytes()))
    ));
    Ok(observation)
}

pub struct CurlBuildLeaseVerificationAdapter<'a, R: ArgvRunner> {
    runner: &'a R,
    verify_url: String,
    bearer_token: Option<String>,
}

impl<'a, R: ArgvRunner> CurlBuildLeaseVerificationAdapter<'a, R> {
    pub fn new(
        runner: &'a R,
        endpoint: impl Into<String>,
        bearer_token: Option<String>,
    ) -> Result<Self, String> {
        let endpoint = endpoint.into();
        let verify_url = kernel_url(
            &endpoint,
            coord_verify_path(),
            KernelEndpointPolicy::default(),
        )
        .map_err(|error| format!("invalid kernel coordination endpoint: {error}"))?;
        Ok(Self {
            runner,
            verify_url,
            bearer_token,
        })
    }
}

impl<R: ArgvRunner> BuildLeaseVerificationPort for CurlBuildLeaseVerificationAdapter<'_, R> {
    fn verify(&self, guard: &LeaseGuard) -> LeaseVerification {
        let body = match serde_json::to_string(guard) {
            Ok(body) => body,
            Err(error) => {
                return LeaseVerification::unavailable(
                    guard.clone(),
                    format!("coordination_lease_guard_serialization_failed:{error}"),
                );
            }
        };
        let mut argv = vec![
            "curl".to_string(),
            "-f".to_string(),
            "-sS".to_string(),
            "-X".to_string(),
            "POST".to_string(),
            "-H".to_string(),
            "Content-Type: application/json".to_string(),
        ];
        if let Some(token) = &self.bearer_token {
            argv.extend(["-H".to_string(), format!("Authorization: Bearer {token}")]);
        }
        argv.extend(["--data-raw".to_string(), body, self.verify_url.clone()]);
        let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.runner.run(&refs);
        if !output.success() {
            return LeaseVerification::unavailable(
                guard.clone(),
                output.spawn_error.unwrap_or_else(|| {
                    "coordination_lease_verification_request_failed".to_string()
                }),
            );
        }
        match serde_json::from_str::<LeaseVerification>(&output.stdout) {
            Ok(verification) if verification.guard == *guard => verification,
            Ok(_) => LeaseVerification::unavailable(
                guard.clone(),
                "coordination_lease_verification_guard_mismatch",
            ),
            Err(_) => LeaseVerification::unavailable(
                guard.clone(),
                "coordination_lease_verification_response_invalid",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use cynic_protocol::LEASE_VERIFICATION_SCHEMA;

    use super::*;
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};

    #[test]
    fn live_verifier_rejects_response_for_another_guard() {
        let guard = LeaseGuard {
            agent_id: "agent-a".to_string(),
            target: "build-runner:builder-1".to_string(),
            lease_id: "lease-1".to_string(),
            fencing_token: 3,
        };
        let other = LeaseVerification::proved(
            LeaseGuard {
                lease_id: "lease-2".to_string(),
                ..guard.clone()
            },
            None,
        );
        let runner = MapArgvRunner {
            responses: vec![(
                vec!["curl".to_string()],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: serde_json::to_string(&other).expect("serialize verification"),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let adapter =
            CurlBuildLeaseVerificationAdapter::new(&runner, "http://kernel.internal:3030", None)
                .expect("non-loopback endpoint");
        let verification = adapter.verify(&guard);
        assert_eq!(verification.schema, LEASE_VERIFICATION_SCHEMA);
        assert_eq!(verification.status, "unavailable");
        assert_eq!(verification.guard, guard);
    }

    #[test]
    fn ssh_observer_requires_strict_noninteractive_transport() {
        let observation = cynic_build::RuntimeNodeObservation {
            schema: cynic_build::BUILD_RUNNER_OBSERVATION_SCHEMA.to_string(),
            status: "observed".to_string(),
            node_id: "builder-1".to_string(),
            hostname: "builder-1".to_string(),
            declared_role: Some("build_runner".to_string()),
            identity_match: cynic_protocol::SecurityEvidenceState::Proven,
            observed_at_unix_ms: 1,
            registry_path: "/etc/cynic/registry.json".to_string(),
            evidence: serde_json::json!({}),
            evidence_refs: vec!["node-proof".to_string()],
        };
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "ssh".to_string(),
                    "-o".to_string(),
                    "BatchMode=yes".to_string(),
                    "-o".to_string(),
                    "StrictHostKeyChecking=yes".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: serde_json::to_string(&observation).expect("serialize observation"),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let adapter = SshBuildRunnerObservationAdapter::new(&runner, "builder.internal")
            .expect("valid target");
        let observed = adapter.observe().expect("SSH observation");
        assert_eq!(observed.node_id, "builder-1");
        assert!(
            observed
                .evidence_refs
                .iter()
                .any(|reference| reference.starts_with("sha256:"))
        );
    }
}
