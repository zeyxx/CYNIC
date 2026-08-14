use crate::domain::mesh::PeerIdentity;

/// Domain Entity: A piece of 'Waste' identified by the Garbage Collector
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Waste {
    GhostContainer(u32), // LXC/VM ID that exists on hypervisor but not in SSoT
    DanglingNetworkRoute(String), // e.g., "10.20.5.5" route with no matching Peer
    DeadProcess(u32),    // Local PID that is a zombie
    StaleMemory(String), // Old cache/log file
}

/// The Garbage Collector entity evaluates truth (SSoT) vs reality (System state)
pub struct GarbageCollector;

impl GarbageCollector {
    /// Pure domain logic: Compare Truth (SSoT) vs Reality to find Waste
    pub fn evaluate_hypervisor_waste(ssot_organs: &[u32], real_instances: &[u32]) -> Vec<Waste> {
        let mut waste = Vec::new();
        for &real_id in real_instances {
            if !ssot_organs.contains(&real_id) {
                waste.push(Waste::GhostContainer(real_id));
            }
        }
        waste
    }

    pub fn evaluate_mesh_waste(ssot_peers: &[PeerIdentity], real_routes: &[String]) -> Vec<Waste> {
        let mut waste = Vec::new();
        let ssot_ips: Vec<String> = ssot_peers
            .iter()
            .map(|p| p.internal_ip.to_string())
            .collect();

        for route in real_routes {
            if !ssot_ips.contains(route) {
                waste.push(Waste::DanglingNetworkRoute(route.clone()));
            }
        }
        waste
    }
}
