//! Fleet probe worker and aggregation helpers.

use super::config::FleetTarget;
use crate::domain::probe::{DogHealthDetails, ProbeStatus};
use crate::infra::probes::fleet::context_n_ctx_from_result;
use cynic_node::verify::IdentityResult;
use std::time::Instant;

pub(super) async fn probe_target(client: reqwest::Client, target: FleetTarget) -> DogHealthDetails {
    let FleetTarget {
        dog_name,
        health_url,
        props_url,
        models_url,
        expected_n_ctx,
        expected_model,
        api_key,
    } = target;

    let probe_start = Instant::now();

    // Health, context and identity are all observed by cynic-node; this probe only
    // projects node verification into the kernel's FleetDetails report. Observing
    // HTTP directly here would duplicate the node's observation contract.
    let reachable =
        cynic_node::verify::check_health_with_bearer(&client, &health_url, 5, api_key.as_deref())
            .await;

    let mut actual_n_ctx = None;
    if let Some(ref props_url) = props_url {
        let context = cynic_node::verify::check_context_with_bearer(
            &client,
            props_url,
            5,
            api_key.as_deref(),
        )
        .await;
        if let Some(n_ctx) = context_n_ctx_from_result(&context) {
            actual_n_ctx = Some(n_ctx);
        }
    }

    let mut actual_model = None;
    let mut model_mismatch = false;

    if reachable && let Some(ref models_url) = models_url {
        match cynic_node::verify::check_identity_with_bearer(
            &client,
            models_url,
            &expected_model,
            5,
            api_key.as_deref(),
        )
        .await
        {
            IdentityResult::Match | IdentityResult::Unknown => {}
            IdentityResult::Unreachable => {
                actual_model = Some("unreachable".to_string());
            }
            IdentityResult::Mismatch { actual, .. } => {
                actual_model = Some(actual);
                model_mismatch = !expected_model.is_empty();
            }
        }
    }

    let latency_ms = Some(probe_start.elapsed().as_millis() as u64);
    DogHealthDetails {
        dog_name,
        reachable,
        latency_ms,
        actual_n_ctx,
        expected_n_ctx: if expected_n_ctx > 0 {
            Some(expected_n_ctx)
        } else {
            None
        },
        actual_model,
        expected_model: if expected_model.is_empty() {
            None
        } else {
            Some(expected_model)
        },
        model_mismatch,
    }
}

pub(super) fn derive_status(dogs: &[DogHealthDetails]) -> ProbeStatus {
    let reachable_count = dogs.iter().filter(|d| d.reachable).count();
    let total = dogs.len();

    if reachable_count == total {
        ProbeStatus::Ok
    } else if reachable_count == 0 {
        ProbeStatus::Unavailable
    } else {
        ProbeStatus::Degraded
    }
}

pub(super) fn log_health_findings(dogs: &[DogHealthDetails]) {
    for dog in dogs {
        if !dog.reachable {
            tracing::warn!(
                dog = %dog.dog_name,
                latency_ms = ?dog.latency_ms,
                "fleet probe: Dog unreachable"
            );
        }

        if let (Some(actual), Some(expected)) = (dog.actual_n_ctx, dog.expected_n_ctx)
            && actual < expected
        {
            tracing::error!(
                dog = %dog.dog_name,
                actual = actual,
                expected = expected,
                "fleet probe: CONTEXT DRIFT DETECTED — backend running with reduced context"
            );
            klog!(
                "[fleet] ⚠ {} CONTEXT DRIFT: running {} (configured {})",
                dog.dog_name,
                actual,
                expected
            );
        }

        if dog.model_mismatch {
            tracing::error!(
                dog = %dog.dog_name,
                actual_model = ?dog.actual_model,
                expected_model = ?dog.expected_model,
                "fleet probe: MODEL IDENTITY MISMATCH — wrong model loaded on backend"
            );
            klog!(
                "[fleet] ⚠ {} MODEL MISMATCH: running {:?} (configured {:?})",
                dog.dog_name,
                dog.actual_model,
                dog.expected_model
            );
        }
    }
}
