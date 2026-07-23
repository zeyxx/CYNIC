use std::collections::BTreeMap;
use std::path::Path;

use crate::command_surface::{
    HostCommandSurface, install_dir_command, install_file_command, validate_sudoers_command,
};
use crate::install_contract::{MODE_0440, MODE_0755, mode_map, root_owned_path};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedHelperProjection {
    pub subject: String,
    pub helper_path: String,
    pub sudoers_path: String,
    pub description: String,
    #[serde(default)]
    pub allowed_invocations: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedHelperInstallRuntime {
    pub helper_target: String,
    pub sudoers_target: String,
    pub helper_dir: String,
    pub sudoers_dir: String,
}

impl BoundedHelperProjection {
    pub fn new(
        subject: impl Into<String>,
        helper_path: impl Into<String>,
        sudoers_path: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            subject: subject.into(),
            helper_path: helper_path.into(),
            sudoers_path: sudoers_path.into(),
            description: description.into(),
            allowed_invocations: Vec::new(),
        }
    }

    pub fn sudoers_rule(&self) -> String {
        if self.allowed_invocations.is_empty() {
            return String::new();
        }
        self.allowed_invocations
            .iter()
            .map(|argv| format!("{} ALL=(root) NOPASSWD: {}", self.subject, argv.join(" ")))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn sudoers_projection(&self) -> String {
        format!(
            "# Generated from cynic bounded helper policy\n# {}\n{}\n",
            self.description,
            self.sudoers_rule()
        )
    }

    pub fn with_allowed_invocations(mut self, allowed_invocations: Vec<Vec<String>>) -> Self {
        self.allowed_invocations = allowed_invocations;
        self
    }
}

impl BoundedHelperInstallRuntime {
    pub fn prepare_directories(&self) -> Vec<HostCommandSurface> {
        vec![
            install_dir_command(
                "prepare_directory",
                Path::new(&self.helper_target)
                    .parent()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| self.helper_dir.clone()),
                root_owned_path(MODE_0755),
            ),
            install_dir_command(
                "prepare_directory",
                Path::new(&self.sudoers_target)
                    .parent()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| self.sudoers_dir.clone()),
                root_owned_path(MODE_0755),
            ),
        ]
    }

    pub fn required_modes(&self) -> BTreeMap<&'static str, String> {
        mode_map(&[("helper", MODE_0755), ("sudoers", MODE_0440)])
    }

    pub fn install_helper_command(
        &self,
        bundle_dir: &str,
        helper_filename: &str,
    ) -> HostCommandSurface {
        install_file_command(
            "install_helper",
            format!("{bundle_dir}/{helper_filename}"),
            self.helper_target.clone(),
            root_owned_path(MODE_0755),
        )
    }

    pub fn install_sudoers_command(
        &self,
        bundle_dir: &str,
        sudoers_filename: &str,
    ) -> HostCommandSurface {
        install_file_command(
            "install_sudoers",
            format!("{bundle_dir}/{sudoers_filename}"),
            self.sudoers_target.clone(),
            root_owned_path(MODE_0440),
        )
    }

    pub fn validate_sudoers_command(&self) -> HostCommandSurface {
        validate_sudoers_command("validate_sudoers", self.sudoers_target.clone())
    }
}

pub fn bundle_complete(path: impl AsRef<Path>, required_files: &[&str]) -> bool {
    let path = path.as_ref();
    path.exists() && required_files.iter().all(|name| path.join(name).exists())
}

#[cfg(test)]
mod tests {
    use super::{BoundedHelperInstallRuntime, BoundedHelperProjection};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn renders_rule_and_projection() {
        let projection = BoundedHelperProjection {
            subject: "cynic".to_string(),
            helper_path: "/usr/local/bin/helper".to_string(),
            sudoers_path: "/etc/sudoers.d/helper".to_string(),
            description: "Allows bounded helper invocation only.".to_string(),
            allowed_invocations: Vec::new(),
        };

        assert!(projection.sudoers_rule().is_empty());
        assert!(!projection.sudoers_projection().contains("NOPASSWD"));
        assert!(
            projection
                .sudoers_projection()
                .contains("Allows bounded helper invocation only.")
        );
    }

    #[test]
    fn constructor_populates_projection() {
        let projection = BoundedHelperProjection::new(
            "cynic",
            "/usr/local/bin/helper",
            "/etc/sudoers.d/helper",
            "Allows bounded helper invocation only.",
        );

        assert_eq!(projection.subject, "cynic");
        assert_eq!(projection.helper_path, "/usr/local/bin/helper");
        assert_eq!(projection.sudoers_path, "/etc/sudoers.d/helper");
    }

    #[test]
    fn renders_bounded_invocation_rules() {
        let projection = BoundedHelperProjection::new(
            "cynic",
            "/usr/local/bin/helper",
            "/etc/sudoers.d/helper",
            "Allows bounded helper invocation only.",
        )
        .with_allowed_invocations(vec![
            vec![
                "/usr/local/bin/helper".to_string(),
                "principal-prepare".to_string(),
            ],
            vec![
                "/usr/local/bin/helper".to_string(),
                "network-baseline".to_string(),
                "observe".to_string(),
            ],
        ]);

        assert_eq!(
            projection.sudoers_rule(),
            "cynic ALL=(root) NOPASSWD: /usr/local/bin/helper principal-prepare\ncynic ALL=(root) NOPASSWD: /usr/local/bin/helper network-baseline observe"
        );
    }

    #[test]
    fn builds_install_commands_from_runtime() {
        let runtime = BoundedHelperInstallRuntime {
            helper_target: "/usr/local/bin/helper".to_string(),
            sudoers_target: "/etc/sudoers.d/helper".to_string(),
            helper_dir: "/usr/local/bin".to_string(),
            sudoers_dir: "/etc/sudoers.d".to_string(),
        };

        assert_eq!(
            runtime.required_modes().get("helper"),
            Some(&"0755".to_string())
        );
        assert_eq!(runtime.prepare_directories().len(), 2);
        assert_eq!(
            crate::command_surface::render_command(
                &runtime.install_helper_command("/tmp/bundle", "helper")
            ),
            "install -o root -g root -m 0755 /tmp/bundle/helper /usr/local/bin/helper"
        );
    }

    #[test]
    fn detects_complete_bundle_directory() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tempdir = std::env::temp_dir().join(format!("cynic-bounded-helper-{unique}"));
        fs::create_dir_all(&tempdir).unwrap();
        fs::write(tempdir.join("manifest.json"), "").unwrap();
        fs::write(tempdir.join("helper"), "").unwrap();

        assert!(super::bundle_complete(
            &tempdir,
            &["manifest.json", "helper"]
        ));
        assert!(!super::bundle_complete(
            &tempdir,
            &["manifest.json", "helper", "sudoers"]
        ));
        let _ = fs::remove_dir_all(&tempdir);
    }
}
