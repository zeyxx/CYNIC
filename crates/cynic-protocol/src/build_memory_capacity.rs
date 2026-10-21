//! Pure memory-capacity decision for the build runner.
//!
//! The disk gate refuses a checkout the runner cannot hold. This is its
//! memory twin: a build whose COMPILATION cannot hold is a build that dies
//! with SIGKILL, which the CI reports exactly like a failing assertion.
//!
//! WHY THIS EXISTS. Measured on builder-119 (8 GiB, 4 cores): the F-gate
//! is sharded into seven jobs that all share one `CARGO_TARGET_DIR`, and
//! two of them — `Gate 3 Canonical Paths` and `Gate 3 Knowledge` — build
//! the `cynic-cli` dependency tree. When they start together the runner
//! holds two concurrent workspace compilations. A single `rustc` peaks at
//! ~396 MiB; the two compilations plus the runner plus the page cache
//! crossed the physical RAM, and the kernel OOM-killed the compiler:
//!
//! ```text
//! cargo test -p cynic-cli --test canonical_paths_coherence
//! 2103008 Killed
//! RUN exit status 137
//! ```
//!
//! The failing job MOVED between two runs on a byte-identical tree
//! (`Gate 1` in one run, `Gate 3 Canonical Paths` in the next) and the
//! same test passes in isolation. That is the signature of a resource
//! peak, not of a defect in the code under test — but the CI cannot tell
//! the two apart, so the gate reports a failure that does not exist.
//!
//! The decision is pure: the adapter observes the memory, this module
//! decides how much parallelism that memory can hold. Nothing here reads
//! the machine.

use serde::{Deserialize, Serialize};

/// Schema identifier of the memory capacity decision.
pub const MEMORY_CAPACITY_DECISION_SCHEMA: &str = "cynic.build_runner.memory_capacity.v1";

/// Peak resident memory attributed to one compilation unit.
///
/// Measured, not guessed: 396 MiB for a `cynic-cli` test build on
/// builder-119. A crate with a large proc-macro or a big generic
/// instantiation costs more, so this is a floor for the estimate, and the
/// estimate is deliberately conservative.
pub const DEFAULT_UNIT_RSS_BYTES: u64 = 512 * 1024 * 1024;

/// Memory the runner keeps for itself, outside any compilation.
///
/// The runner, the act container, the page cache and the soak services
/// are all resident while a job compiles. Reserving this headroom is what
/// keeps the estimate on the safe side of the OOM killer.
pub const DEFAULT_RESERVED_BYTES: u64 = 1024 * 1024 * 1024;

/// Outcome of the pre-build memory gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCapacityDecision {
    /// The runner has room for `jobs` concurrent compilations.
    Proceed {
        /// How many concurrent compilations are safe.
        jobs: u32,
        /// The memory the estimate was derived from.
        available_bytes: u64,
    },
    /// The runner cannot hold a single compilation. The build must not
    /// start: it would be killed by the OOM killer and reported as a
    /// test failure.
    Refused {
        available_bytes: u64,
        required_bytes: u64,
    },
    /// The memory state could not be observed. Fail closed: an unknown
    /// memory budget is not a generous one, and a build that starts
    /// against an unknown budget dies in a way that looks like a defect.
    Unknown { reason: String },
}

/// The full decision record, with the read-back that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCapacityAssessment {
    pub schema: &'static str,
    pub decision: MemoryCapacityDecision,
    /// Bytes observed available on the runner, when the read-back succeeded.
    pub available_bytes: Option<u64>,
    /// Bytes the estimate reserves for one compilation unit.
    pub unit_rss_bytes: u64,
    /// Bytes the estimate reserves for the runner itself.
    pub reserved_bytes: u64,
    /// How many concurrent compilations are safe.
    ///
    /// This is the FLAT, stable projection of the decision: the enum
    /// serializes as `{"proceed": {"jobs": N}}`, so a consumer reading
    /// `decision.jobs` would find nothing. A caller that needs the
    /// parallelism reads this field, which is present and `Some` exactly
    /// when the decision is `Proceed`, and `None` otherwise.
    pub jobs: Option<u32>,
}

