use serde::{Deserialize, Serialize};

pub const FEDERATED_BOOTSTRAP_SCHEMA: &str = "cynic.federated_bootstrap.v1";

/// The only phases allowed to advance the first node of a fleet.  The seed is
/// deliberately before the scheduler: a scheduler cannot be the authority
/// that installs the authority which is required to run it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FederatedBootstrapPhase {
    SeedAdmitted,
    BootstrapControllerReady,
    ProviderReady,
    NodeEnrolled,
    StorageReady,
    NodeReady,
    SchedulerReady,
    BuildProven,
    ReleaseReadBack,
    Complete,
}

impl FederatedBootstrapPhase {
    pub const fn ordinal(self) -> u8 {
        match self {
            Self::SeedAdmitted => 0,
            Self::BootstrapControllerReady => 1,
            Self::ProviderReady => 2,
            Self::NodeEnrolled => 3,
            Self::StorageReady => 4,
            Self::NodeReady => 5,
            Self::SchedulerReady => 6,
            Self::BuildProven => 7,
            Self::ReleaseReadBack => 8,
            Self::Complete => 9,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedBootstrapObservation {
    pub schema: String,
    pub node_id: String,
    pub epoch_id: String,
    pub binding: FederatedBootstrapBinding,
    pub phase: FederatedBootstrapPhase,
    pub seed_admitted: bool,
    pub bootstrap_controller_ready: bool,
    pub provider_ready: bool,
    pub node_enrolled: bool,
    pub storage_ready: bool,
    pub node_ready: bool,
    pub scheduler_ready: bool,
    pub build_proven: bool,
    pub release_read_back: bool,
    #[serde(default)]
    pub blockers: Vec<String>,
}

/// Runtime facts collected by adapters before the pure bootstrap reconciler.
/// The adapter owns how a fact was observed; this constructor owns the
/// canonical mapping from missing facts to bootstrap blockers and phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedBootstrapReadBack {
    pub node_id: String,
    pub epoch_id: String,
    pub binding: FederatedBootstrapBinding,
    pub seed_admitted: bool,
    pub bootstrap_controller_ready: bool,
    pub provider_ready: bool,
    pub node_enrolled: bool,
    pub storage_ready: bool,
    pub node_ready: bool,
    pub scheduler_ready: bool,
    pub build_proven: bool,
    pub release_read_back: bool,
    #[serde(default)]
    pub blockers: Vec<String>,
}

/// Identity shared by every fact assembled into one federated bootstrap
/// observation. A boolean from an older build, a different node deployment,
/// or an unknown instant must not advance a newer deployment epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedBootstrapBinding {
    pub source_commit: String,
    pub deployment_epoch_id: String,
    pub observed_at_unix_ms: u128,
    pub evidence_refs: Vec<String>,
}

fn binding_blockers(epoch_id: &str, binding: &FederatedBootstrapBinding) -> Vec<String> {
    let mut blockers = Vec::new();
    if binding.source_commit.len() != 40
        || !binding
            .source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        blockers.push("federated_bootstrap_source_commit_unbound".to_string());
    }
    if binding.deployment_epoch_id.is_empty() || binding.deployment_epoch_id != epoch_id {
        blockers.push("federated_bootstrap_epoch_binding_mismatch".to_string());
    }
    if binding.observed_at_unix_ms == 0 {
        blockers.push("federated_bootstrap_observation_time_unbound".to_string());
    }
    if binding.evidence_refs.is_empty() {
        blockers.push("federated_bootstrap_evidence_unbound".to_string());
    }
    blockers
}

pub fn federated_bootstrap_observation(
    read_back: FederatedBootstrapReadBack,
) -> FederatedBootstrapObservation {
    let phase = observed_phase(&read_back);
    let mut blockers = read_back.blockers;
    blockers.extend(binding_blockers(&read_back.epoch_id, &read_back.binding));
    let required = [
        (
            !read_back.seed_admitted,
            "seed_attestation_read_back_required",
        ),
        (
            !read_back.bootstrap_controller_ready,
            "runtime_seed_capability_read_back_required",
        ),
        (!read_back.provider_ready, "provider_read_back_required"),
        (
            !read_back.node_enrolled,
            "node_enrollment_read_back_required",
        ),
        (!read_back.storage_ready, "storage_read_back_required"),
        (!read_back.scheduler_ready, "scheduler_read_back_required"),
        (!read_back.build_proven, "build_proof_read_back_required"),
        (!read_back.release_read_back, "release_read_back_required"),
    ];
    blockers.extend(
        required
            .into_iter()
            .filter(|(missing, _)| *missing)
            .map(|(_, blocker)| blocker.to_string()),
    );
    blockers.sort();
    blockers.dedup();

    FederatedBootstrapObservation {
        schema: FEDERATED_BOOTSTRAP_SCHEMA.to_string(),
        node_id: read_back.node_id,
        epoch_id: read_back.epoch_id,
        binding: read_back.binding,
        phase,
        seed_admitted: read_back.seed_admitted,
        bootstrap_controller_ready: read_back.bootstrap_controller_ready,
        provider_ready: read_back.provider_ready,
        node_enrolled: read_back.node_enrolled,
        storage_ready: read_back.storage_ready,
        node_ready: read_back.node_ready,
        scheduler_ready: read_back.scheduler_ready,
        build_proven: read_back.build_proven,
        release_read_back: read_back.release_read_back,
        blockers,
    }
}

