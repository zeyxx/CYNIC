//! Bubblewrap (bwrap) sandbox adapter.
//!
//! Rootless container via Linux namespaces (user, mount, pid, net, ipc, uts).
//! No privileges required. Shares the host kernel.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

    #[test]
    fn bwrap_config_ephemeral() {
        let c = BwrapConfig::ephemeral_local("/tmp/test.sock".as_ref(), "/tmp/workspace".as_ref());
        assert_eq!(c.network, BwrapNetwork::Isolated);
        assert!(!c.ro_mounts.is_empty());
    }
}
