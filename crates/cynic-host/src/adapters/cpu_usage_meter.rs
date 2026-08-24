//! CpuUsageMeter — monotone-window cpu_ms measurement via sysinfo (plan §9
//! gap 6, opportunistic dimension).
//!
//! Data-centric layering:
//!   1. This adapter samples CPU jiffies/percent at window open and close
//!      and converts the delta to cpu_ms (cpu-percent × wall-ms / 100).
//!   2. It never guesses: a missing sample yields None, the unknown that
//!      `EnergyConsumed::within` already treats as honest.
//!   3. It is Sync+Send and cheap to hold (sysinfo refresh is incremental).

use std::time::Instant;

use sysinfo::System;

/// Measures cpu_ms between two marks on a monotone clock. One instance per
/// metering scope (e.g. per tick window). Clone-free: hand out Arc if the
/// open/close sites differ.
pub struct CpuUsageMeter {
    system: System,
    /// Wall-clock mark of the last sample — deltas pair with cpu percent.
    marked_at: Instant,
    /// Last observed global cpu usage percent (0..100), None before first
    /// successful refresh.
    last_cpu_pct: Option<f32>,
}

impl Default for CpuUsageMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuUsageMeter {
    pub fn new() -> Self {
        let mut system = System::new();
        // Two refreshes separated by MIN_CPU_INTERVAL give sysinfo a delta
        // baseline; without it the first read reports 0% spuriously.
        system.refresh_cpu_usage();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        system.refresh_cpu_usage();
        let last_cpu_pct = Some(system.global_cpu_usage());
        Self {
            system,
            marked_at: Instant::now(),
            last_cpu_pct,
        }
    }

    /// Re-sample now. Cheap enough to call at every window boundary.
    pub fn mark(&mut self) -> CpuMark {
        self.system.refresh_cpu_usage();
        self.last_cpu_pct = Some(self.system.global_cpu_usage());
        self.marked_at = Instant::now();
        CpuMark {
            sampled_at: self.marked_at,
            cpu_pct: self.last_cpu_pct,
        }
    }

    /// cpu_ms consumed between two marks. Either mark missing its cpu
    /// sample → None (unknown, not zero). Formula: pct × Δwall_ms / 100.
    pub fn cpu_ms_between(&self, earlier: &CpuMark, later: &CpuMark) -> Option<u64> {
        let (pct_e, pct_l) = (earlier.cpu_pct?, later.cpu_pct?);
        if later.sampled_at <= earlier.sampled_at {
            return None; // inverted or zero-length interval → unmeasurable
        }
        let wall_ms = later
            .sampled_at
            .duration_since(earlier.sampled_at)
            .as_millis() as f64;
        let avg_pct = ((pct_e + pct_l) as f64) / 2.0;
        Some((wall_ms * avg_pct / 100.0).round() as u64)
    }
}

/// A single cpu sample bound to its monotone timestamp.
#[derive(Debug, Clone, Copy)]
pub struct CpuMark {
    pub(crate) sampled_at: Instant,
    pub(crate) cpu_pct: Option<f32>,
}

impl CpuMark {
    /// Wall ms elapsed from this mark to a later monotone instant.
    pub fn elapsed_ms_since(&self, later: Instant) -> u64 {
        later.duration_since(self.sampled_at).as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_marks_yield_measurable_cpu_ms() {
        let mut meter = CpuUsageMeter::new();
        let start = meter.mark();
        // Busy work so the delta has something to observe; even ~0% is fine,
        // the contract is Some(_), not a minimum value.
        let mut acc = 0u64;
        for i in 0..200_000u64 {
            acc = acc.wrapping_add(i.wrapping_mul(2654435761));
        }
        std::hint::black_box(acc);
        let end = meter.mark();
        let cpu_ms = meter.cpu_ms_between(&start, &end);
        assert!(cpu_ms.is_some(), "both marks carry samples");
        // Sanity: cannot exceed wall time by more than core-count factor on
        // any sane host; we only assert it parsed into a finite u64 here.
        let _ = cpu_ms.unwrap();
    }

    #[test]
    fn inverted_interval_is_unknown_not_zero() {
        let mut meter = CpuUsageMeter::new();
        let a = meter.mark();
        let b = meter.mark();
        // Swapped order must yield None (unmeasurable), never 0 (fabricated).
        assert!(meter.cpu_ms_between(&b, &a).is_none());
        let _ = a;
    }

    #[test]
    fn zero_length_interval_is_none() {
        let mut m1 = CpuUsageMeter::new();
        // Same mark compared with itself: Δwall = 0 → unmeasurable.
        let mark = m1.mark();
        assert!(m1.cpu_ms_between(&mark, &mark).is_none());
    }

    #[test]
    fn missing_sample_is_none_via_synthetic_marks() {
        // Construct marks directly through the public struct fields being
        // private — use the real meter but simulate a missing sample by
        // checking the None branch via an un-sampled clone path.
        let mut meter = CpuUsageMeter::new();
        let early = CpuMark {
            sampled_at: Instant::now(),
            cpu_pct: None, // simulated failed sample
        };
        let late = meter.mark();
        assert!(meter.cpu_ms_between(&early, &late).is_none());
        assert!(meter.cpu_ms_between(&late, &early).is_none());
    }
}
