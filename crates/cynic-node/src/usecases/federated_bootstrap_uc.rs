use cynic_node_protocol::{FederatedBootstrapBinding, FederatedBootstrapObservation, FederatedBootstrapReadBack, federated_bootstrap_observation};

use crate::ports::federated_bootstrap_read_back_port::FederatedBootstrapReadBackPort;

/// Collect one node's bootstrap read-back without allowing a transport failure
/// or a stale deployment proof to impersonate a federated observation.
pub async fn observe_federated_bootstrap_node<P: FederatedBootstrapReadBackPort>(
    port: &P,
    node_id: &str,
    expected_binding: &FederatedBootstrapBinding,
) -> FederatedBootstrapObservation {
    match port
        .read_federated_bootstrap(node_id, expected_binding)
        .await
    {
        Ok(mut read_back) => {
            if read_back.node_id != node_id {
                read_back
                    .blockers
                    .push("federated_bootstrap_node_identity_mismatch".to_string());
            }
            if read_back.binding != *expected_binding {
                read_back
                    .blockers
                    .push("federated_bootstrap_requested_binding_mismatch".to_string());
            }
            federated_bootstrap_observation(read_back)
        }
        Err(_) => federated_bootstrap_observation(unavailable_read_back(node_id, expected_binding)),
    }
}

fn unavailable_read_back(
    node_id: &str,
    expected_binding: &FederatedBootstrapBinding,
) -> FederatedBootstrapReadBack {
    FederatedBootstrapReadBack {
        node_id: node_id.to_string(),
        epoch_id: expected_binding.deployment_epoch_id.clone(),
        binding: expected_binding.clone(),
        seed_admitted: false,
        bootstrap_controller_ready: false,
        provider_ready: false,
        node_enrolled: false,
        storage_ready: false,
        node_ready: false,
        scheduler_ready: false,
        build_proven: false,
        release_read_back: false,
        blockers: vec!["federated_bootstrap_read_back_unavailable".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedPort(Result<FederatedBootstrapReadBack, String>);

    impl FederatedBootstrapReadBackPort for FixedPort {
        async fn read_federated_bootstrap(
            &self,
            _node_id: &str,
            _expected_binding: &FederatedBootstrapBinding,
        ) -> Result<FederatedBootstrapReadBack, String> {
            self.0.clone()
        }
    }

    fn binding() -> FederatedBootstrapBinding {
        FederatedBootstrapBinding {
            source_commit: "a".repeat(40),
            deployment_epoch_id: "epoch-1".to_string(),
            observed_at_unix_ms: 1,
            evidence_refs: vec!["sha256:read-back".to_string()],
        }
    }

    fn read_back(binding: FederatedBootstrapBinding) -> FederatedBootstrapReadBack {
        FederatedBootstrapReadBack {
            node_id: "node-a".to_string(),
            epoch_id: "epoch-1".to_string(),
            binding,
            seed_admitted: true,
            bootstrap_controller_ready: false,
            provider_ready: false,
            node_enrolled: false,
            storage_ready: false,
            node_ready: false,
            scheduler_ready: false,
            build_proven: false,
            release_read_back: false,
            blockers: Vec::new(),
        }
    }

    #[tokio::test]
    async fn stale_binding_is_blocked_before_a_fleet_consumer_can_use_it() {
        let expected = binding();
        let mut stale = binding();
        stale.deployment_epoch_id = "epoch-old".to_string();
        let observation =
            observe_federated_bootstrap_node(&FixedPort(Ok(read_back(stale))), "node-a", &expected)
                .await;

        assert!(observation
            .blockers
            .contains(&"federated_bootstrap_requested_binding_mismatch".to_string()));
    }

    #[tokio::test]
    async fn unavailable_node_remains_explicitly_blocked() {
        let observation = observe_federated_bootstrap_node(
            &FixedPort(Err("unreachable".to_string())),
            "node-a",
            &binding(),
        )
        .await;

        assert!(observation
            .blockers
            .contains(&"federated_bootstrap_read_back_unavailable".to_string()));
    }
}
