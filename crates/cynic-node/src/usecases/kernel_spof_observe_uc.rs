//! Kernel SPOF observation (design D: kernel kept separate from the federated
//! peer set). The kernel is deployed centrally — operator machine plus ONE
//! dedicated VM — not per node. That dedicated VM is a genuine single point of
//! failure: if it dies, the federation loses its cortex (coordination + event
//! stream). This use case makes that residual risk explicit and tells the node
//! to degrade to local autonomy when the kernel is unreachable.
//!
//! It deliberately does NOT crash or block on kernel unavailability: per design
//! A (node autonomy), local build/observe must keep working without the kernel.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cynic_protocol::health_signal::KernelSpofSignal;
use cynic_protocol::peer_resolver::PeerResolver;

use super::federated_mesh_probe_uc::CANONICAL_SYNAPSE_NODES;
use super::federated_mesh_resolver::LocalRegistryMeshResolver;

pub const KERNEL_HEARTBEAT_TIMEOUT_MS: u64 = 800;

/// Observe the kernel SPOF from this node's perspective.
///
/// Returns a `KernelSpofSignal` (never panics). `kernel_reachable` reflects a
/// TCP reachability probe of the kernel's declared endpoint; if the probe is
/// impossible (no declared cortex node, or syscall error) we treat the kernel
/// as unreachable and record the residual risk — degraded autonomy is the safe
/// default, not a crash.
pub fn observe_kernel_spof() -> KernelSpofSignal {
    let resolver = LocalRegistryMeshResolver::from_repo_infra_default(CANONICAL_SYNAPSE_NODES);
    let primary_peers = resolver.resolve_peers("cortex_primary");
    let standby_peers = resolver.resolve_peers("cortex_standby");

    let mut kernel_peers = primary_peers;
    kernel_peers.extend(standby_peers);

    let (reachable, age_s) = match kernel_peers.into_iter().find(|peer| {
        let port = peer.probe_ports.first().copied().unwrap_or(8000);
        probe_kernel_endpoint(&peer.addr, port)
    }) {
        None => (false, None),
        Some(_) => (true, Some(0)),
    };

    KernelSpofSignal::single_vm(reachable, age_s)
}

/// Minimal TCP reachability probe. We don't authenticate or parse the kernel
/// protocol here — liveness is enough to decide degraded vs coordinated mode.
fn probe_kernel_endpoint(addr: &str, port: u16) -> bool {
    use std::net::ToSocketAddrs;
    let Ok(sockaddr) = (addr, port).to_socket_addrs() else {
        return false;
    };
    let mut iter = sockaddr;
    match iter.next() {
        Some(sa) => {
            let start = Instant::now();
            let timeout = Duration::from_millis(KERNEL_HEARTBEAT_TIMEOUT_MS);
            // TcpStream::connect_timeout is the only fallible step; any error
            // => unreachable => degrade. Never panic here.
            match std::net::TcpStream::connect_timeout(&sa, timeout) {
                Ok(_) => {
                    let _ = start.elapsed();
                    true
                }
                Err(_) => false,
            }
        }
        None => false,
    }
}

/// Snapshot helper for downstream read-back (CLI / kernel proof bundle).
pub fn kernel_spof_report_json() -> String {
    let signal = observe_kernel_spof();
    serde_json::to_string_pretty(&signal)
        .unwrap_or_else(|_| "{\"error\":\"kernel_spof_signal_unserializable\"}".to_string())
}

#[allow(dead_code)]
fn now_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
