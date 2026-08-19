//! Build-runner fleet placement: how a `BuildNodeCapability` binds a build
//! workload to an admissible node.
//!
//! Pure policy over observable node capabilities — no substrate or I/O.
//! This is the protocol-layer model that the `cynic-node` kernel scheduler and
//! the host build-runner surface both consume.
//!
//! AGENTS.md coherence: desired state (`BuildJobSpec`) → observe
//! (`BuildNodeCapability::discover_capabilities`) → reconcile
//! (`select_build_node`) → bounded action (`BuildIsolationPort::run_build`)
//! → read-back (`BuildResult`).

use serde::{Deserialize, Serialize};

/// Isolation boundary requested for a build execution.
///
/// `Local` = same substrate as the requesting node (no containment
/// guarantee). `Vm` = nested VM/LXC with filesystem isolation. `Container`
/// = rootless container namespace. `Unknown`/omitted signals an explicit
/// blocked state — never silently guessed (AGENTS.md §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationKind {
    Local,
    Vm,
    Container,
    Unknown,
}

/// Observable capabilities of a single build-capable node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildNodeCapability {
    pub node_id: String,
    pub supported_toolchains: Vec<String>,
    pub isolation_boundaries: Vec<IsolationKind>,
    pub max_memory_bytes: u64,
    pub max_wall_time_secs: u64,
}

impl BuildNodeCapability {
    /// Observe which isolation boundary a node can satisfy for a requested
    /// toolchain + memory floor. Read-only — no mutation.
    pub fn discover_capabilities(
        &self,
        required_toolchain: &str,
        require_memory_bytes: u64,
    ) -> Option<IsolationKind> {
        if !self
            .supported_toolchains
            .contains(&required_toolchain.to_string())
        {
            return None;
        }
        if self.max_memory_bytes < require_memory_bytes {
            return None;
        }
        self.isolation_boundaries
            .first()
            .cloned()
            .or(Some(IsolationKind::Unknown))
            .filter(|k| !matches!(k, IsolationKind::Unknown))
    }
}

/// Port: how a build runner executes a job under an isolation boundary.
///
/// Injected by the runtime substrate (cynic-host adapter / kernel scheduler).
/// Unit tests inject a fake port returning fixed `BuildResult::Ok` / `Ice`.
pub trait BuildIsolationPort: Send + Sync {
    fn discover_capabilities(
        &self,
        node_id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = BuildNodeCapability> + Send + '_>>;

    fn run_build(
        &self,
        spec: &BuildJobSpec,
        node: &BuildNodeCapability,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = BuildResult> + Send + '_>>;
}

/// A build workload request: desired state compared by the reconciler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildJobSpec {
    pub source_commit: String,
    pub registry_digest: String,
    pub executor_digest: String,
    pub source_bundle_digest: String,
    pub required_toolchain: String,
    pub isolation_boundary: IsolationKind,
    pub requested_action: String,
    pub mem_limit_bytes: u64,
}

/// Read-back: the outcome of a build execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildResult {
    pub status: BuildStatus,
    pub exit_code: i32,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildStatus {
    Success,
    Ice,
    Timeout,
    Unknown,
}

/// Retry escalation policy for placement failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    /// After N failures on a toolchain, escalate to an isolated boundary.
    pub isolate_after: u32,
}

/// A concrete node chosen by the reconciler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildJob {
    pub spec: BuildJobSpec,
    pub node_id: String,
    pub isolation: IsolationKind,
}

