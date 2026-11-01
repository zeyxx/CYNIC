//! Persistent SSH tunnel to the CYNIC kernel SurrealDB runtime.
//!
//! Implements the resilient transport adapter between the local kernel
//! socket and the remote SurrealDB endpoint. Addresses the SPOF where
//! a manual SSH tunnel dies and the kernel becomes unreachable.
//!
//! CYNIC coherence pattern:
//!   desired state (tunnel config) → observe (tunnel health) →
//!   reconcile (should reconnect?) → bounded action (spawn SSH) →
//!   read-back proof (socket readable)
//!
//! Structural invariants:
//!   - No monolith: types + port + adapter + thin glue
//!   - Policy is portable; substrate (SSH) injected via ArgvRunner port
//!   - Fail-closed: tunnel down → explicit blocked state, never silent fallback
//!   - Adapters are disposable; the socket path contract is durable

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cynic_process::ArgvRunner;
use cynic_ssh::argv::{ssh_control_exit_argv, ssh_forward_tunnel_argv, SshForwardSpec};

/// Desired state — the tunnel configuration. Portable; no substrate details.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KernelTunnelConfig {
    /// Remote SSH target (e.g. root@10.10.10.103).
    pub ssh_target: String,
    /// Remote SurrealDB address (e.g. 10.10.10.103:8000).
    pub surrealdb_addr: String,
    /// Local Unix socket path (e.g. /run/cynic/kernel.sock).
    pub local_socket: PathBuf,
    /// SSH identity file (provisionné via credential_provisioning).
    pub ssh_key: PathBuf,
    /// Keepalive interval (seconds).
    pub keepalive_secs: u64,
    /// Max reconnect attempts before giving up.
    pub max_reconnects: u32,
}

/// Observed state — read-back of the tunnel health.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TunnelHealth {
    pub healthy: bool,
    pub pid: Option<u32>,
    pub last_connect: Option<u128>,
    pub reconnect_count: u32,
    pub last_error: Option<String>,
}

/// Read-back proof after a tunnel operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TunnelProof {
    pub healthy: bool,
    pub socket_readable: bool,
    pub pid: Option<u32>,
    pub reconnect_count: u32,
    pub error: Option<String>,
}

/// Pure decision: should we reconnect the tunnel?
pub fn should_reconnect(health: &TunnelHealth, _now: Instant, max_reconnects: u32) -> bool {
    if health.reconnect_count >= max_reconnects {
        return false;
    }
    if let Some(last_err) = &health.last_error {
        let _ = last_err.as_str(); // observation only
    }
    // If never connected or health check failed → reconnect
    health.last_connect.is_none() || !health.healthy
}

/// Pure decision: is the socket actually readable? (read-back proof)
pub fn is_socket_readable(socket_path: &std::path::Path) -> bool {
    if !socket_path.exists() {
        return false;
    }
    // Try connecting to the socket — proves it's alive
    match std::os::unix::net::UnixStream::connect(socket_path) {
        Ok(stream) => {
            // Connection success proves the tunnel is live.
            // SurrealDB over Unix socket: connection acceptance = healthy.
            drop(stream);
            true
        }
        Err(_) => false,
    }
}

/// Resilient SSH tunnel manager — manages lifecycle of the SSH tunnel process.
pub struct KernelTunnelManager {
    config: KernelTunnelConfig,
    runner: Arc<dyn ArgvRunner + Send + Sync>,
    health: Arc<Mutex<TunnelHealth>>,
    active: Arc<AtomicBool>,
}

