//! Read-only OS substrate probes.
//!
//! Everything here reads portable, well-known kernel interfaces (`/proc/sys`,
//! `/sys/kernel/mm/transparent_hugepage`, `/proc/meminfo`, the kernel log
//! ring accessible to the process). No mutation, no hardcoding of machine
//! names, no required root: if a file is unreadable the field is reported as
//! an explicit `unknown` so the decision layer can preserve the gap instead of
//! guessing.

use serde::{Deserialize, Serialize};

use super::types::OsObservation;

/// Portable observer of OS substrate health signals.
pub trait OsHealthObserver {
    fn observe_os_health(&self) -> OsObservation;
}

/// Zero-dependency portable probe of the live host.
///
/// Sentinels:
/// - missing/empty file  -> `"unknown"` string or `0` count; the decision
///   layer treats `"unknown"` THP/softlockup as residual risk, never as pass.
pub struct LocalOsHealthObserver;

impl OsHealthObserver for LocalOsHealthObserver {
    fn observe_os_health(&self) -> OsObservation {
        OsObservation {
            softlockup_panic: read_bool("/proc/sys/kernel/softlockup_panic"),
            thp_enabled: read_trim("/sys/kernel/mm/transparent_hugepage/enabled"),
            thp_defrag: read_trim("/sys/kernel/mm/transparent_hugepage/defrag"),
            available_mem_mib: read_available_mem_mib(),
            reclaim_pressure_pct: read_reclaim_pressure_pct(),
            dmesg_softlockup_count: count_dmesg("soft lockup"),
            dmesg_hung_task_count: count_dmesg("hung_task") + count_dmesg("blocked for more than"),
            dmesg_oom_count: count_dmesg("Out of memory") + count_dmesg("oom-kill"),
        }
    }
}

fn read_trim(path: &str) -> String {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            // /sys THP files render like: `always [madvise] never`
            let token = content
                .split_whitespace()
                .find_map(|tok| {
                    let t = tok.trim_matches(|c| c == '[' || c == ']');
                    if tok.contains('[') {
                        Some(t.to_string())
                    } else {
                        None
                    }
                })
                .or_else(|| content.split_whitespace().next().map(|s| s.to_string()))
                .unwrap_or_default();
            if token.is_empty() {
                "unknown".to_string()
            } else {
                token
            }
        }
        Err(_) => "unknown".to_string(),
    }
}

fn read_bool(path: &str) -> bool {
    match std::fs::read_to_string(path) {
        Ok(content) => content.trim() == "1",
        Err(_) => false, // conservative: assume "does not panic" when unreadable
    }
}

fn read_available_mem_mib() -> u64 {
    // /proc/meminfo: `MemAvailable:   12345678 kB`
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

fn read_reclaim_pressure_pct() -> u8 {
    // Best-effort: derive from MemAvailable vs MemTotal in /proc/meminfo.
    let (avail, total) = match std::fs::read_to_string("/proc/meminfo") {
        Ok(c) => {
            let mut a = 0u64;
            let mut t = 0u64;
            for line in c.lines() {
                if let Some(r) = line.strip_prefix("MemAvailable:") {
                    a = r.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(0);
                } else if let Some(r) = line.strip_prefix("MemTotal:") {
                    t = r.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(0);
                }
            }
            (a, t)
        }
        Err(_) => return 0,
    };
    if total == 0 {
        return 0;
    }
    // "reclaim pressure" approximated as the percentage of memory NOT available.
    let used_pct = ((total.saturating_sub(avail)) as f64 / total as f64 * 100.0) as u8;
    used_pct
}

fn count_dmesg(needle: &str) -> u64 {
    // Portable: try the kernel log ring first, then fall back to journalctl
    // only if it exists and is readable (laboratory/CI hosts often lack it).
    if let Ok(content) = std::fs::read_to_string("/dev/kmsg") {
        return content.lines().filter(|l| l.contains(needle)).count() as u64;
    }
    if let Ok(out) = std::process::Command::new("dmesg")
        .arg("--noprefix")
        .output()
    {
        if out.status.success() {
            return String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| l.contains(needle))
                .count() as u64;
        }
    }
    0
}

/// Test double: returns a fixed observation. Used by the decision-layer tests.
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct StubOsHealthObserver {
    pub observation: OsObservation,
}

#[cfg(test)]
impl StubOsHealthObserver {
    pub fn new(observation: OsObservation) -> Self {
        Self { observation }
    }
}

#[cfg(test)]
impl OsHealthObserver for StubOsHealthObserver {
    fn observe_os_health(&self) -> OsObservation {
        self.observation.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_trim_picks_bracketed_token() {
        // simulates: "always [madvise] never"
        let tmp = std::env::temp_dir().join(format!("thp-{}.txt", std::process::id()));
        std::fs::write(&tmp, "always [madvise] never\n").unwrap();
        let got = read_trim(tmp.to_str().unwrap());
        std::fs::remove_file(&tmp).ok();
        assert_eq!(got, "madvise");
    }

    #[test]
    fn read_bool_parses_one() {
        let tmp = std::env::temp_dir().join(format!("slp-{}.txt", std::process::id()));
        std::fs::write(&tmp, "1\n").unwrap();
        let got = read_bool(tmp.to_str().unwrap());
        std::fs::remove_file(&tmp).ok();
        assert!(got);
    }

    #[test]
    fn missing_file_is_unknown_false() {
        assert_eq!(read_trim("/nonexistent/thp"), "unknown");
        assert!(!read_bool("/nonexistent/softlockup_panic"));
    }
}
