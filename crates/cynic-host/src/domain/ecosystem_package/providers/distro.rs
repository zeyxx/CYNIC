//! Distro-managed providers — formal channel, plan-only (no CYNIC apply).

use crate::domain::ecosystem_package::port::EcosystemPackageProvider;
use crate::domain::ecosystem_package::types::{
    EcosystemPackagePlan, InstallPrincipalKind, PrincipalInstallLayout, ProviderChannel,
};

pub struct DistroManagedProvider {
    pub id: &'static str,
    pub summary: &'static str,
}

impl EcosystemPackageProvider for DistroManagedProvider {
    fn id(&self) -> &'static str {
        self.id
    }

    fn channel(&self) -> ProviderChannel {
        ProviderChannel::DistroManaged
    }

    fn principal_kind(&self) -> InstallPrincipalKind {
        InstallPrincipalKind::SystemManaged
    }

    fn plan(&self, reason: &str, layout: &PrincipalInstallLayout) -> EcosystemPackagePlan {
        EcosystemPackagePlan {
            dep_id: self.id.into(),
            channel: ProviderChannel::DistroManaged,
            principal: InstallPrincipalKind::SystemManaged,
            layout: layout.clone(),
            reason: reason.to_string(),
            steps: vec![],
            auto_applicable: false,
            proofs: vec![format!("probe {id} via cynic deps", id = self.id)],
            summary_commands: vec![
                format!("# distro_managed: {}", self.summary),
                format!("# use host package domain / distro tools for {}", self.id),
            ],
        }
    }
}

pub static DOCKER: DistroManagedProvider = DistroManagedProvider {
    id: "docker",
    summary: "container runtime (apt/dnf/…)",
};

pub static SYSTEMD: DistroManagedProvider = DistroManagedProvider {
    id: "systemd",
    summary: "service manager (distro upgrade)",
};

pub static WIREGUARD_TOOLS: DistroManagedProvider = DistroManagedProvider {
    id: "wireguard-tools",
    summary: "synapse P2P mesh network tools (apt/dnf install wireguard-tools)",
};
