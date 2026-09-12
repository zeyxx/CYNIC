//! Bubblewrap (bwrap) sandbox adapter.
//!
//! Rootless container via Linux namespaces (user, mount, pid, net, ipc, uts).
//! No privileges required. Shares the host kernel.

use cynic_protocol::{
    AgentSandboxAllocation, SandboxNetworkPhase, validate_agent_sandbox_allocation,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const BWRAP_SANDBOX_SCHEMA: &str = "cynic.sandbox.bwrap.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BwrapConfig {
    pub schema: String,
    /// UDS socket path to expose inside the sandbox.
    pub uds_path: PathBuf,
    /// Read-only bind mounts (host_path -> sandbox_path).
    pub ro_mounts: Vec<(PathBuf, PathBuf)>,
    /// Ephemeral tmpfs workspace path.
    pub workspace_path: PathBuf,
    /// Workspace tmpfs size limit in bytes.
    pub workspace_limit_bytes: u64,
    /// Network isolation mode.
    pub network: BwrapNetwork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BwrapNetwork {
    /// Loopback only (default-deny egress).
    Isolated,
    /// Route via host network namespace (for proxy egress).
    Proxy,
}

impl BwrapConfig {
    pub fn ephemeral_local(uds_path: &Path, workspace: &Path) -> Self {
        Self {
            schema: BWRAP_SANDBOX_SCHEMA.to_string(),
            uds_path: uds_path.to_path_buf(),
            ro_mounts: vec![
                ("/usr".into(), "/usr".into()),
                ("/bin".into(), "/bin".into()),
                ("/lib".into(), "/lib".into()),
                ("/lib64".into(), "/lib64".into()),
            ],
            workspace_path: workspace.to_path_buf(),
            workspace_limit_bytes: 0,
            network: BwrapNetwork::Isolated,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BwrapReport {
    pub schema: String,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait BwrapPort {
    /// Launch a sandboxed process with the given config.
    fn run(&self, config: &BwrapConfig, argv: &[&str]) -> Result<BwrapReport, String>;
}

/// Builds the exact bubblewrap argv from a typed AgentSandboxAllocation contract.
pub fn build_bwrap_argv(
    allocation: &AgentSandboxAllocation,
    command: &[&str],
) -> Result<Vec<String>, String> {
    let violations = validate_agent_sandbox_allocation(allocation);
    if !violations.is_empty() {
        return Err(format!(
            "invalid_sandbox_allocation: {}",
            violations.join(", ")
        ));
    }

    let mut argv = vec![
        "bwrap".to_string(),
        "--die-with-parent".to_string(),
        "--unshare-pid".to_string(),
        "--unshare-uts".to_string(),
        "--unshare-ipc".to_string(),
    ];

    // Network isolation: default-deny (unshare-net) unless in bootstrap phase
    match allocation.network_phase {
        SandboxNetworkPhase::BootstrapForgejo => {
            argv.push("--share-net".to_string());
        }
        SandboxNetworkPhase::ZeroEgressAgent | SandboxNetworkPhase::ProxyInferenceUds => {
            argv.push("--unshare-net".to_string());
        }
    }

    // System base read-only mounts
    for dir in &[
        "/usr",
        "/lib",
        "/lib64",
        "/bin",
        "/sbin",
        "/etc/resolv.conf",
        "/etc/ssl",
    ] {
        if Path::new(dir).exists() {
            argv.push("--ro-bind".to_string());
            argv.push(dir.to_string());
            argv.push(dir.to_string());
        }
    }

    // Mounts declared in allocation
    for mount in &allocation.mounts {
        if mount.read_only {
            argv.push("--ro-bind".to_string());
        } else {
            argv.push("--bind".to_string());
        }
        argv.push(mount.source.clone());
        argv.push(mount.target.clone());
    }

    // Read-only bind of the private mirror git-dir to /repo.git
    argv.push("--ro-bind".to_string());
    argv.push(allocation.private_git_dir.clone());
    argv.push("/repo.git".to_string());

    // Read-write bind of the dedicated workspace to /workspace
    argv.push("--bind".to_string());
    argv.push(allocation.private_workspace_path.clone());
    argv.push("/workspace".to_string());

    // Private /tmp, /proc, /dev
    argv.push("--tmpfs".to_string());
    argv.push("/tmp".to_string());
    argv.push("--proc".to_string());
    argv.push("/proc".to_string());
    argv.push("--dev".to_string());
    argv.push("/dev".to_string());

    // CWD inside sandbox
    argv.push("--chdir".to_string());
    argv.push("/workspace".to_string());

    // Safe sanitized environment
    argv.push("--setenv".to_string());
    argv.push("HOME".to_string());
    argv.push("/tmp".to_string());
    argv.push("--setenv".to_string());
    argv.push("GIT_DIR".to_string());
    argv.push("/repo.git".to_string());
    argv.push("--setenv".to_string());
    argv.push("GIT_WORK_TREE".to_string());
    argv.push("/workspace".to_string());

    // Target command
    for arg in command {
        argv.push(arg.to_string());
    }

    Ok(argv)
}

/// Concrete local Bubblewrap adapter executing bwrap on the local machine or inside the guest VM.
pub struct LocalBwrapAdapter;

impl BwrapPort for LocalBwrapAdapter {
    fn run(&self, config: &BwrapConfig, argv: &[&str]) -> Result<BwrapReport, String> {
        let mut cmd = Command::new("bwrap");
        cmd.arg("--die-with-parent")
            .arg("--unshare-pid")
            .arg("--unshare-uts")
            .arg("--unshare-ipc");

        match config.network {
            BwrapNetwork::Isolated => {
                cmd.arg("--unshare-net");
            }
            BwrapNetwork::Proxy => {
                cmd.arg("--share-net");
            }
        }

        for (host, sandbox) in &config.ro_mounts {
            if host.exists() {
                cmd.arg("--ro-bind").arg(host).arg(sandbox);
            }
        }

        if config.workspace_path.exists() {
            cmd.arg("--bind")
                .arg(&config.workspace_path)
                .arg(&config.workspace_path);
        }

        cmd.arg("--tmpfs")
            .arg("/tmp")
            .arg("--proc")
            .arg("/proc")
            .arg("--dev")
            .arg("/dev");

        cmd.args(argv);

        let output = cmd
            .output()
            .map_err(|e| format!("failed to spawn bwrap: {e}"))?;

        Ok(BwrapReport {
            schema: BWRAP_SANDBOX_SCHEMA.to_string(),
            pid: None,
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

pub fn null_bwrap_port() -> NullBwrapPort {
    NullBwrapPort
}

pub struct NullBwrapPort;

impl BwrapPort for NullBwrapPort {
    fn run(&self, _config: &BwrapConfig, _argv: &[&str]) -> Result<BwrapReport, String> {
        Err("bwrap adapter not implemented for this platform".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{SandboxMountSpec, SandboxStrategy};

    fn make_allocation(id: &str, branch: &str) -> AgentSandboxAllocation {
        AgentSandboxAllocation {
            schema: cynic_protocol::AGENT_SANDBOX_ALLOCATION_SCHEMA.to_string(),
            allocation_id: format!("alloc-{id}"),
            workload_id: format!("workload-{id}"),
            agent_id: format!("agent-{id}"),
            host_vm_id: "cynic-agent-host".to_string(),
            strategy: SandboxStrategy::BwrapEphemeral,
            isolation_digest: format!("sha256:{}", "b".repeat(64)),
            ttl_seconds: 3600,
            max_actions: 50,
            max_memory_bytes: 2 * 1024 * 1024 * 1024,
            max_disk_bytes: 10 * 1024 * 1024 * 1024,
            private_git_dir: format!("/var/lib/cynic/sandboxes/workload-{id}/repo.git"),
            private_workspace_path: format!("/var/lib/cynic/sandboxes/workload-{id}/workspace"),
            repository_id: "cynic".to_string(),
            forgejo_base_sha: "0123456789abcdef0123456789abcdef01234567".to_string(),
            target_branch: branch.to_string(),
            mounts: vec![SandboxMountSpec {
                source: format!("/var/lib/cynic/sandboxes/workload-{id}/workspace"),
                target: "/workspace".to_string(),
                read_only: false,
            }],
            network_phase: SandboxNetworkPhase::ZeroEgressAgent,
            allocated_at_unix_ms: 1788267750000,
        }
    }

    #[test]
    fn bwrap_config_ephemeral() {
        let c = BwrapConfig::ephemeral_local("/tmp/test.sock".as_ref(), "/tmp/workspace".as_ref());
        assert_eq!(c.network, BwrapNetwork::Isolated);
        assert!(!c.ro_mounts.is_empty());
    }

    #[test]
    fn build_bwrap_argv_unshares_network_in_agent_phase() {
        let alloc = make_allocation("1", "agent/a/feature-1");
        let argv = build_bwrap_argv(&alloc, &["git", "status"]).unwrap();

        assert_eq!(argv[0], "bwrap");
        assert!(argv.contains(&"--unshare-net".to_string()));
        assert!(argv.contains(&"--unshare-pid".to_string()));
        assert!(argv.contains(&"/var/lib/cynic/sandboxes/workload-1/workspace".to_string()));
        assert!(argv.contains(&"/var/lib/cynic/sandboxes/workload-1/repo.git".to_string()));
        assert_eq!(argv.last().unwrap(), "status");
    }

    #[test]
    fn build_bwrap_argv_shares_net_only_in_bootstrap() {
        let mut alloc = make_allocation("1", "agent/a/feature-1");
        alloc.network_phase = SandboxNetworkPhase::BootstrapForgejo;
        let argv = build_bwrap_argv(&alloc, &["git", "fetch"]).unwrap();

        assert!(argv.contains(&"--share-net".to_string()));
        assert!(!argv.contains(&"--unshare-net".to_string()));
    }

    #[test]
    fn concurrent_allocations_have_strictly_independent_mounts() {
        let alloc_a = make_allocation("alpha", "agent/alpha/slice-a");
        let alloc_b = make_allocation("beta", "agent/beta/slice-b");

        let argv_a =
            build_bwrap_argv(&alloc_a, &["git", "checkout", "-b", "agent/alpha/slice-a"]).unwrap();
        let argv_b =
            build_bwrap_argv(&alloc_b, &["git", "checkout", "-b", "agent/beta/slice-b"]).unwrap();

        // Ensure workspace paths are completely distinct
        assert_ne!(
            alloc_a.private_workspace_path,
            alloc_b.private_workspace_path
        );
        assert_ne!(alloc_a.private_git_dir, alloc_b.private_git_dir);

        // Ensure argv_a contains only alpha paths, not beta paths
        assert!(argv_a.iter().any(|arg| arg.contains("workload-alpha")));
        assert!(!argv_a.iter().any(|arg| arg.contains("workload-beta")));

        // Ensure argv_b contains only beta paths, not alpha paths
        assert!(argv_b.iter().any(|arg| arg.contains("workload-beta")));
        assert!(!argv_b.iter().any(|arg| arg.contains("workload-alpha")));
    }
}
