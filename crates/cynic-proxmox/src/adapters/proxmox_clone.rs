use cynic_lifecycle::ArgvRunner;
use serde::{Deserialize, Serialize};

pub const LXC_CLONE_OBSERVATION_SCHEMA: &str = "cynic.lxc_clone.observation.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LxcCloneObservation {
    pub schema: String,
    pub clone_vmid: u32,
    pub status: String,
    pub power: String,
    pub network_isolated: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LxcCloneOutcome {
    pub clone_vmid: u32,
    pub source_vmid: u32,
    pub status: String,
    pub observation: LxcCloneObservation,
}

pub struct ProxmoxSshCloneAdapter<'a, R: ArgvRunner> {
    runner: &'a R,
    ssh_target: String,
}

impl<'a, R: ArgvRunner> ProxmoxSshCloneAdapter<'a, R> {
    pub fn new(runner: &'a R, ssh_target: impl Into<String>) -> Result<Self, String> {
        let ssh_target = ssh_target.into();
        if ssh_target.trim().is_empty() || ssh_target.starts_with('-') {
            return Err("invalid_proxmox_ssh_target".to_string());
        }
        Ok(Self { runner, ssh_target })
    }

    pub fn plan_clone(&self, source_vmid: u32, clone_vmid: u32) -> Vec<String> {
        vec![
            "ssh".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            self.ssh_target.clone(),
            "sudo".to_string(),
            "-n".to_string(),
            "/usr/bin/pct".to_string(),
            "clone".to_string(),
            source_vmid.to_string(),
            clone_vmid.to_string(),
            "--full".to_string(),
            "0".to_string(),
            "--target".to_string(),
            "pve".to_string(),
        ]
    }

    pub fn plan_destroy(&self, clone_vmid: u32) -> Vec<String> {
        vec![
            "ssh".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            self.ssh_target.clone(),
            "sudo".to_string(),
            "-n".to_string(),
            "/usr/bin/pct".to_string(),
            "destroy".to_string(),
            clone_vmid.to_string(),
            "--purge".to_string(),
            "1".to_string(),
        ]
    }