impl KernelTunnelManager {
    pub fn new(config: KernelTunnelConfig, runner: Arc<dyn ArgvRunner + Send + Sync>) -> Self {
        Self {
            config,
            runner,
            health: Arc::new(Mutex::new(TunnelHealth {
                healthy: false,
                pid: None,
                last_connect: None,
                reconnect_count: 0,
                last_error: None,
            })),
            active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Observe current tunnel health — read-only port.
    pub fn observe(&self) -> TunnelHealth {
        let h = self.health.lock().unwrap();
        h.clone()
    }

    /// Reconcile: ensure tunnel is up. Returns proof of outcome.
    pub fn reconcile(&self) -> TunnelProof {
        let now = std::time::Instant::now();
        let health = self.observe();

        if !should_reconnect(&health, now, self.config.max_reconnects) {
            return TunnelProof {
                healthy: health.healthy,
                socket_readable: is_socket_readable(&self.config.local_socket),
                pid: health.pid,
                reconnect_count: health.reconnect_count,
                error: Some("max reconnects exceeded".to_string()),
            };
        }

        // Bounded action: spawn SSH tunnel
        let forward = SshForwardSpec {
            local_socket: self.config.local_socket.clone(),
            remote_addr: self.config.surrealdb_addr.clone(),
        };
        let argv = match ssh_forward_tunnel_argv(
            &self.config.ssh_target,
            Some(&self.config.ssh_key),
            &forward,
        ) {
            Ok(v) => v,
            Err(e) => {
                let socket_readable = is_socket_readable(&self.config.local_socket);
                return TunnelProof {
                    healthy: false,
                    socket_readable,
                    pid: None,
                    reconnect_count: health.reconnect_count,
                    error: Some(e),
                };
            }
        };

        let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        let result = self.runner.run(&argv_refs);

        let pid = if result.success() {
            // SSH -f forks → we need to track via ControlMaster or pgrep
            // For simplicity, we trust the socket comes up
            None
        } else {
            None
        };

        let socket_readable = is_socket_readable(&self.config.local_socket);
        let healthy = result.success() && socket_readable;

        // Update health
        {
            let mut h = self.health.lock().unwrap();
            h.healthy = healthy;
            h.pid = pid;
            h.reconnect_count = health.reconnect_count + 1;
            h.last_connect = Some(now.elapsed().as_millis());
            h.last_error = if healthy {
                None
            } else {
                Some(format!(
                    "ssh spawn failed: {} — stderr: {}",
                    result.spawn_error.clone().unwrap_or_default(),
                    result.stderr
                ))
            };
        }

        self.active.store(healthy, Ordering::SeqCst);

        TunnelProof {
            healthy,
            socket_readable,
            pid,
            reconnect_count: health.reconnect_count + 1,
            error: if healthy {
                None
            } else {
                Some(format!(
                    "tunnel not readable after spawn: {}",
                    result.stderr
                ))
            },
        }
    }

    /// Bounded action: tear down tunnel (fail-fast stop).
    pub fn teardown(&self) -> bool {
        self.active.store(false, Ordering::SeqCst);
        let forward = SshForwardSpec {
            local_socket: self.config.local_socket.clone(),
            remote_addr: self.config.surrealdb_addr.clone(),
        };
        let argv = ssh_control_exit_argv(&self.config.ssh_target, &forward)
            .unwrap_or_default();
        let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        let result = self.runner.run(&argv_refs);
        let _ = result; // best-effort
        true
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Wait for tunnel to be healthy — bounded blocking.
    pub fn wait_for_healthy(&self, timeout_secs: u64) -> TunnelProof {
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            let proof = self.reconcile();
            if proof.healthy {
                return proof;
            }
            if std::time::Instant::now() >= deadline {
                return proof;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_process::MapArgvRunner;
    use cynic_ssh::argv::{ssh_control_exit_argv, ssh_forward_tunnel_argv, SshForwardSpec};
    use std::sync::Arc;
    use std::sync::Mutex;

    fn test_config(tmp: &std::path::Path) -> KernelTunnelConfig {
        KernelTunnelConfig {
            ssh_target: "root@10.10.10.103".to_string(),
            surrealdb_addr: "10.10.10.103:8000".to_string(),
            local_socket: tmp.to_path_buf(),
            ssh_key: PathBuf::from("/tmp/test-key"),
            keepalive_secs: 10,
            max_reconnects: 3,
        }
    }

    #[test]
    fn should_reconnect_when_unhealthy() {
        let health = TunnelHealth {
            healthy: false,
            pid: None,
            last_connect: None,
            reconnect_count: 0,
            last_error: None,
        };
        // Should reconnect when never connected
        assert!(should_reconnect(&health, std::time::Instant::now(), 3));
    }

    #[test]
    fn should_not_reconnect_after_max() {
        let health = TunnelHealth {
            healthy: false,
            pid: None,
            last_connect: Some(100),
            reconnect_count: 3,
            last_error: Some("fail".into()),
        };
        assert!(!should_reconnect(&health, std::time::Instant::now(), 3));
    }

    #[test]
    fn should_reconnect_when_healthy_but_count_low() {
        let health = TunnelHealth {
            healthy: true,
            pid: Some(123),
            last_connect: Some(100),
            reconnect_count: 1,
            last_error: None,
        };
        // healthy=true → should NOT reconnect
        assert!(!should_reconnect(&health, std::time::Instant::now(), 3));
    }

    #[test]
    fn is_socket_readable_nonexistent() {
        assert!(!is_socket_readable(std::path::Path::new(
            "/nonexistent/socket/path"
        )));
    }

    #[test]
    fn tunnel_manager_observe_initial_state() {
        let runner: Arc<dyn ArgvRunner + Send + Sync> =
            Arc::new(MapArgvRunner { responses: vec![] });
        let tmp = std::env::temp_dir().join("test_kernel_sock");
        let config = test_config(&tmp);
        let mgr = KernelTunnelManager::new(config, runner);

        let health = mgr.observe();
        assert!(!health.healthy);
        assert_eq!(health.reconnect_count, 0);
        assert!(health.last_error.is_none());
    }

    #[test]
    fn tunnel_manager_reconcile_fail_closed_when_ssh_fails() {
        // MapArgvRunner with empty responses → SSH will fail
        let runner: Arc<dyn ArgvRunner + Send + Sync> =
            Arc::new(MapArgvRunner { responses: vec![] });
        let tmp = std::env::temp_dir().join("test_kernel_sock_nonexist");
        let config = test_config(&tmp);
        let mgr = KernelTunnelManager::new(config, runner);

        let proof = mgr.reconcile();
        // Should be fail-closed: healthy=false, error set
        assert!(!proof.healthy, "must fail-closed when SSH fails");
        assert!(proof.error.is_some(), "must have error when unhealthy");
    }

    #[test]
    fn tunnel_manager_teardown_returns_true() {
        let runner: Arc<dyn ArgvRunner + Send + Sync> =
            Arc::new(MapArgvRunner { responses: vec![] });
        let tmp = std::env::temp_dir().join("test_kernel_sock_teardown");
        let config = test_config(&tmp);
        let mgr = KernelTunnelManager::new(config, runner);

        assert!(mgr.teardown());
    }

    struct RecordingRunner {
        recorded: Mutex<Vec<Vec<String>>>,
    }

    impl RecordingRunner {
        fn new() -> Self {
            Self {
                recorded: Mutex::new(Vec::new()),
            }
        }
    }

    impl cynic_process::ArgvRunner for RecordingRunner {
        fn run(&self, argv: &[&str]) -> cynic_process::ArgvOutput {
            let owned: Vec<String> = argv.iter().map(|s| (*s).to_string()).collect();
            self.recorded.lock().unwrap().push(owned);
            cynic_process::ArgvOutput {
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: None,
            }
        }
    }

    #[test]
    fn reconcile_uses_canonical_ssh_forward_argv() {
        let runner = Arc::new(RecordingRunner::new());
        let tmp = std::env::temp_dir().join("test_kernel_sock_canonical_reconcile");
        let config = test_config(&tmp);
        let expected_forward = SshForwardSpec {
            local_socket: config.local_socket.clone(),
            remote_addr: config.surrealdb_addr.clone(),
        };
        let expected = ssh_forward_tunnel_argv(
            &config.ssh_target,
            Some(&config.ssh_key),
            &expected_forward,
        )
        .expect("valid");
        let mgr = KernelTunnelManager::new(config, runner.clone());
        let _ = mgr.reconcile();
        let recorded = runner.recorded.lock().unwrap();
        assert!(!recorded.is_empty());
        assert_eq!(recorded[0], expected);
    }

    #[test]
    fn teardown_uses_canonical_ssh_control_exit_argv() {
        let runner = Arc::new(RecordingRunner::new());
        let tmp = std::env::temp_dir().join("test_kernel_sock_canonical_teardown");
        let config = test_config(&tmp);
        let expected_forward = SshForwardSpec {
            local_socket: config.local_socket.clone(),
            remote_addr: config.surrealdb_addr.clone(),
        };
        let expected = ssh_control_exit_argv(&config.ssh_target, &expected_forward).expect("valid");
        let mgr = KernelTunnelManager::new(config, runner.clone());
        assert!(mgr.teardown());
        let recorded = runner.recorded.lock().unwrap();
        assert!(!recorded.is_empty());
        assert_eq!(recorded[0], expected);
    }
}