fn observed_phase(read_back: &FederatedBootstrapReadBack) -> FederatedBootstrapPhase {
    if read_back.release_read_back {
        FederatedBootstrapPhase::ReleaseReadBack
    } else if read_back.build_proven {
        FederatedBootstrapPhase::BuildProven
    } else if read_back.scheduler_ready {
        FederatedBootstrapPhase::SchedulerReady
    } else if read_back.node_ready {
        FederatedBootstrapPhase::NodeReady
    } else if read_back.storage_ready {
        FederatedBootstrapPhase::StorageReady
    } else if read_back.node_enrolled {
        FederatedBootstrapPhase::NodeEnrolled
    } else if read_back.provider_ready {
        FederatedBootstrapPhase::ProviderReady
    } else if read_back.bootstrap_controller_ready {
        FederatedBootstrapPhase::BootstrapControllerReady
    } else {
        FederatedBootstrapPhase::SeedAdmitted
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedBootstrapPlan {
    pub schema: String,
    pub node_id: String,
    pub epoch_id: String,
    pub observed_phase: FederatedBootstrapPhase,
    pub next_phase: Option<FederatedBootstrapPhase>,
    pub actionable: bool,
    pub blockers: Vec<String>,
}

pub fn reconcile_federated_bootstrap(
    observation: &FederatedBootstrapObservation,
) -> FederatedBootstrapPlan {
    let mut blockers = observation.blockers.clone();
    if observation.schema != FEDERATED_BOOTSTRAP_SCHEMA {
        blockers.push("federated_bootstrap_observation_schema_mismatch".to_string());
    }
    if observation.node_id.is_empty() {
        blockers.push("federated_bootstrap_node_id_missing".to_string());
    }
    if observation.epoch_id.is_empty() {
        blockers.push("federated_bootstrap_epoch_missing".to_string());
    }
    blockers.extend(binding_blockers(
        &observation.epoch_id,
        &observation.binding,
    ));

    let facts = [
        (
            FederatedBootstrapPhase::SeedAdmitted,
            observation.seed_admitted,
        ),
        (
            FederatedBootstrapPhase::BootstrapControllerReady,
            observation.bootstrap_controller_ready,
        ),
        (
            FederatedBootstrapPhase::ProviderReady,
            observation.provider_ready,
        ),
        (
            FederatedBootstrapPhase::NodeEnrolled,
            observation.node_enrolled,
        ),
        (
            FederatedBootstrapPhase::StorageReady,
            observation.storage_ready,
        ),
        (FederatedBootstrapPhase::NodeReady, observation.node_ready),
        (
            FederatedBootstrapPhase::SchedulerReady,
            observation.scheduler_ready,
        ),
        (
            FederatedBootstrapPhase::BuildProven,
            observation.build_proven,
        ),
        (
            FederatedBootstrapPhase::ReleaseReadBack,
            observation.release_read_back,
        ),
    ];
    let mut next_phase = None;
    for (phase, ready) in facts {
        if !ready {
            next_phase = Some(phase);
            break;
        }
    }
    if next_phase.is_none() && blockers.is_empty() {
        next_phase = Some(FederatedBootstrapPhase::Complete);
    }

    if let Some(next) = next_phase {
        let prerequisite_ready = match next {
            FederatedBootstrapPhase::SeedAdmitted => true,
            FederatedBootstrapPhase::BootstrapControllerReady => observation.seed_admitted,
            FederatedBootstrapPhase::ProviderReady => {
                observation.seed_admitted && observation.bootstrap_controller_ready
            }
            FederatedBootstrapPhase::NodeEnrolled => {
                observation.seed_admitted
                    && observation.bootstrap_controller_ready
                    && observation.provider_ready
            }
            FederatedBootstrapPhase::StorageReady => {
                observation.seed_admitted
                    && observation.bootstrap_controller_ready
                    && observation.provider_ready
                    && observation.node_enrolled
            }
            FederatedBootstrapPhase::NodeReady => {
                observation.seed_admitted
                    && observation.bootstrap_controller_ready
                    && observation.provider_ready
                    && observation.node_enrolled
                    && observation.storage_ready
            }
            FederatedBootstrapPhase::SchedulerReady => {
                observation.seed_admitted
                    && observation.bootstrap_controller_ready
                    && observation.provider_ready
                    && observation.node_enrolled
                    && observation.storage_ready
                    && observation.node_ready
            }
            FederatedBootstrapPhase::BuildProven => observation.scheduler_ready,
            FederatedBootstrapPhase::ReleaseReadBack => observation.build_proven,
            FederatedBootstrapPhase::Complete => observation.release_read_back,
        };
        if !prerequisite_ready {
            blockers.push("federated_bootstrap_prerequisite_missing".to_string());
        }
    }

    blockers.sort();
    blockers.dedup();
    let actionable = blockers.is_empty() && next_phase.is_some();
    FederatedBootstrapPlan {
        schema: FEDERATED_BOOTSTRAP_SCHEMA.to_string(),
        node_id: observation.node_id.clone(),
        epoch_id: observation.epoch_id.clone(),
        observed_phase: observation.phase,
        next_phase,
        actionable,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation() -> FederatedBootstrapObservation {
        FederatedBootstrapObservation {
            schema: FEDERATED_BOOTSTRAP_SCHEMA.to_string(),
            node_id: "cynic-builder".to_string(),
            epoch_id: "epoch-1".to_string(),
            binding: binding("epoch-1"),
            phase: FederatedBootstrapPhase::SeedAdmitted,
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

    fn binding(epoch_id: &str) -> FederatedBootstrapBinding {
        FederatedBootstrapBinding {
            source_commit: "a".repeat(40),
            deployment_epoch_id: epoch_id.to_string(),
            observed_at_unix_ms: 1,
            evidence_refs: vec!["sha256:bootstrap-read-back".to_string()],
        }
    }

    #[test]
    fn scheduler_cannot_bootstrap_itself() {
        let plan = reconcile_federated_bootstrap(&observation());
        assert_eq!(
            plan.next_phase,
            Some(FederatedBootstrapPhase::BootstrapControllerReady)
        );
        assert!(plan.actionable);

        let mut blocked = observation();
        blocked.seed_admitted = false;
        blocked.scheduler_ready = true;
        let plan = reconcile_federated_bootstrap(&blocked);
        assert_eq!(plan.next_phase, Some(FederatedBootstrapPhase::SeedAdmitted));
        assert!(plan.blockers.is_empty());
    }

    #[test]
    fn complete_requires_release_read_back() {
        let mut complete = observation();
        complete.provider_ready = true;
        complete.bootstrap_controller_ready = true;
        complete.node_enrolled = true;
        complete.storage_ready = true;
        complete.node_ready = true;
        complete.scheduler_ready = true;
        complete.build_proven = true;
        complete.release_read_back = true;
        complete.phase = FederatedBootstrapPhase::ReleaseReadBack;
        let plan = reconcile_federated_bootstrap(&complete);
        assert_eq!(plan.next_phase, Some(FederatedBootstrapPhase::Complete));
        assert!(plan.actionable);
    }

    #[test]
    fn read_back_constructor_owns_missing_fact_blockers_and_phase() {
        let observation = federated_bootstrap_observation(FederatedBootstrapReadBack {
            node_id: "node-a".to_string(),
            epoch_id: "epoch-a".to_string(),
            binding: binding("epoch-a"),
            seed_admitted: true,
            bootstrap_controller_ready: true,
            provider_ready: true,
            node_enrolled: true,
            storage_ready: true,
            node_ready: true,
            scheduler_ready: true,
            build_proven: true,
            release_read_back: false,
            blockers: vec!["adapter_unknown".to_string(), "adapter_unknown".to_string()],
        });
        assert_eq!(observation.phase, FederatedBootstrapPhase::BuildProven);
        assert_eq!(
            observation.blockers,
            vec![
                "adapter_unknown".to_string(),
                "release_read_back_required".to_string()
            ]
        );
    }

    #[test]
    fn mixed_epoch_or_unbound_evidence_cannot_advance_bootstrap() {
        let mut complete = observation();
        complete.provider_ready = true;
        complete.bootstrap_controller_ready = true;
        complete.node_enrolled = true;
        complete.storage_ready = true;
        complete.node_ready = true;
        complete.scheduler_ready = true;
        complete.build_proven = true;
        complete.release_read_back = true;
        complete.phase = FederatedBootstrapPhase::ReleaseReadBack;
        complete.binding.deployment_epoch_id = "epoch-old".to_string();

        let plan = reconcile_federated_bootstrap(&complete);
        assert!(!plan.actionable);
        assert!(
            plan.blockers
                .contains(&"federated_bootstrap_epoch_binding_mismatch".to_string())
        );
    }
}
