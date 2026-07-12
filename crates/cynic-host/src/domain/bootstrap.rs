use serde::{Deserialize, Serialize};

use crate::domain::shared::SurfaceStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretClass {
    HumanAdmin,
    MachineEnrollment,
    RuntimeMachineIdentity,
    RecoveryBreakglass,
}

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
    pub next_actions: Vec<&'static str>,
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
        next_actions.push("install_host_exec_bridge_once");
    }
    if !status.host_substrate_active {
        next_actions.push("repair_runtime_host_substrate_through_host_exec");
    }
    if status.runtime_human_secret_dependency {
        next_actions.push("eliminate_runtime_human_secret_dependency");
    }

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
