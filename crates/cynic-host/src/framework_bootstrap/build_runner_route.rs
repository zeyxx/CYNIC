//! Host-side projection of observed Builder evidence into the canonical route decision.
//!
//! This module deliberately does not execute `pct`, SSH, or a bootstrap command.
//! It only composes independent read-backs so a later bounded adapter receives a
//! route selected once by `cynic-protocol`.

use cynic_protocol::{
    BuildRunnerCoordinationRouteObservation, BuildRunnerProviderRouteObservation,
    BuildRunnerRouteDecision, BuildRunnerRouteDesiredState, BuildRunnerRouteObservation,
    BuildRunnerSeedRouteObservation, BuildRunnerSshTrustRouteObservation, NodeNetworkAttachment,
    NodeSubstrateExistence, NodeSubstrateObservation, NodeSubstratePower,
    reconcile_build_runner_route,
};

use crate::ssh_trust::SshTrustVerification;

use super::build_runner::BootstrapBuildProof;

pub fn reconcile_build_runner_route_from_read_backs(
    desired: &BuildRunnerRouteDesiredState,
    provider: &NodeSubstrateObservation,
    ssh_trust: &SshTrustVerification,
    seed: Option<&BootstrapBuildProof>,
    builder_runtime: Option<cynic_protocol::BuildRunnerObservation>,
    coordination: BuildRunnerCoordinationRouteObservation,
    now_unix_ms: u64,
) -> BuildRunnerRouteDecision {
    let provider_status = if provider.status == "observed"
        && provider.existence == NodeSubstrateExistence::Present
        && provider.power == NodeSubstratePower::Running
        && provider.network_attachment == NodeNetworkAttachment::Attached
        && provider.observation_authority.is_some()
    {
        "observed"
    } else {
        "unobserved"
    };
    let provider = BuildRunnerProviderRouteObservation {
        status: provider_status.to_string(),
        node_id: provider.node_id.clone(),
        provider: provider.provider.clone().unwrap_or_default(),
        resource_id: provider.resource_id.clone().unwrap_or_default(),
        observed_at_unix_ms: provider.observed_at_unix_ms.unwrap_or_default(),
        evidence_refs: provider.evidence_refs.clone(),
    };
    let ssh_trust = BuildRunnerSshTrustRouteObservation {
        status: ssh_trust.status.clone(),
        observed_at_unix_ms: ssh_trust.observed_at_unix_ms,
        evidence_refs: (!ssh_trust.observed_fingerprints.is_empty())
            .then(|| format!("ssh-trust:{}:{}", ssh_trust.target, ssh_trust.status))
            .into_iter()
            .collect(),
    };
    let seed = seed.map_or_else(
        || BuildRunnerSeedRouteObservation {
            status: "absent".to_string(),
            node_id: desired.node_id.clone(),
            source_commit: desired.source_commit.clone(),
            observed_at_unix_ms: 0,
            evidence_refs: Vec::new(),
        },
        |proof| BuildRunnerSeedRouteObservation {
            status: if proof.status == "proved"
                && proof.artifact_digest.is_some()
                && proof.blockers.is_empty()
            {
                "proved"
            } else {
                "refuted"
            }
            .to_string(),
            node_id: desired.node_id.clone(),
            source_commit: proof.source_commit.clone(),
            observed_at_unix_ms: proof.proved_at_unix_ms,
            evidence_refs: proof.evidence_refs.clone(),
        },
    );
    reconcile_build_runner_route(
        desired,
        &BuildRunnerRouteObservation {
            provider,
            ssh_trust,
            seed,
            builder_runtime,
            coordination,
        },
        now_unix_ms,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{BuildRunnerRoutePath, BuildRunnerRouteState, NodeSubstrateObservation};

    const NOW: u64 = 1_000_000;

    fn desired() -> BuildRunnerRouteDesiredState {
        BuildRunnerRouteDesiredState {
            node_id: "cynic-builder".to_string(),
            provider: "proxmox".to_string(),
            resource_id: "lxc/105".to_string(),
            role: "build_runner".to_string(),
            source_commit: "a".repeat(40),
        }
    }

    fn provider() -> NodeSubstrateObservation {
        NodeSubstrateObservation {
            schema: "cynic.node_substrate.observation.v1".to_string(),
            status: "observed".to_string(),
            node_id: "cynic-builder".to_string(),
            provider: Some("proxmox".to_string()),
            resource_id: Some("lxc/105".to_string()),
            existence: NodeSubstrateExistence::Present,
            power: NodeSubstratePower::Running,
            network_attachment: NodeNetworkAttachment::Attached,
            observed_addresses: vec!["10.10.10.119".to_string()],
            management_reachable: Some(true),
            observation_authority: Some("proxmox:pvesh".to_string()),
            observed_at_unix_ms: Some(NOW),
            evidence_refs: vec!["proxmox:lxc/105".to_string()],
            unknowns: Vec::new(),
        }
    }

    fn ssh_trust() -> SshTrustVerification {
        SshTrustVerification {
            schema: "cynic.ssh_trust_verification.v1",
            target: "cynic-builder".to_string(),
            status: "not_observed".to_string(),
            anchor: None,
            observed_fingerprints: Vec::new(),
            observed_at_unix_ms: NOW,
            max_age_ms: 300_000,
            expires_at_unix_ms: NOW + 300_000,
            residual_risks: vec!["no anchor".to_string()],
        }
    }

    #[test]
    fn fresh_provider_without_ssh_proof_only_proposes_recovery() {
        let decision = reconcile_build_runner_route_from_read_backs(
            &desired(),
            &provider(),
            &ssh_trust(),
            None,
            None,
            BuildRunnerCoordinationRouteObservation {
                status: "unknown".to_string(),
                observed_at_unix_ms: 0,
                evidence_refs: Vec::new(),
            },
            NOW,
        );

        assert_eq!(decision.state, BuildRunnerRouteState::ProviderObserved);
        assert_eq!(
            decision.next_path,
            Some(BuildRunnerRoutePath::ProxmoxRecovery)
        );
        assert!(!decision.mutates);
    }
}