/// Decide how many concurrent compilations the runner can hold.
///
/// `available_bytes` is `None` when the read-back failed. The decision is
/// fail-closed: an unobserved memory budget blocks the build with an
/// explicit unknown, never a silent proceed.
///
/// The estimate divides the usable memory (available minus reserved) by
/// the per-unit cost, and never returns zero: a build that cannot run one
/// compilation is refused, not given a parallelism of zero, because zero
/// would hang instead of failing.
pub fn decide_memory_capacity(
    available_bytes: Option<u64>,
    unit_rss_bytes: u64,
    reserved_bytes: u64,
) -> MemoryCapacityAssessment {
    decide_memory_capacity_with_cores(available_bytes, unit_rss_bytes, reserved_bytes, None)
}

/// Decide how many concurrent compilations fit, bounded by the core count.
///
/// The memory bound alone over-estimates on a small machine: 8 GiB with
/// 512 MiB per unit yields ten, but the runner has four cores, so ten
/// compilations would time-share four cores while holding ten times the
/// memory. The parallelism is therefore the SMALLER of the memory bound
/// and the core count: the machine can only run so many at once, and
/// running more of them than it has cores costs memory without buying
/// time.
pub fn decide_memory_capacity_with_cores(
    available_bytes: Option<u64>,
    unit_rss_bytes: u64,
    reserved_bytes: u64,
    cores: Option<u32>,
) -> MemoryCapacityAssessment {
    let decision = match available_bytes {
        None => MemoryCapacityDecision::Unknown {
            reason: "runner_memory_unobserved".to_string(),
        },
        Some(bytes) => {
            let required = unit_rss_bytes.saturating_add(reserved_bytes);
            if bytes < required {
                MemoryCapacityDecision::Refused {
                    available_bytes: bytes,
                    required_bytes: required,
                }
            } else {
                let usable = bytes - reserved_bytes;
                let memory_bound = (usable / unit_rss_bytes.max(1)).max(1);
                let jobs = match cores {
                    Some(cores) if cores > 0 => memory_bound.min(cores as u64),
                    _ => memory_bound,
                };
                MemoryCapacityDecision::Proceed {
                    jobs: jobs.min(u32::MAX as u64) as u32,
                    available_bytes: bytes,
                }
            }
        }
    };
    let jobs = match &decision {
        MemoryCapacityDecision::Proceed { jobs, .. } => Some(*jobs),
        MemoryCapacityDecision::Refused { .. } | MemoryCapacityDecision::Unknown { .. } => None,
    };
    MemoryCapacityAssessment {
        schema: MEMORY_CAPACITY_DECISION_SCHEMA,
        decision,
        available_bytes,
        unit_rss_bytes,
        reserved_bytes,
        jobs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gib(bytes: u64) -> u64 {
        bytes * 1024 * 1024 * 1024
    }

    #[test]
    fn an_unobserved_memory_budget_refuses_the_build() {
        let assessment = decide_memory_capacity(None, DEFAULT_UNIT_RSS_BYTES, DEFAULT_RESERVED_BYTES);
        assert_eq!(
            assessment.decision,
            MemoryCapacityDecision::Unknown {
                reason: "runner_memory_unobserved".to_string()
            }
        );
    }

    #[test]
    fn a_memory_budget_below_one_unit_refuses_the_build() {
        let assessment = decide_memory_capacity(
            Some(gib(1)),
            DEFAULT_UNIT_RSS_BYTES,
            DEFAULT_RESERVED_BYTES,
        );
        assert!(matches!(
            assessment.decision,
            MemoryCapacityDecision::Refused { .. }
        ));
    }

    #[test]
    fn eight_gibibytes_allow_several_compilations() {
        // Measured on builder-119: 8 GiB total, ~6.3 GiB available,
        // 512 MiB per unit, 1 GiB reserved.
        let assessment = decide_memory_capacity(
            Some(6 * gib(1) + 300 * 1024 * 1024),
            DEFAULT_UNIT_RSS_BYTES,
            DEFAULT_RESERVED_BYTES,
        );
        match assessment.decision {
            MemoryCapacityDecision::Proceed { jobs, .. } => {
                assert!(jobs >= 5, "expected several concurrent compilations, got {jobs}");
                assert!(jobs <= 13, "expected a bounded parallelism, got {jobs}");
            }
            other => panic!("expected proceed, got {other:?}"),
        }
    }

    #[test]
    fn the_estimate_never_returns_zero() {
        // Exactly enough for one unit: one job, not zero. Zero would hang.
        let assessment = decide_memory_capacity(
            Some(DEFAULT_UNIT_RSS_BYTES + DEFAULT_RESERVED_BYTES),
            DEFAULT_UNIT_RSS_BYTES,
            DEFAULT_RESERVED_BYTES,
        );
        match assessment.decision {
            MemoryCapacityDecision::Proceed { jobs, .. } => assert_eq!(jobs, 1),
            other => panic!("expected proceed, got {other:?}"),
        }
    }

    #[test]
    fn the_flat_projection_carries_the_jobs_only_when_proceeding() {
        let proceeding = decide_memory_capacity(
            Some(gib(4)),
            DEFAULT_UNIT_RSS_BYTES,
            DEFAULT_RESERVED_BYTES,
        );
        assert!(proceeding.jobs.is_some(), "a proceed must expose its jobs");

        let refused =
            decide_memory_capacity(Some(gib(1)), DEFAULT_UNIT_RSS_BYTES, DEFAULT_RESERVED_BYTES);
        assert!(
            refused.jobs.is_none(),
            "a refusal must not expose a parallelism"
        );

        let unknown =
            decide_memory_capacity(None, DEFAULT_UNIT_RSS_BYTES, DEFAULT_RESERVED_BYTES);
        assert!(
            unknown.jobs.is_none(),
            "an unknown budget must not expose a parallelism"
        );
    }

    #[test]
    fn the_core_count_bounds_the_memory_bound() {
        // 8 GiB yields ten by memory alone, but the runner has four cores:
        // ten compilations on four cores time-share the cores while
        // holding ten times the memory.
        let by_memory = decide_memory_capacity(
            Some(6 * gib(1) + 300 * 1024 * 1024),
            DEFAULT_UNIT_RSS_BYTES,
            DEFAULT_RESERVED_BYTES,
        );
        let bounded = decide_memory_capacity_with_cores(
            Some(6 * gib(1) + 300 * 1024 * 1024),
            DEFAULT_UNIT_RSS_BYTES,
            DEFAULT_RESERVED_BYTES,
            Some(4),
        );
        let (memory_jobs, bounded_jobs) = match (by_memory.decision, bounded.decision) {
            (
                MemoryCapacityDecision::Proceed { jobs: a, .. },
                MemoryCapacityDecision::Proceed { jobs: b, .. },
            ) => (a, b),
            other => panic!("expected proceed on both sides, got {other:?}"),
        };
        assert!(memory_jobs > bounded_jobs, "the memory bound alone is looser");
        assert_eq!(bounded_jobs, 4, "the core count is the tighter bound");
    }

    #[test]
    fn an_unobserved_core_count_leaves_the_memory_bound_alone() {
        let by_memory = decide_memory_capacity(
            Some(6 * gib(1) + 300 * 1024 * 1024),
            DEFAULT_UNIT_RSS_BYTES,
            DEFAULT_RESERVED_BYTES,
        );
        let unknown_cores = decide_memory_capacity_with_cores(
            Some(6 * gib(1) + 300 * 1024 * 1024),
            DEFAULT_UNIT_RSS_BYTES,
            DEFAULT_RESERVED_BYTES,
            None,
        );
        assert_eq!(by_memory.jobs, unknown_cores.jobs);
    }

    #[test]
    fn a_larger_reservation_yields_a_smaller_parallelism() {
        let available = Some(gib(4));
        let wide = decide_memory_capacity(available, DEFAULT_UNIT_RSS_BYTES, 0);
        let narrow = decide_memory_capacity(available, DEFAULT_UNIT_RSS_BYTES, gib(2));
        let (wide_jobs, narrow_jobs) = match (wide.decision, narrow.decision) {
            (
                MemoryCapacityDecision::Proceed { jobs: a, .. },
                MemoryCapacityDecision::Proceed { jobs: b, .. },
            ) => (a, b),
            other => panic!("expected proceed on both sides, got {other:?}"),
        };
        assert!(narrow_jobs < wide_jobs);
    }
}