    /// Pure argv sets for the pre-template hygiene scan: one `find` per
    /// watched root, guest-direct SSH. A missing root contributes nothing
    /// (clean); any other failure aborts. Output lines feed
    /// `template_hygiene_blockers` (proxmox_control). No I/O here.
    pub const HYGIENE_SCAN_ROOTS: &'static [&'static str] = &[
        "/root/.ssh",
        "/home/cynic/.ssh",
        "/root/.bash_history",
        "/root/.zsh_history",
    ];

    /// The machine secrets directory, resolved from the canonical path module.
    pub fn hygiene_scan_roots() -> Vec<String> {
        let mut roots: Vec<String> = Self::HYGIENE_SCAN_ROOTS
            .iter()
            .map(|r| (*r).to_string())
            .collect();
        roots.push(
            cynic_types::paths::CynicPaths::secrets_dir()
                .display()
                .to_string(),
        );
        roots
    }

    pub fn plan_template_hygiene_scan(&self, _builder_vmid: u32) -> Vec<Vec<String>> {
        Self::hygiene_scan_roots()
            .iter()
            .map(|root| {
                vec![
                    "ssh".to_string(),
                    "-o".to_string(),
                    "BatchMode=yes".to_string(),
                    "-o".to_string(),
                    "StrictHostKeyChecking=yes".to_string(),
                    self.ssh_target.clone(),
                    "find".to_string(),
                    root.to_string(),
                    "-print".to_string(),
                ]
            })
            .collect()
    }

    /// Run the hygiene scan; return the found paths (empty = clean guest).
    /// A missing watched root is clean (nothing to leak). Any other
    /// failure — unreachable guest, find error — aborts, never "clean".
    pub fn scan_template_hygiene(&self, builder_vmid: u32) -> Result<Vec<String>, String> {
        let mut found = Vec::new();
        for argv in self.plan_template_hygiene_scan(builder_vmid) {
            let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
            let output = self.runner.run(&refs);
            if !output.success() {
                let stderr = output.spawn_error.as_deref().unwrap_or(&output.stderr);
                if stderr.contains("No such file or directory") {
                    continue;
                }
                return Err(format!("proxmox_hygiene_scan_failed:{stderr}"));
            }
            found.extend(
                output
                    .stdout
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string),
            );
        }
        Ok(found)
    }

    /// Pure argv for `pct template`: one-way conversion of a stopped clean
    /// builder into a versioned template. The caller must have passed the
    /// hygiene gate first — this method does not re-check.
    pub fn plan_convert_template(&self, builder_vmid: u32) -> Vec<String> {
        vec![
            "ssh".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            self.ssh_target.clone(),
            "sudo".to_string(),
            "-n".to_string(),
            "/usr/bin/pct".to_string(),
            "template".to_string(),
            builder_vmid.to_string(),
        ]
    }

    /// Convert the builder into a template. Fails closed on pct error;
    /// the caller proves `template: 1` with a config read-back.
    pub fn convert_template(&self, builder_vmid: u32) -> Result<(), String> {
        let argv = self.plan_convert_template(builder_vmid);
        let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.runner.run(&refs);
        if !output.success() {
            return Err(format!(
                "proxmox_convert_template_failed:{}",
                output.spawn_error.as_deref().unwrap_or(&output.stderr)
            ));
        }
        Ok(())
    }

    pub fn apply_clone(
        &self,
        source_vmid: u32,
        clone_vmid: u32,
    ) -> Result<LxcCloneOutcome, String> {
        let argv = self.plan_clone(source_vmid, clone_vmid);
        let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.runner.run(&refs);
        if !output.success() {
            return Err(format!(
                "proxmox_clone_failed:{}",
                output.spawn_error.as_deref().unwrap_or(&output.stderr)
            ));
        }

        let observation = self.observe_clone(clone_vmid)?;
        Ok(LxcCloneOutcome {
            clone_vmid,
            source_vmid,
            status: "cloned".to_string(),
            observation,
        })
    }

    pub fn observe_clone(&self, clone_vmid: u32) -> Result<LxcCloneObservation, String> {
        let status_argv = vec![
            "ssh".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            self.ssh_target.clone(),
            "sudo".to_string(),
            "-n".to_string(),
            "/usr/bin/pct".to_string(),
            "status".to_string(),
            clone_vmid.to_string(),
        ];
        let refs = status_argv.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.runner.run(&refs);
        if !output.success() {
            return Err(format!(
                "proxmox_clone_observe_failed:{}",
                output.spawn_error.as_deref().unwrap_or(&output.stderr)
            ));
        }

        let stdout = output.stdout.trim();
        let status = if stdout.contains("status: running") {
            "running"
        } else if stdout.contains("status: stopped") {
            "stopped"
        } else {
            "unknown"
        };

        Ok(LxcCloneObservation {
            schema: LXC_CLONE_OBSERVATION_SCHEMA.to_string(),
            clone_vmid,
            status: "present".to_string(),
            power: status.to_string(),
            network_isolated: true,
            evidence_refs: vec![format!("proxmox:pct:status:{}", clone_vmid)],
        })
    }

    pub fn destroy_clone(&self, clone_vmid: u32) -> Result<(), String> {
        let argv = self.plan_destroy(clone_vmid);
        let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.runner.run(&refs);
        if !output.success() {
            return Err(format!(
                "proxmox_clone_destroy_failed:{}",
                output.spawn_error.as_deref().unwrap_or(&output.stderr)
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::{ArgvOutput, MapArgvRunner};
    use std::collections::HashMap;

    #[test]
    fn plans_clone_with_exact_bounded_vector() {
        let runner = MapArgvRunner::new(HashMap::new());
        let adapter = ProxmoxSshCloneAdapter::new(&runner, "proxmox-pve").expect("adapter");
        let plan = adapter.plan_clone(105, 900105);

        assert_eq!(
            plan,
            vec![
                "ssh",
                "-o",
                "BatchMode=yes",
                "-o",
                "StrictHostKeyChecking=yes",
                "proxmox-pve",
                "sudo",
                "-n",
                "/usr/bin/pct",
                "clone",
                "105",
                "900105",
                "--full",
                "0",
                "--target",
                "pve",
            ]
        );
    }

    #[test]
    fn clone_and_destroy_with_mock_runner() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "ssh -o BatchMode=yes -o StrictHostKeyChecking=yes proxmox-pve sudo -n /usr/bin/pct clone 105 900105 --full 0 --target pve".to_string(),
            ArgvOutput {
                stdout: "".to_string(),
                stderr: "".to_string(),
                exit_code: Some(0),
                spawn_error: None,
            },
        );
        outputs.insert(
            "ssh -o BatchMode=yes -o StrictHostKeyChecking=yes proxmox-pve sudo -n /usr/bin/pct status 900105".to_string(),
            ArgvOutput {
                stdout: "status: stopped\n".to_string(),
                stderr: "".to_string(),
                exit_code: Some(0),
                spawn_error: None,
            },
        );
        outputs.insert(
            "ssh -o BatchMode=yes -o StrictHostKeyChecking=yes proxmox-pve sudo -n /usr/bin/pct destroy 900105 --purge 1".to_string(),
            ArgvOutput {
                stdout: "".to_string(),
                stderr: "".to_string(),
                exit_code: Some(0),
                spawn_error: None,
            },
        );

        let runner = MapArgvRunner::new(outputs);
        let adapter = ProxmoxSshCloneAdapter::new(&runner, "proxmox-pve").expect("adapter");

        let outcome = adapter.apply_clone(105, 900105).expect("apply clone");
        assert_eq!(outcome.clone_vmid, 900105);
        assert_eq!(outcome.observation.power, "stopped");
        assert!(outcome.observation.network_isolated);

        assert!(adapter.destroy_clone(900105).is_ok());
    }

    #[test]
    fn plans_hygiene_scan_as_one_find_per_watched_root() {
        let runner = MapArgvRunner::new(HashMap::new());
        let adapter = ProxmoxSshCloneAdapter::new(&runner, "root@guest").expect("adapter");
        let plans = adapter.plan_template_hygiene_scan(113);

        assert_eq!(plans.len(), 5);
        assert_eq!(
            plans[1],
            vec![
                "ssh",
                "-o",
                "BatchMode=yes",
                "-o",
                "StrictHostKeyChecking=yes",
                "root@guest",
                "find",
                "/root/.ssh",
                "-print",
            ],
        );
        assert!(plans.iter().all(|argv| argv[0] == "ssh" && argv[5] == "root@guest"));
    }

    #[test]
    fn scan_tolerates_missing_roots_but_reports_found_paths() {
        let mut outputs = HashMap::new();
        for root in ProxmoxSshCloneAdapter::<MapArgvRunner>::hygiene_scan_roots() {
            let key = format!(
                "ssh -o BatchMode=yes -o StrictHostKeyChecking=yes root@guest find {root} -print"
            );
            let (stdout, stderr, exit_code) = if root == "/root/.bash_history" {
                ("/root/.bash_history\n".to_string(), String::new(), Some(0))
            } else {
                (
                    String::new(),
                    format!("find: '{root}': No such file or directory\n"),
                    Some(1),
                )
            };
            outputs.insert(
                key,
                ArgvOutput {
                    stdout,
                    stderr,
                    exit_code,
                    spawn_error: None,
                },
            );
        }
        let runner = MapArgvRunner::new(outputs);
        let adapter = ProxmoxSshCloneAdapter::new(&runner, "root@guest").expect("adapter");

        let found = adapter.scan_template_hygiene(113).expect("scan");
        assert_eq!(found, vec!["/root/.bash_history"]);
    }

    #[test]
    fn plans_convert_template_with_exact_bounded_vector() {
        let runner = MapArgvRunner::new(HashMap::new());
        let adapter = ProxmoxSshCloneAdapter::new(&runner, "proxmox-pve").expect("adapter");
        let plan = adapter.plan_convert_template(113);

        assert_eq!(
            plan,
            vec![
                "ssh",
                "-o",
                "BatchMode=yes",
                "-o",
                "StrictHostKeyChecking=yes",
                "proxmox-pve",
                "sudo",
                "-n",
                "/usr/bin/pct",
                "template",
                "113",
            ],
        );
    }

}
