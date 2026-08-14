//! Distro-managed providers — dynamic OS package manager detection with build-from-source fallback.

use std::path::Path;

use crate::domain::ecosystem_package::port::EcosystemPackageProvider;
use crate::domain::ecosystem_package::types::{
    EcosystemPackagePlan, InstallPrincipalKind, PrincipalInstallLayout, ProviderChannel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistroPackageManager {
    Apt,
    Pacman,
    Dnf,
    Apk,
    Brew,
    Winget,
    SourceFallback,
}

pub fn detect_distro_package_manager() -> DistroPackageManager {
    if Path::new("/usr/bin/apt-get").exists() || Path::new("/usr/bin/apt").exists() {
        DistroPackageManager::Apt
    } else if Path::new("/usr/bin/pacman").exists() {
        DistroPackageManager::Pacman
    } else if Path::new("/usr/bin/dnf").exists() || Path::new("/usr/bin/yum").exists() {
        DistroPackageManager::Dnf
    } else if Path::new("/sbin/apk").exists() {
        DistroPackageManager::Apk
    } else if Path::new("/opt/homebrew/bin/brew").exists() || Path::new("/usr/local/bin/brew").exists() {
        DistroPackageManager::Brew
    } else if cfg!(target_os = "windows") {
        DistroPackageManager::Winget
    } else {
        DistroPackageManager::SourceFallback
    }
}

pub struct DistroManagedProvider {
    pub id: &'static str,
    pub summary: &'static str,
    pub package_name: &'static str,
    pub source_repo_url: Option<&'static str>,
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
        let pm = detect_distro_package_manager();
        let install_cmd = match pm {
            DistroPackageManager::Apt => format!("sudo apt install -y {}", self.package_name),
            DistroPackageManager::Pacman => format!("sudo pacman -S --noconfirm {}", self.package_name),
            DistroPackageManager::Dnf => format!("sudo dnf install -y {}", self.package_name),
            DistroPackageManager::Apk => format!("sudo apk add {}", self.package_name),
            DistroPackageManager::Brew => format!("brew install {}", self.package_name),
            DistroPackageManager::Winget => format!("winget install {}", self.package_name),
            DistroPackageManager::SourceFallback => {
                if let Some(url) = self.source_repo_url {
                    format!("git clone --depth 1 {} /tmp/{} && make -C /tmp/{} && sudo make -C /tmp/{} install", url, self.id, self.id, self.id)
                } else {
                    format!("# build from official source: {}", self.summary)
                }
            }
        };

        let mut summary_commands = vec![
            format!("# detected_package_manager: {:?}", pm),
            format!("# canonical_install_command: {}", install_cmd),
        ];

        if let Some(url) = self.source_repo_url {
            summary_commands.push(format!("# official_source_repo: {}", url));
            summary_commands.push(format!("# build_from_source: git clone --depth 1 {} && make && sudo make install", url));
        }

        EcosystemPackagePlan {
            dep_id: self.id.into(),
            channel: ProviderChannel::DistroManaged,
            principal: InstallPrincipalKind::SystemManaged,
            layout: layout.clone(),
            reason: reason.to_string(),
            steps: vec![],
            auto_applicable: false,
            proofs: vec![format!("probe {id} via cynic deps", id = self.id)],
            summary_commands,
        }
    }
}

pub static DOCKER: DistroManagedProvider = DistroManagedProvider {
    id: "docker",
    summary: "container runtime (Docker Engine / CLI)",
    package_name: "docker.io",
    source_repo_url: Some("https://github.com/moby/moby.git"),
};

pub static SYSTEMD: DistroManagedProvider = DistroManagedProvider {
    id: "systemd",
    summary: "system and service manager",
    package_name: "systemd",
    source_repo_url: Some("https://github.com/systemd/systemd.git"),
};

pub static WIREGUARD_TOOLS: DistroManagedProvider = DistroManagedProvider {
    id: "wireguard-tools",
    summary: "synapse P2P mesh network tools (wg, wg-quick)",
    package_name: "wireguard-tools",
    source_repo_url: Some("https://git.zx2c4.com/wireguard-tools"),
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ecosystem_package::types::PrincipalInstallLayout;
    use std::path::PathBuf;

    #[test]
    fn test_distro_managed_provider_plans_wireguard() {
        let layout = PrincipalInstallLayout {
            runtime_user: "cynic".into(),
            home_dir: PathBuf::from("/var/lib/cynic"),
            bin_dir: PathBuf::from("/var/lib/cynic/.local/bin"),
            src_dir: PathBuf::from("/var/lib/cynic/src"),
            share_dir: PathBuf::from("/var/lib/cynic/share"),
            run_as: crate::domain::ecosystem_package::types::RunAs::Current,
            writable: true,
            notes: vec![],
        };

        let plan = WIREGUARD_TOOLS.plan("test upgrade", &layout);
        assert_eq!(plan.dep_id, "wireguard-tools");
        assert!(plan.summary_commands.iter().any(|c| c.contains("wireguard-tools")));
        assert!(plan.summary_commands.iter().any(|c| c.contains("git.zx2c4.com/wireguard-tools")));
    }
}
