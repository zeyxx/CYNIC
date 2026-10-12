//! Pure placement decision predicates for compute workloads (Zero-I/O).

use super::types::{ComputeRequirements, RemoteRunnerEnrollmentPlan, RemoteRunnerSpec, SubstrateKind};
use crate::agent_isolation::SandboxStrategy;

/// Observed capability of an available compute substrate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubstrateObservation {
    pub name: String,
    pub kind: SubstrateKind,
    pub available_vcpus: u32,
    pub available_memory_bytes: u64,
    pub available_disk_bytes: u64,
    pub has_gpu: bool,
    pub supported_strategies: Vec<SandboxStrategy>,
}

/// Placement decision for an admitted workload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementDecision {
    pub admitted: bool,
    pub selected_substrate: Option<String>,
    pub selected_strategy: Option<SandboxStrategy>,
    pub reason: String,
}

/// Pure predicate deciding the optimal substrate for a compute workload.
#[must_use]
pub fn decide_compute_placement(
    requirements: &ComputeRequirements,
    substrates: &[SubstrateObservation],
) -> PlacementDecision {
    if substrates.is_empty() {
        return PlacementDecision {
            admitted: false,
            selected_substrate: None,
            selected_strategy: None,
            reason: "no compute substrates available in fleet".to_string(),
        };
    }

    // Try to match preferred strategy first
    for sub in substrates {
        if !sub.supported_strategies.contains(&requirements.preferred_strategy) {
            continue;
        }
        if sub.available_vcpus < requirements.vcpus {
            continue;
        }
        if sub.available_memory_bytes < requirements.memory_bytes {
            continue;
        }
        if sub.available_disk_bytes < requirements.disk_bytes {
            continue;
        }
        if requirements.requires_gpu && !sub.has_gpu {
            continue;
        }

        return PlacementDecision {
            admitted: true,
            selected_substrate: Some(sub.name.clone()),
            selected_strategy: Some(requirements.preferred_strategy),
            reason: format!(
                "matched preferred strategy {:?} on substrate '{}'",
                requirements.preferred_strategy, sub.name
            ),
        };
    }

    // Fallback: match any allowed strategy
    for allowed in &requirements.allowed_strategies {
        for sub in substrates {
            if !sub.supported_strategies.contains(allowed) {
                continue;
            }
            if sub.available_vcpus < requirements.vcpus
                || sub.available_memory_bytes < requirements.memory_bytes
                || sub.available_disk_bytes < requirements.disk_bytes
            {
                continue;
            }
            if requirements.requires_gpu && !sub.has_gpu {
                continue;
            }

            return PlacementDecision {
                admitted: true,
                selected_substrate: Some(sub.name.clone()),
                selected_strategy: Some(*allowed),
                reason: format!(
                    "fallback matched strategy {:?} on substrate '{}'",
                    allowed, sub.name
                ),
            };
        }
    }

    PlacementDecision {
        admitted: false,
        selected_substrate: None,
        selected_strategy: None,
        reason: "insufficient compute capacity or incompatible isolation profile across fleet"
            .to_string(),
    }
}

/// Pure predicate validating a remote runner spec and producing a bounded mesh enrollment plan.
pub fn plan_remote_runner_enrollment(
    spec: &RemoteRunnerSpec,
) -> Result<RemoteRunnerEnrollmentPlan, String> {
    if spec.node_name.trim().is_empty() {
        return Err("invalid_node_name: node_name cannot be empty".to_string());
    }
    if !spec
        .node_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "invalid_node_name: '{}' contains illegal characters",
            spec.node_name
        ));
    }
    if spec.vcpus == 0 {
        return Err("invalid_capacity: vcpus must be >= 1".to_string());
    }
    if spec.memory_bytes < 512 * 1024 * 1024 {
        return Err("invalid_capacity: memory must be >= 512 MiB".to_string());
    }
    if spec.wireguard_pubkey.trim().len() < 40 {
        return Err("invalid_wireguard_pubkey: public key must be at least 40 characters".to_string());
    }

    Ok(RemoteRunnerEnrollmentPlan {
        node_name: spec.node_name.clone(),
        user: "cynic".to_string(),
        acl_tags: vec!["tag:runner".to_string()],
        expiration_seconds: 3600,
        reusable: false,
    })
}
