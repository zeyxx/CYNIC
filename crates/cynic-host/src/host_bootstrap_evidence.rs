use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;

use crate::adapters::local_observation::LocalHostObservationAdapter;
use crate::bootstrap_chain::build_bootstrap_chain_report_with_observer;
use crate::bridge::{HostExecBridgeStatus, bridge_status};
use crate::domain::shared::{
    BoundaryKind, LifecycleDisposition, LifecycleProjection, LifecycleStatus, SurfaceStatus,
};
use crate::filesystem_observation::path_exists;
use crate::ports::{
    HostFilesystemPort, HostPackageObservationPort, HostPrincipalObservationPort,
    HostPrivilegeObservationPort, HostServiceObservationPort,
};
use crate::profiles::HostProfile;
use crate::registry::HostProfileResolution;
use crate::runtime_context::observe_host_runtime_context_with_observer;

#[derive(Debug, Clone, Serialize)]
pub struct HostBootstrapEvidenceReport {
    pub schema: &'static str,
    pub status: SurfaceStatus,
    pub lifecycle_status: LifecycleStatus,
    pub lifecycle_disposition: LifecycleDisposition,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    pub next_command: Option<String>,
    pub host_profile: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_resolution: Option<HostProfileResolution>,
    pub target_stage: String,
    pub reached_stage: String,
    pub target_satisfied: bool,
    pub bridge: HostExecBridgeStatus,
    pub installed_cli_sha256: Option<String>,
    pub stages: Vec<HostBootstrapStageEvidence>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostBootstrapStageEvidence {
    pub stage: &'static str,
    pub status: SurfaceStatus,
    pub detail: String,
}

pub fn observe_host_bootstrap_evidence(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
) -> Result<HostBootstrapEvidenceReport, Box<dyn std::error::Error>> {
    observe_host_bootstrap_evidence_with_observer(profile, resolution, &LocalHostObservationAdapter)
}

pub fn observe_host_bootstrap_evidence_with_observer(
    profile: HostProfile,
    resolution: Option<HostProfileResolution>,
    observed: &(
         impl HostPrincipalObservationPort
         + HostPrivilegeObservationPort
         + HostPackageObservationPort
         + HostServiceObservationPort
     ),
) -> Result<HostBootstrapEvidenceReport, Box<dyn std::error::Error>> {
    let host_context =
        observe_host_runtime_context_with_observer(profile, resolution.clone(), observed);
    let host_chain =
        build_bootstrap_chain_report_with_observer(profile, resolution.clone(), observed)?;
    let filesystem_policy = profile.desired_filesystem();
    let bridge = bridge_status();
    let installed_cli_sha256 = file_sha256("/usr/local/bin/cynic");
    let managed_paths = filesystem_policy.managed_paths;
    let existing_paths = managed_paths
        .iter()
        .filter(|path| path_exists(&path.install_path))
        .count();

    let mut stages = Vec::with_capacity(7);
    stages.push(HostBootstrapStageEvidence {
        stage: "h0_host_identity_ready",
        status: if host_identity_ready(&host_context) {
            SurfaceStatus::Active
        } else {
            SurfaceStatus::Blocked
        },
        detail: format!(
            "hostname={} os={} arch={}",
            host_context
                .host_resolution
                .as_ref()
                .map(|resolution| resolution.hostname_observed.as_str())
                .unwrap_or("unknown"),
            std::env::consts::OS,
            std::env::consts::ARCH
        ),
    });
    stages.push(HostBootstrapStageEvidence {
        stage: "h1_principal_materialized",
        status: host_context.host_substrate.principal.status,
        detail: format!(
            "user_present={} group_present={} home_matches={} missing_state_dirs={}",
            host_context.host_substrate.principal.user_present,
            host_context.host_substrate.principal.group_present,
            host_context.host_substrate.principal.home_matches,
            host_context
                .host_substrate
                .principal
                .missing_state_dirs
                .len()
        ),
    });
    stages.push(HostBootstrapStageEvidence {
        stage: "h2_filesystem_projected",
        status: if existing_paths == managed_paths.len() {
            SurfaceStatus::Active
        } else {
            SurfaceStatus::Blocked
        },
        detail: format!(
            "managed_paths_present={existing_paths}/{}",
            managed_paths.len()
        ),
    });
    stages.push(HostBootstrapStageEvidence {
        stage: "h3_packages_ready",
        status: host_context.host_substrate.packages.status,
        detail: format!(
            "missing_packages={}",
            host_context
                .host_substrate
                .packages
                .missing_packages
                .join(",")
        ),
    });
    stages.push(HostBootstrapStageEvidence {
        stage: "h4_privilege_helpers_ready",
        status: host_context.host_substrate.privilege.status,
        detail: format!(
            "helper_dir_present={} sudoers_projection_present={} denial_proof_ready={}",
            host_context.host_substrate.privilege.helper_dir_present,
            host_context
                .host_substrate
                .privilege
                .sudoers_projection_present,
            host_context.host_substrate.privilege.denial_proof_ready
        ),
    });
    stages.push(HostBootstrapStageEvidence {
        stage: "h5_services_projected",
        status: host_context.host_substrate.services.status,
        detail: format!(
            "missing_units={} disabled_units={} inactive_units={}",
            host_context.host_substrate.services.missing_units.len(),
            host_context.host_substrate.services.disabled_units.len(),
            host_context.host_substrate.services.inactive_units.len()
        ),
    });
    stages.push(HostBootstrapStageEvidence {
        stage: "h6_host_actionable",
        status: host_context.host_bootstrap.status,
        detail: format!(
            "bootstrap_chain_status={:?} bridge_active={} runtime_human_secret_dependency={}",
            host_chain.status,
            host_context.host_bootstrap.bridge_active,
            host_context.host_bootstrap.runtime_human_secret_dependency
        ),
    });

    let reached_stage = highest_consecutive_stage(&stages).to_string();
    let target_stage = resolution
        .as_ref()
        .and_then(|value| value.host_bootstrap_target_stage.clone())
        .unwrap_or_else(|| "h6_host_actionable".to_string());
    let target_satisfied = reached_or_exceeds_stage(&reached_stage, &target_stage);
    let lifecycle_status = if target_satisfied {
        LifecycleStatus::Ready
    } else if reached_stage == "none" {
        LifecycleStatus::Missing
    } else {
        LifecycleStatus::PrincipalMismatch
    };
    let projection = match lifecycle_status {
        LifecycleStatus::Ready => LifecycleProjection::ready_observe(
            "declared bootstrap target stage is reached and the host is actionable at the current authority boundary",
        ),
        LifecycleStatus::Missing => LifecycleProjection::missing_adopt(
            "bootstrap evidence has not reached the first declared stage yet",
            Some("cynic bootstrap host plan --json".to_string()),
        ),
        LifecycleStatus::PrincipalMismatch => LifecycleProjection::principal_mismatch_adopt(
            "bootstrap evidence reached a partial stage but has not yet satisfied the declared target",
            Some("cynic bootstrap host plan --json".to_string()),
        ),
        LifecycleStatus::ConnectionRequired => LifecycleProjection::connection_required_observe(
            "bootstrap evidence does not model a connection boundary",
        ),
    };

    Ok(HostBootstrapEvidenceReport {
        schema: "cynic.host_bootstrap_evidence.v1",
        status: if target_satisfied {
            SurfaceStatus::Active
        } else {
            SurfaceStatus::Blocked
        },
        lifecycle_status,
        lifecycle_disposition: projection.disposition,
        next_boundary_kind: projection.next_boundary_kind,
        next_boundary_reason: projection.next_boundary_reason,
        next_command: projection.next_command,
        host_profile: profile.name(),
        host_resolution: resolution,
        target_stage,
        reached_stage,
        target_satisfied,
        bridge,
        installed_cli_sha256,
        stages,
    })
}

fn file_sha256(path: &str) -> Option<String> {
    fs::read(path)
        .ok()
        .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
}

fn host_identity_ready(context: &crate::runtime_context::HostRuntimeContext) -> bool {
    let hostname = context
        .host_resolution
        .as_ref()
        .map(|resolution| resolution.hostname_observed.as_str())
        .unwrap_or("");
    !hostname.is_empty() && !std::env::consts::OS.is_empty() && !std::env::consts::ARCH.is_empty()
}

fn highest_consecutive_stage(stages: &[HostBootstrapStageEvidence]) -> &'static str {
    let mut reached = "none";
    for stage in stages {
        if stage.status == SurfaceStatus::Active {
            reached = stage.stage;
        } else {
            break;
        }
    }
    reached
}

