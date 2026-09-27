//! Pure parsers over Linux procfs/sysfs text (`/proc/meminfo`, `/proc/cpuinfo`).
//!
//! Zero I/O: every function takes already-read `&str` and returns plain data.
//! Host adapters read the files; domain crates decide from the parsed values.
//! Shared here (Tier 1 leaf) so host probes and inference planners cannot drift.

/// Parse a `<number>[unit]` field value as kilobytes (returns 0 when absent).
pub fn parse_kb_value(field: &str) -> u64 {
    field
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Parse `/proc/meminfo` text into `(total_bytes, available_bytes)`.
///
/// Missing lines yield 0 — callers apply their own documented fallback.
pub fn parse_meminfo_bytes(raw: &str) -> (u64, u64) {
    let mut total_kb = 0u64;
    let mut avail_kb = 0u64;
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total_kb = parse_kb_value(v);
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            avail_kb = parse_kb_value(v);
        }
    }
    (total_kb * 1024, avail_kb * 1024)
}

/// Parse the `flags : ...` line of `/proc/cpuinfo` text into flag tokens.
pub fn parse_cpu_flags(raw: &str) -> Vec<String> {
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("flags")
            && let Some(flags) = rest.split(':').nth(1)
        {
            return flags.split_whitespace().map(|s| s.to_string()).collect();
        }
    }
    Vec::new()
}

/// Normalize a cpufreq governor string (`None` when blank).
pub fn parse_governor(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Parse one `<Field>:` line of procfs text as kilobytes (`None` when absent).
///
/// Generic primitive for `/proc/meminfo` and `/proc/self/status` alike —
/// host adapters read the files, every crate parses through this one function.
pub fn parse_proc_field_kb(raw: &str, field: &str) -> Option<u64> {
    raw.lines().find_map(|line| {
        line.strip_prefix(field)?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meminfo_lines_become_bytes() {
        let raw = "MemTotal:       16384000 kB\nMemAvailable:    8192000 kB\n";
        assert_eq!(parse_meminfo_bytes(raw), (16384000 * 1024, 8192000 * 1024));
    }

    #[test]
    fn meminfo_missing_lines_yield_zero() {
        assert_eq!(parse_meminfo_bytes(""), (0, 0));
    }

    #[test]
    fn cpuinfo_flags_tokenize() {
        let raw = "processor\t: 0\nflags\t\t: fpu vme avx2 fma\n";
        assert_eq!(parse_cpu_flags(raw), vec!["fpu", "vme", "avx2", "fma"]);
    }

    #[test]
    fn governor_trims_and_rejects_blank() {
        assert_eq!(parse_governor("schedutil\n"), Some("schedutil".into()));
        assert_eq!(parse_governor("  \n"), None);
    }

    #[test]
    fn proc_field_kb_reads_any_prefixed_line() {
        let raw = "Name:\tcynic\nVmRSS:\t   16384 kB\nThreads:\t8\n";
        assert_eq!(parse_proc_field_kb(raw, "VmRSS:"), Some(16384));
        assert_eq!(parse_proc_field_kb(raw, "Missing:"), None);
    }
}
