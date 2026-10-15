//! Pure policy predicates for build latency (Task 6).
//!
//! Each predicate encodes one winning threshold measured in Tasks 1-5.
//! Zero I/O -- orchestration stays in cynic-node.

// -- H1: stable checkout reuse ----------------------------------------------

/// Whether to reuse a stable workspace path for builds.
///
/// H1 measured: persistent stable checkout + shared `CARGO_TARGET_DIR`
/// gives **-51.6 %** execute on same SHA (92 929 ms -> 44 985 ms).
/// Source: Task 2 settlement sidecar, builder 10.10.10.119, 2026-10-14.
pub fn decide_reuse_stable_checkout(stable_root_configured: bool) -> bool {
    stable_root_configured
}

// -- H2: parallel slot admission with RSS cap -------------------------------

/// RSS cap in MB -- H2 measured peak ~1.2-1.3 GB flat across JOBS 1/2/4 on
/// 4-CPU/8GB builder. Cap set at **6144 MB** (6 GiB) for headroom.
/// Source: Task 3 settlement sidecar, builder 10.10.10.119, 2026-10-14.
pub const RSS_CAP_MB: u64 = 6144;

/// Whether to admit a parallel build slot given projected RSS.
///
/// H2 measured: `jobs=2` is the sweet spot on 4-CPU/8GB
/// (79 691 ms -> 49 962 ms -> 46 222 ms for J1/J2/J4).
/// RSS peak ~1.2-1.3 GB flat across JOBS, cap **6144 MB**.
/// Source: Task 3 settlement sidecar, builder 10.10.10.119, 2026-10-14.
pub fn decide_admit_parallel_slot(rss_cap_mb: u64, projected_mb: u64) -> bool {
    projected_mb <= rss_cap_mb
}

// -- H3: sccache adoption ----------------------------------------------------

/// sccache adopt threshold -- H3 measured cross-SHA Rust hit rate **~90 %**.
/// Adopt threshold was **20 %**.
/// Source: Task 4 sccache stats, builder 10.10.10.119, 2026-10-14.
pub const SCCACHE_ADOPT_THRESHOLD: f64 = 0.20;

/// Whether to require sccache based on measured hit rate.
///
/// H3 measured: sccache cross-SHA Rust hit rate **~90 %**
/// (adopt threshold was **20 %**).
/// Source: Task 4 sccache stats, builder 10.10.10.119, 2026-10-14.
pub fn decide_require_sccache(hit_rate_floor: f64, measured_hit_rate: f64) -> bool {
    measured_hit_rate >= hit_rate_floor
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- H1: stable checkout reuse -----------------------------------------

    #[test]
    fn h1_reuse_when_stable_root_configured() {
        // H1: stable checkout gives -51.6 % execute (92 929 -> 44 985 ms).
        assert!(decide_reuse_stable_checkout(true));
    }

    #[test]
    fn h1_no_reuse_without_stable_root() {
        assert!(!decide_reuse_stable_checkout(false));
    }

    // -- H2: parallel slot admission with RSS cap ---------------------------

    #[test]
    fn h2_admit_when_projected_at_cap() {
        // Boundary: projected == cap -> admit.
        assert!(decide_admit_parallel_slot(RSS_CAP_MB, RSS_CAP_MB));
    }

    #[test]
    fn h2_admit_when_projected_below_cap() {
        // Measured peak ~1.2-1.3 GB, well under 6144 MB cap.
        assert!(decide_admit_parallel_slot(RSS_CAP_MB, 1300));
    }

    #[test]
    fn h2_reject_when_projected_above_cap() {
        // Boundary: projected > cap -> reject.
        assert!(!decide_admit_parallel_slot(RSS_CAP_MB, RSS_CAP_MB + 1));
    }

    #[test]
    fn h2_reject_when_projected_far_above_cap() {
        assert!(!decide_admit_parallel_slot(RSS_CAP_MB, 8192));
    }

    #[test]
    fn h2_cap_constant_is_6144() {
        // H2 winning number: cap 6144 MB.
        assert_eq!(RSS_CAP_MB, 6144);
    }

    // -- H3: sccache adoption ------------------------------------------------

    #[test]
    fn h3_require_when_measured_at_threshold() {
        // Boundary: measured == floor -> require.
        assert!(decide_require_sccache(SCCACHE_ADOPT_THRESHOLD, 0.20));
    }

    #[test]
    fn h3_require_when_measured_above_threshold() {
        // H3 measured: ~90 % hit rate.
        assert!(decide_require_sccache(SCCACHE_ADOPT_THRESHOLD, 0.90));
    }

    #[test]
    fn h3_no_require_when_measured_below_threshold() {
        // Boundary: measured < floor -> do not require.
        assert!(!decide_require_sccache(SCCACHE_ADOPT_THRESHOLD, 0.19));
    }

    #[test]
    fn h3_no_require_when_measured_zero() {
        assert!(!decide_require_sccache(SCCACHE_ADOPT_THRESHOLD, 0.0));
    }

    #[test]
    fn h3_adopt_threshold_constant() {
        // H3 winning number: adopt threshold 20 %.
        assert_eq!(SCCACHE_ADOPT_THRESHOLD, 0.20);
    }
}