fn reached_or_exceeds_stage(reached: &str, target: &str) -> bool {
    match (stage_index(reached), stage_index(target)) {
        (Some(reached), Some(target)) => reached >= target,
        _ => false,
    }
}

fn stage_index(stage: &str) -> Option<usize> {
    Some(match stage {
        "h0_host_identity_ready" => 0,
        "h1_principal_materialized" => 1,
        "h2_filesystem_projected" => 2,
        "h3_packages_ready" => 3,
        "h4_privilege_helpers_ready" => 4,
        "h5_services_projected" => 5,
        "h6_host_actionable" => 6,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::packages::PackageObservedState;
    use crate::domain::principal::PrincipalObservedState;
    use crate::domain::privilege::PrivilegeObservedState;
    use crate::domain::services::{ServiceObservedState, ServiceObservedUnit};
    use crate::ports::{
        HostPackageObservationPort, HostPrincipalObservationPort, HostPrivilegeObservationPort,
        HostServiceObservationPort,
    };

    struct ReadyObserver;

    impl HostPrincipalObservationPort for ReadyObserver {
        fn observe_principal(
            &self,
            policy: &crate::domain::principal::PrincipalPolicy,
        ) -> PrincipalObservedState {
            PrincipalObservedState {
                user_present: true,
                group_present: true,
                observed_home_dir: Some(policy.home_dir.clone()),
                present_state_dirs: policy.state_dirs.clone(),
                owned_paths: policy.managed_paths(),
                ownership_unknown_paths: Vec::new(),
            }
        }
    }

    impl HostPrivilegeObservationPort for ReadyObserver {
        fn observe_privilege(
            &self,
            _policy: &crate::domain::privilege::PrivilegePolicy,
        ) -> PrivilegeObservedState {
            PrivilegeObservedState {
                helper_dir_present: true,
                sudoers_projection_present: true,
                denial_proof_ready: true,
            }
        }
    }

    impl HostPackageObservationPort for ReadyObserver {
        fn observe_packages(
            &self,
            policy: &crate::domain::packages::PackagePolicy,
        ) -> PackageObservedState {
            crate::domain::packages::promoted_package_state(policy.bootstrap_packages.clone())
        }
    }

    impl HostServiceObservationPort for ReadyObserver {
        fn observe_services(
            &self,
            policy: &crate::domain::services::ServicePolicy,
        ) -> ServiceObservedState {
            ServiceObservedState {
                units: policy
                    .units
                    .iter()
                    .map(|unit| ServiceObservedUnit {
                        unit_name: unit.unit_name.clone(),
                        present: true,
                        enabled: true,
                        active: true,
                    })
                    .collect(),
            }
        }
    }

    struct MissingPrivilegeObserver;

    impl HostPrincipalObservationPort for MissingPrivilegeObserver {
        fn observe_principal(
            &self,
            policy: &crate::domain::principal::PrincipalPolicy,
        ) -> PrincipalObservedState {
            ReadyObserver.observe_principal(policy)
        }
    }

    impl HostPackageObservationPort for MissingPrivilegeObserver {
        fn observe_packages(
            &self,
            policy: &crate::domain::packages::PackagePolicy,
        ) -> PackageObservedState {
            ReadyObserver.observe_packages(policy)
        }
    }

    impl HostServiceObservationPort for MissingPrivilegeObserver {
        fn observe_services(
            &self,
            policy: &crate::domain::services::ServicePolicy,
        ) -> ServiceObservedState {
            ReadyObserver.observe_services(policy)
        }
    }

    impl HostPrivilegeObservationPort for MissingPrivilegeObserver {
        fn observe_privilege(
            &self,
            _policy: &crate::domain::privilege::PrivilegePolicy,
        ) -> PrivilegeObservedState {
            PrivilegeObservedState {
                helper_dir_present: false,
                sudoers_projection_present: false,
                denial_proof_ready: false,
            }
        }
    }

    #[test]
    fn host_bootstrap_evidence_reports_target_satisfied_when_stage_is_reached() {
        let resolution = HostProfileResolution {
            hostname_observed: "cynic-build-runner".to_string(),
            matched_node: Some("cynic-build-runner".to_string()),
            matched_role: Some("build_runner".to_string()),
            canonical_role: None,
            profile_applied: "network-node",
            match_source: "node_name",
            host_bootstrap_target_stage: Some("h2_filesystem_projected".to_string()),
            host_bootstrap_required_domains: vec!["identity".to_string()],
            registry_path: Some("infra/registry.json".to_string()),
        };
        let report = observe_host_bootstrap_evidence_with_observer(
            HostProfile::NetworkNode,
            Some(resolution),
            &ReadyObserver,
        )
        .expect("evidence");
        assert_eq!(report.schema, "cynic.host_bootstrap_evidence.v1");
        // Target h2 is satisfiable; h3+ are not, because the profile policy
        // declares an unresolved distribution authority by design.
        assert!(
            report.target_satisfied,
            "stages: {:?}",
            report
                .stages
                .iter()
                .map(|stage| (stage.stage, stage.status, stage.detail.as_str()))
                .collect::<Vec<_>>()
        );
        assert_eq!(report.reached_stage, "h2_filesystem_projected");
        assert_eq!(report.lifecycle_status, LifecycleStatus::Ready);
        assert_eq!(report.next_boundary_kind, BoundaryKind::Observe);
    }

    #[test]
    fn host_bootstrap_evidence_blocks_when_target_stage_is_not_reached() {
        let resolution = HostProfileResolution {
            hostname_observed: "cynic-cortex".to_string(),
            matched_node: Some("cynic-cortex".to_string()),
            matched_role: Some("cortex_host".to_string()),
            canonical_role: None,
            profile_applied: "network-node",
            match_source: "node_name",
            host_bootstrap_target_stage: Some("h4_privilege_helpers_ready".to_string()),
            host_bootstrap_required_domains: vec!["privilege".to_string()],
            registry_path: Some("infra/registry.json".to_string()),
        };
        let report = observe_host_bootstrap_evidence_with_observer(
            HostProfile::NetworkNode,
            Some(resolution),
            &MissingPrivilegeObserver,
        )
        .expect("evidence");
        assert_eq!(report.status, SurfaceStatus::Blocked);
        assert!(!report.target_satisfied);
        // h3_packages_ready cannot be reached from a profile policy: families.rs
        // declares provenance as unresolved_distribution_authority() on purpose,
        // so the package domain is Blocked by design until an authority is
        // selected — no observation can satisfy it. The reachable consecutive
        // stage is therefore h2, and h3 is the first blocker.
        assert_eq!(
            report.reached_stage,
            "h2_filesystem_projected",
            "stages: {:?}",
            report
                .stages
                .iter()
                .map(|stage| (stage.stage, stage.status, stage.detail.as_str()))
                .collect::<Vec<_>>()
        );
        let packages_stage = report
            .stages
            .iter()
            .find(|stage| stage.stage == "h3_packages_ready")
            .expect("h3 stage present");
        assert_eq!(
            packages_stage.status,
            SurfaceStatus::Blocked,
            "an unselected distribution authority must block the package stage"
        );
        assert_eq!(report.lifecycle_status, LifecycleStatus::PrincipalMismatch);
        assert_eq!(report.next_boundary_kind, BoundaryKind::Adopt);
        assert_eq!(
            report.next_command.as_deref(),
            Some("cynic bootstrap host plan --json")
        );
    }
}