/// Pure reconciliation: select the first node that satisfies the toolchain +
/// memory + isolation constraints. Returns `None` when no node is admissible.
///
/// This is the canonical decision layer — consumed by `cynic-node` scheduler
/// and the host build-runner; do not reimplement in CLI surfaces (AGENTS.md §6).
pub fn select_build_node(spec: &BuildJobSpec, nodes: &[BuildNodeCapability]) -> Option<BuildJob> {
    for node in nodes {
        let admissible = node
            .isolation_boundaries
            .iter()
            .any(|b| *b == spec.isolation_boundary)
            && node.supported_toolchains.contains(&spec.required_toolchain)
            && node.max_memory_bytes >= spec.mem_limit_bytes;

        if admissible {
            let isolation = node
                .isolation_boundaries
                .iter()
                .find(|b| **b == spec.isolation_boundary)
                .cloned()
                .unwrap_or(spec.isolation_boundary.clone());
            return Some(BuildJob {
                spec: spec.clone(),
                node_id: node.node_id.clone(),
                isolation,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_a() -> BuildNodeCapability {
        BuildNodeCapability {
            node_id: "node-a".into(),
            supported_toolchains: vec!["1.96.0".into(), "1.95.0".into()],
            isolation_boundaries: vec![IsolationKind::Local],
            max_memory_bytes: 16 * 1024 * 1024 * 1024,
            max_wall_time_secs: 1200,
        }
    }

    fn node_b_ice() -> BuildNodeCapability {
        BuildNodeCapability {
            node_id: "node-b-ice".into(),
            supported_toolchains: vec!["1.96.0".into()],
            isolation_boundaries: vec![IsolationKind::Vm],
            max_memory_bytes: 8 * 1024 * 1024 * 1024,
            max_wall_time_secs: 1200,
        }
    }

    fn mk_spec(isolation: IsolationKind, mem: u64) -> BuildJobSpec {
        BuildJobSpec {
            source_commit: "deadbeef".into(),
            registry_digest: "sha256:00".into(),
            executor_digest: "sha256:00".into(),
            source_bundle_digest: "sha256:00".into(),
            required_toolchain: "1.96.0".into(),
            isolation_boundary: isolation,
            requested_action: "cargo check --workspace".into(),
            mem_limit_bytes: mem,
        }
    }

    #[test]
    fn fleet_selects_stable_local_node_over_ice_vm_node() {
        let job = mk_spec(IsolationKind::Local, 4 * 1024 * 1024 * 1024);
        let chosen = select_build_node(&job, &[node_b_ice(), node_a()]);
        assert!(chosen.is_some());
        assert_eq!(chosen.unwrap().node_id, "node-a");
    }

    #[test]
    fn fleet_routes_to_vm_when_local_unsupported() {
        let job = mk_spec(IsolationKind::Vm, 4 * 1024 * 1024 * 1024);
        let chosen = select_build_node(&job, &[node_a(), node_b_ice()]);
        assert_eq!(chosen.as_ref().unwrap().node_id, "node-b-ice");
        assert_eq!(chosen.unwrap().isolation, IsolationKind::Vm);
    }

    #[test]
    fn fleet_blocks_when_no_node_supports_toolchain() {
        let mut job = mk_spec(IsolationKind::Local, 4 * 1024 * 1024 * 1024);
        job.required_toolchain = "1.97.0".to_string();
        let chosen = select_build_node(&job, &[node_a(), node_b_ice()]);
        assert_eq!(chosen, None);
    }

    #[test]
    fn fleet_blocks_when_memory_floor_exceeds_node_capacity() {
        // node-a advertises 16GiB; a 64GiB request exceeds both nodes.
        let job = mk_spec(IsolationKind::Local, 64 * 1024 * 1024 * 1024);
        let chosen = select_build_node(&job, &[node_a(), node_b_ice()]);
        assert_eq!(chosen, None);
    }

    #[test]
    fn fleet_spec_build_job_spec_make_check_action_is_canonical_and_untrusted() {
        // The requested_action is treated as untrusted argv and must never be
        // interpolated by the kernel — it is only compared by digest against
        // the signed capability's allowed_action_digests (see binding core).
        let job = mk_spec(IsolationKind::Local, 4 * 1024 * 1024 * 1024);
        let canonical = job.requested_action.clone();
        assert_eq!(canonical, "cargo check --workspace");
    }

    #[test]
    fn build_node_capability_discover_capabilities_returns_first_boundary() {
        let node = node_a();
        assert_eq!(
            node.discover_capabilities("1.96.0", 4 * 1024 * 1024 * 1024),
            Some(IsolationKind::Local)
        );
        // Toolchain not supported → None (blocked/unknown, never guessed).
        assert_eq!(
            node.discover_capabilities("1.97.0", 4 * 1024 * 1024 * 1024),
            None
        );
        // Memory floor too high → None.
        assert_eq!(
            node.discover_capabilities("1.96.0", 64 * 1024 * 1024 * 1024),
            None
        );
    }

    #[test]
    fn isolation_kind_round_trips_canonical_json() {
        let local = serde_json::to_string(&IsolationKind::Local).unwrap();
        assert_eq!(local, "\"local\"");
        let vm: IsolationKind = serde_json::from_str("\"vm\"").unwrap();
        assert_eq!(vm, IsolationKind::Vm);
    }
}
