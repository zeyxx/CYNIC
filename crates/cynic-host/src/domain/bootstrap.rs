use serde::{Deserialize, Serialize};

pub use cynic_protocol::SecretClass;

use crate::domain::shared::{
    BoundaryKind, LifecycleDisposition, LifecycleProjection, LifecycleStatus, SurfaceStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapFinding {
    pub id: String,
    pub status: SurfaceStatus,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapStatusReport {
    pub schema: &'static str,
    pub status: SurfaceStatus,
    pub lifecycle_status: LifecycleStatus,
    pub lifecycle_disposition: LifecycleDisposition,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    pub next_command: Option<String>,
    pub runtime_human_secret_dependency: bool,
    pub bridge_active: bool,
    pub host_substrate_active: bool,
    pub allowed_runtime_secret_classes: Vec<SecretClass>,
    pub forbidden_runtime_secret_classes: Vec<SecretClass>,
    pub findings: Vec<BootstrapFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapPlanReport {
    pub schema: &'static str,
    pub status: SurfaceStatus,
    pub bootstrap_secret_classes: Vec<SecretClass>,
    pub runtime_secret_classes: Vec<SecretClass>,
    pub forbidden_runtime_secret_classes: Vec<SecretClass>,
    pub bootstrap_mechanisms: Vec<&'static str>,
    pub runtime_mechanisms: Vec<&'static str>,
    pub forbidden_runtime_mechanisms: Vec<&'static str>,
    pub next_actions: Vec<String>,
}

pub fn build_bootstrap_status(
    bridge_active: bool,
    host_substrate_active: bool,
) -> BootstrapStatusReport {
    let runtime_human_secret_dependency = !bridge_active;
    let status = if bridge_active && host_substrate_active {
        SurfaceStatus::Active
    } else {
        SurfaceStatus::Blocked
    };
    let lifecycle_status = if !bridge_active {
        LifecycleStatus::Missing
    } else if !host_substrate_active {
        LifecycleStatus::PrincipalMismatch
    } else {
        LifecycleStatus::Ready
    };
    let projection = match lifecycle_status {
        LifecycleStatus::Ready => LifecycleProjection::ready_observe(
            "bootstrap bridge is active and host substrate is ready for steady-state runtime",
        ),
        LifecycleStatus::Missing => LifecycleProjection::missing_adopt(
            "bootstrap bridge is not yet installed or active",
            Some("cynic bootstrap bridge plan".to_string()),
        ),
        LifecycleStatus::PrincipalMismatch => LifecycleProjection::principal_mismatch_adopt(
            "bootstrap bridge is active but the runtime host substrate is not yet fully active",
            Some("cynic host bootstrap plan --json".to_string()),
        ),
        LifecycleStatus::ConnectionRequired => LifecycleProjection::connection_required_observe(
            "bootstrap lifecycle does not model a connection boundary",
        ),
    };

    let mut findings = vec![BootstrapFinding {
        id: "runtime_human_secret_dependency".to_string(),
        status: if runtime_human_secret_dependency {
            SurfaceStatus::Blocked
        } else {
            SurfaceStatus::Active
        },
        detail: if runtime_human_secret_dependency {
            "runtime still depends on a human sudo/bootstrap gesture on this machine".to_string()
        } else {
            "runtime can cross the privilege boundary through the canonical host-exec helper"
                .to_string()
        },
    }];
    findings.push(BootstrapFinding {
        id: "bootstrap_bridge".to_string(),
        status: if bridge_active {
            SurfaceStatus::Active
        } else {
            SurfaceStatus::Blocked
        },
        detail: "host-exec helper + sudoers bridge installation state".to_string(),
    });
    findings.push(BootstrapFinding {
        id: "host_substrate_runtime".to_string(),
        status: if host_substrate_active {
            SurfaceStatus::Active
        } else {
            SurfaceStatus::Blocked
        },
        detail: "principal, privilege, packages, and services runtime substrate state".to_string(),
    });

    BootstrapStatusReport {
        schema: "cynic.host_bootstrap.status.v1",
        status,
        lifecycle_status,
        lifecycle_disposition: projection.disposition,
        next_boundary_kind: projection.next_boundary_kind,
        next_boundary_reason: projection.next_boundary_reason,
        next_command: projection.next_command,
        runtime_human_secret_dependency,
        bridge_active,
        host_substrate_active,
        allowed_runtime_secret_classes: vec![SecretClass::RuntimeMachineIdentity],
        forbidden_runtime_secret_classes: vec![
            SecretClass::HumanAdmin,
            SecretClass::MachineEnrollment,
            SecretClass::RecoveryBreakglass,
        ],
        findings,
    }
}

pub fn build_bootstrap_plan(status: &BootstrapStatusReport) -> BootstrapPlanReport {
    let mut next_actions = Vec::new();
    if !status.bridge_active {
        next_actions.push("install_host_exec_bridge_once".to_string());
    }
    if !status.host_substrate_active {
        next_actions.push("repair_runtime_host_substrate_through_host_exec".to_string());
    }
    if status.runtime_human_secret_dependency {
        next_actions.push("eliminate_runtime_human_secret_dependency".to_string());
    }
    if let Some(command) = status.next_command.as_ref() {
        next_actions.push(command.clone());
    }
    next_actions.sort();
    next_actions.dedup();

    BootstrapPlanReport {
        schema: "cynic.host_bootstrap.plan.v1",
        status: status.status,
        bootstrap_secret_classes: vec![
            SecretClass::HumanAdmin,
            SecretClass::MachineEnrollment,
            SecretClass::RecoveryBreakglass,
        ],
        runtime_secret_classes: vec![SecretClass::RuntimeMachineIdentity],
        forbidden_runtime_secret_classes: vec![
            SecretClass::HumanAdmin,
            SecretClass::MachineEnrollment,
            SecretClass::RecoveryBreakglass,
        ],
        bootstrap_mechanisms: vec![
            "console-root",
            "cloud-init",
            "hypervisor-provisioning",
            "temporary-install-key",
        ],
        runtime_mechanisms: vec![
            "root-owned-helper",
            "sudoers-allowlist-for-helper-only",
            "machine-identity",
        ],
        forbidden_runtime_mechanisms: vec![
            "human-sudo-password-prompt",
            "stored-human-password",
            "generic-root-shell-helper",
            "shell-command-replay-from-db",
        ],
        next_actions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_status_projects_missing_bridge_as_adoptable_gap() {
        let report = build_bootstrap_status(false, false);

        assert_eq!(report.status, SurfaceStatus::Blocked);
        assert_eq!(report.lifecycle_status, LifecycleStatus::Missing);
        assert_eq!(
            report.lifecycle_disposition,
            LifecycleDisposition::NotDiscovered
        );
        assert_eq!(report.next_boundary_kind, BoundaryKind::Adopt);
        assert_eq!(
            report.next_command.as_deref(),
            Some("cynic bootstrap bridge plan")
        );
    }

    #[test]
    fn bootstrap_status_projects_ready_state_as_observable() {
        let report = build_bootstrap_status(true, true);

        assert_eq!(report.status, SurfaceStatus::Active);
        assert_eq!(report.lifecycle_status, LifecycleStatus::Ready);
        assert_eq!(
            report.lifecycle_disposition,
            LifecycleDisposition::ObservableCandidate
        );
        assert_eq!(report.next_boundary_kind, BoundaryKind::Observe);
        assert_eq!(report.next_command, None);
    }
}
