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

/// Why a build attempt produced no verdict about the source under test.
///
/// A toolchain crash (rustc ICE, `SIGSEGV`, corrupted codegen) is a fact about
/// the *substrate*, never about the code. Some hosts exhibit it chronically and
/// non-reproducibly; the runtime must therefore be able to say "this attempt
/// proved nothing" instead of recording a false failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolchainFaultClass {
    /// Internal compiler error / crash signal. Not attributable to the source.
    CompilerCrash,
    /// Wall-clock budget exhausted before any verdict.
    BudgetExhausted,
}

/// Verdict of one build attempt, separating source truth from substrate noise.
///
/// This is the type that makes the lie unrepresentable: a `ToolchainFault`
/// cannot carry a source verdict, so a chronic ICE host can never be recorded
/// as having proved the code broken.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptVerdict {
    /// The toolchain ran to completion and judged the source.
    SourceJudged { passed: bool, exit_code: i32 },
    /// The toolchain failed to produce a judgement. Says nothing about source.
    ToolchainFault {
        class: ToolchainFaultClass,
        /// The node that exhibited the fault — attribution is to substrate.
        node_id: String,
    },
}

impl AttemptVerdict {
    /// Classify a raw read-back into a verdict.
    ///
    /// `BuildStatus::Unknown` is a fault, not a pass: an unparsable or absent
    /// outcome must never be projected as success (AGENTS.md: unknown is data).
    pub fn from_result(result: &BuildResult, node_id: &str) -> Self {
        match result.status {
            BuildStatus::Success => Self::SourceJudged {
                passed: true,
                exit_code: result.exit_code,
            },
            BuildStatus::Ice => Self::ToolchainFault {
                class: ToolchainFaultClass::CompilerCrash,
                node_id: node_id.to_string(),
            },
            BuildStatus::Timeout => Self::ToolchainFault {
                class: ToolchainFaultClass::BudgetExhausted,
                node_id: node_id.to_string(),
            },
            BuildStatus::Unknown => Self::ToolchainFault {
                class: ToolchainFaultClass::CompilerCrash,
                node_id: node_id.to_string(),
            },
        }
    }

    /// True when this attempt carries information about the source.
    pub fn is_source_evidence(&self) -> bool {
        matches!(self, Self::SourceJudged { .. })
    }
}

/// What the reconciler decides to do after an attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptDisposition {
    /// A source verdict was obtained. Stop; record it as proof.
    AcceptVerdict { passed: bool },
    /// Substrate faulted and budget remains on the same boundary. Retry as-is;
    /// the source is unchanged so this is not a code repair.
    RetrySameNode { attempt: u32 },
    /// Faults reached `isolate_after`. Move to a stronger isolation boundary
    /// (or a different node) before spending more attempts.
    EscalateIsolation { after_faults: u32 },
    /// `max_attempts` consumed without any source verdict. The correct terminal
    /// state is an explicit unknown, never a failure attributed to the code.
    ExhaustedWithoutVerdict { faults: u32 },
}

/// Pure decision: given the fault history of a job, what happens next.
///
/// `faults_so_far` counts attempts already lost to the substrate (this call
/// classifies the current one). Chronic-ICE hosts are handled by policy here
/// instead of by ad-hoc retry loops in adapters or CLI surfaces (AGENTS.md §6).
pub fn decide_attempt_disposition(
    verdict: &AttemptVerdict,
    faults_so_far: u32,
    policy: &RetryPolicy,
) -> AttemptDisposition {
    match verdict {
        AttemptVerdict::SourceJudged { passed, .. } => {
            AttemptDisposition::AcceptVerdict { passed: *passed }
        }
        AttemptVerdict::ToolchainFault { .. } => {
            let faults = faults_so_far.saturating_add(1);
            if faults >= policy.max_attempts {
                AttemptDisposition::ExhaustedWithoutVerdict { faults }
            } else if policy.isolate_after > 0 && faults >= policy.isolate_after {
                AttemptDisposition::EscalateIsolation {
                    after_faults: faults,
                }
            } else {
                AttemptDisposition::RetrySameNode { attempt: faults }
            }
        }
    }
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

    fn chronic_ice_policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 4,
            isolate_after: 2,
        }
    }

    fn result(status: BuildStatus, exit_code: i32) -> BuildResult {
        BuildResult {
            status,
            exit_code,
            evidence_refs: vec![],
        }
    }

    #[test]
    fn ice_never_becomes_a_source_verdict() {
        // The core refusal: a chronic-ICE host must not be able to report that
        // it proved the code broken.
        let verdict = AttemptVerdict::from_result(&result(BuildStatus::Ice, 101), "cortex");
        assert!(!verdict.is_source_evidence());
        assert_eq!(
            verdict,
            AttemptVerdict::ToolchainFault {
                class: ToolchainFaultClass::CompilerCrash,
                node_id: "cortex".into(),
            }
        );
    }

    #[test]
    fn unknown_status_is_a_fault_not_a_pass() {
        let verdict = AttemptVerdict::from_result(&result(BuildStatus::Unknown, 0), "cortex");
        assert!(!verdict.is_source_evidence());
    }

    #[test]
    fn timeout_is_attributed_to_budget_not_to_source() {
        let verdict = AttemptVerdict::from_result(&result(BuildStatus::Timeout, 124), "cortex");
        assert_eq!(
            verdict,
            AttemptVerdict::ToolchainFault {
                class: ToolchainFaultClass::BudgetExhausted,
                node_id: "cortex".into(),
            }
        );
    }

    #[test]
    fn success_carries_the_source_verdict() {
        let verdict =
            AttemptVerdict::from_result(&result(BuildStatus::Success, 0), "cynic-builder");
        assert!(verdict.is_source_evidence());
        assert_eq!(
            decide_attempt_disposition(&verdict, 0, &chronic_ice_policy()),
            AttemptDisposition::AcceptVerdict { passed: true }
        );
    }

    #[test]
    fn chronic_ice_escalates_then_exhausts_without_blaming_source() {
        let policy = chronic_ice_policy();
        let fault = AttemptVerdict::from_result(&result(BuildStatus::Ice, 101), "cortex");

        // First fault: retry the same boundary — source unchanged.
        assert_eq!(
            decide_attempt_disposition(&fault, 0, &policy),
            AttemptDisposition::RetrySameNode { attempt: 1 }
        );
        // Second fault reaches isolate_after: move to a stronger boundary.
        assert_eq!(
            decide_attempt_disposition(&fault, 1, &policy),
            AttemptDisposition::EscalateIsolation { after_faults: 2 }
        );
        // Budget consumed: terminal state is an explicit unknown, never a
        // failure attributed to the code.
        assert_eq!(
            decide_attempt_disposition(&fault, 3, &policy),
            AttemptDisposition::ExhaustedWithoutVerdict { faults: 4 }
        );
    }

    #[test]
    fn a_real_compile_error_is_source_evidence_even_when_it_fails() {
        // cynic-builder returning E0599 is a verdict about the code; it must be
        // accepted as proof, unlike an ICE on the same source.
        let verdict = AttemptVerdict::SourceJudged {
            passed: false,
            exit_code: 101,
        };
        assert!(verdict.is_source_evidence());
        assert_eq!(
            decide_attempt_disposition(&verdict, 2, &chronic_ice_policy()),
            AttemptDisposition::AcceptVerdict { passed: false }
        );
    }
}
