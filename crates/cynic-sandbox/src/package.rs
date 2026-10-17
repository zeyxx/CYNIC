//! System dependency observation.
//!
//! Observes whether required sandbox tools are present (bwrap, nftables, tmux).
//! Does NOT install them — installation is a separate explicit capability action.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

pub const PACKAGE_OBSERVATION_SCHEMA: &str = "cynic.sandbox.package_observation.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageObservation {
    pub schema: String,
    pub name: String,
    pub present: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyReport {
    pub schema: String,
    pub packages: Vec<PackageObservation>,
}

pub trait PackageObservationPort {
    /// Check if a command-line tool is available.
    fn observe_package(&self, name: &str) -> PackageObservation;
    /// Check all sandbox dependencies at once.
    fn observe_sandbox_dependencies(&self) -> DependencyReport;
}

pub struct SystemPackageObserver;

impl SystemPackageObserver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemPackageObserver {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageObservationPort for SystemPackageObserver {
    fn observe_package(&self, name: &str) -> PackageObservation {
        let which = Command::new("which").arg(name).output();
        match which {
            Ok(output) if output.status.success() => {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let version = self.guess_version(name);
                PackageObservation {
                    schema: PACKAGE_OBSERVATION_SCHEMA.to_string(),
                    name: name.to_string(),
                    present: true,
                    path: Some(path),
                    version,
                    error: None,
                }
            }
            _ => PackageObservation {
                schema: PACKAGE_OBSERVATION_SCHEMA.to_string(),
                name: name.to_string(),
                present: false,
                path: None,
                version: None,
                error: Some(format!("{} not found in PATH", name)),
            },
        }
    }

    fn observe_sandbox_dependencies(&self) -> DependencyReport {
        let packages = vec![
            self.observe_package("bwrap"),
            self.observe_package("nft"),
            self.observe_package("ip"),
            self.observe_package("tmux"),
        ];
        DependencyReport {
            schema: PACKAGE_OBSERVATION_SCHEMA.to_string(),
            packages,
        }
    }
}

impl SystemPackageObserver {
    fn guess_version(&self, name: &str) -> Option<String> {
        let flag = if name == "tmux" { "-V" } else { "--version" };
        let output = Command::new(name).arg(flag).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        text.lines().next().map(|s| s.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_sh_present() {
        let observer = SystemPackageObserver::new();
        let obs = observer.observe_package("sh");
        assert!(obs.present);
        assert!(obs.path.is_some());
    }

    #[test]
    fn observe_missing_package() {
        let observer = SystemPackageObserver::new();
        let obs = observer.observe_package("definitely_not_a_real_tool_xyz123");
        assert!(!obs.present);
        assert!(obs.error.is_some());
    }

    #[test]
    fn sandbox_dependencies_checks_multiple() {
        let observer = SystemPackageObserver::new();
        let report = observer.observe_sandbox_dependencies();
        assert_eq!(report.packages.len(), 4);
    }
}
