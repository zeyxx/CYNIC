//! Shared portable OS substrate probes.
//!
//! These read-only helpers were previously duplicated between
//! `os_health/observe.rs` and `peer_resolver_mesh.rs`. They are extracted here
//! as a single source of truth so a probe change (e.g. a new memory threshold)
//! cannot silently diverge between the two call sites.
//!
//! Portable kernel interfaces only (`/proc/sys`, `/proc/meminfo`); no mutation,
//! no hardcoding of machine names, no required root. If a file is unreadable
//! the field is reported as a safe sentinel (`false` / `0`) so the decision
//! layer preserves the gap instead of guessing.

/// Read a `0`/`1` sysctl-style file. Returns `false` when unreadable
/// (conservative: assume "does not panic" / disabled when unknown).
pub fn read_bool(path: &str) -> bool {
    std::fs::read_to_string(path)
        .map(|c| c.trim() == "1")
        .unwrap_or(false)
}

/// Available memory in MiB from `/proc/meminfo` `MemAvailable`.
/// Returns `0` when unreadable.
pub fn read_available_mem_mib() -> u64 {
    let content = match std::fs::read_to_string("/proc/meminfo") {
        Ok(c) => c,
        Err(_) => return 0,
    };
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            if let Some(kb) = rest.split_whitespace().next() {
                if let Ok(kb_val) = kb.parse::<u64>() {
                    return kb_val / 1024;
                }
            }
        }
    }
    0
}

/// Reclaim pressure as a percentage of total memory (memory NOT available).
/// Best-effort from `MemAvailable` vs `MemTotal` in `/proc/meminfo`.
/// Returns `0` when unreadable or total is zero.
pub fn read_reclaim_pressure_pct() -> u8 {
    let content = match std::fs::read_to_string("/proc/meminfo") {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let mut avail = 0u64;
    let mut total = 0u64;
    for line in content.lines() {
        if let Some(r) = line.strip_prefix("MemAvailable:") {
            avail = r
                .split_whitespace()
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        } else if let Some(r) = line.strip_prefix("MemTotal:") {
            total = r
                .split_whitespace()
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        }
    }
    if total == 0 {
        return 0;
    }
    ((total.saturating_sub(avail)) as f64 / total as f64 * 100.0) as u8
}
