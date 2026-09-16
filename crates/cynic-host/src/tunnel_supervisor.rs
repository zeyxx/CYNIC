//! Active supervision of the kernel SSH tunnel.
//!
//! Runs the reconcile loop at a bounded interval, producing periodic
//! read-back proofs. Fail-closed: if reconcile fails N times, surface
//! blocks with explicit error.
//!
//! CYNIC coherence pattern maintained: desired state → observe →
//! reconcile → bounded action → read-back proof (every interval).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::kernel_tunnel::{KernelTunnelConfig, KernelTunnelManager, TunnelHealth, TunnelProof};

/// Active supervisor — reconciles the tunnel at a fixed interval.
pub struct TunnelSupervisor {
    manager: KernelTunnelManager,
    interval: Duration,
    max_failures: u32,
    running: Arc<AtomicBool>,
}

impl TunnelSupervisor {
    pub fn new(
        config: KernelTunnelConfig,
        runner: std::sync::Arc<dyn cynic_lifecycle::ArgvRunner + Send + Sync>,
        interval_secs: u64,
        max_failures: u32,
    ) -> Self {
        Self {
            manager: KernelTunnelManager::new(config, runner),
            interval: Duration::from_secs(interval_secs),
            max_failures,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Blocking reconcile loop. Emits TunnelProof each tick.
    pub fn run(&self) -> ! {
        let mut failures = 0u32;
        while self.running.load(Ordering::SeqCst) {
            let proof: TunnelProof = self.manager.reconcile();

            if proof.healthy {
                failures = 0;
                println!(
                    "[cynic kernel] tunnel healthy — proof: socket={} reconnects={}",
                    proof.socket_readable, proof.reconnect_count
                );
            } else {
                failures += 1;
                println!(
                    "[cynic kernel] tunnel FAIL ({} / {}) — error: {}",
                    failures,
                    self.max_failures,
                    proof.error.as_deref().unwrap_or("unknown")
                );

                if failures >= self.max_failures {
                    // Fail-closed: stop the loop, surface is blocked.
                    eprintln!(
                        "[cynic kernel] tunnel supervisor giving up after {} consecutive failures",
                        failures
                    );
                    std::process::exit(1);
                }
            }

            thread::sleep(self.interval);
        }

        std::process::exit(0);
    }

    pub fn observe(&self) -> TunnelHealth {
        self.manager.observe()
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::MapArgvRunner;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn fake_config() -> KernelTunnelConfig {
        KernelTunnelConfig {
            ssh_target: "root@10.10.10.103".to_string(),
            surrealdb_addr: "10.10.10.103:8000".to_string(),
            local_socket: PathBuf::from("/tmp/test-tunnel.sock"),
            ssh_key: PathBuf::from("/tmp/test-key"),
            keepalive_secs: 1,
            max_reconnects: 3,
        }
    }

    #[test]
    fn supervisor_observes_initial_unhealthy() {
        let runner: Arc<dyn cynic_lifecycle::ArgvRunner + Send + Sync> =
            Arc::new(MapArgvRunner { responses: vec![] });
        let sup = TunnelSupervisor::new(fake_config(), runner, 1, 3);
        let health = sup.observe();
        assert!(!health.healthy);
    }

    #[test]
    fn supervisor_stops_loop() {
        let runner: Arc<dyn cynic_lifecycle::ArgvRunner + Send + Sync> =
            Arc::new(MapArgvRunner { responses: vec![] });
        let sup = TunnelSupervisor::new(fake_config(), runner, 1, 3);
        sup.stop();
        // verify running flag flipped
        assert!(!sup.running.load(Ordering::SeqCst));
    }
}
