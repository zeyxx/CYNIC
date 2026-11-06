//! Pure parsers for process-level self-monitoring — no I/O, fully deterministic.
//!
//! Takes raw `/proc/self` text contents and parses them without syscalls
//! or global state. Reusable across kernel self-monitoring and host adapters.

/// Parse `VmRSS:` from `/proc/self/status` (in kB → MB).
pub fn parse_proc_status_rss(text: &str) -> Option<f64> {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: f64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb / 1024.0);
        }
    }
    None
}

/// Parse `Threads:` count from `/proc/self/status`.
pub fn parse_proc_status_threads(text: &str) -> Option<u64> {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Threads:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

/// Parse soft `RLIMIT_NOFILE` from `/proc/self/limits`.
pub fn parse_proc_limits_nofile(text: &str) -> Option<u64> {
    for line in text.lines() {
        if line.starts_with("Max open files") {
            // Format: "Max open files            1048576              1048576              files"
            let fields: Vec<&str> = line.split_whitespace().collect();
            return fields.get(3)?.parse().ok();
        }
    }
    None
}

/// Parse `utime` and `stime` from `/proc/self/stat` (fields 14 and 15, 1-indexed).
/// Returns (user_seconds, system_seconds) assuming standard 100 Hz tick rate.
pub fn parse_proc_stat_cpu_times(text: &str) -> Option<(f64, f64)> {
    let right_paren = text.rfind(')')?;
    let rest = text.get(right_paren + 2..)?;
    let fields: Vec<&str> = rest.split_whitespace().collect();

    // Fields after ')' are 0-indexed; field 14 in raw stat is index 11 here (14 - 3).
    let utime_ticks: f64 = fields.get(11)?.parse().ok()?;
    let stime_ticks: f64 = fields.get(12)?.parse().ok()?;

    let hz = 100.0;
    Some((utime_ticks / hz, stime_ticks / hz))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_extracts_rss_and_threads() {
        let status = "Name:\tcynic-kernel\nState:\tR (running)\nVmRSS:\t   16384 kB\nThreads:\t8\n";
        assert_eq!(parse_proc_status_rss(status), Some(16.0));
        assert_eq!(parse_proc_status_threads(status), Some(8));
    }

    #[test]
    fn parse_limits_extracts_nofile_soft_limit() {
        let limits = "Limit                     Soft Limit           Hard Limit           Units     \nMax open files            1048576              1048576              files     \n";
        assert_eq!(parse_proc_limits_nofile(limits), Some(1048576));
    }

    #[test]
    fn parse_stat_extracts_cpu_times() {
        let stat = "12345 (cynic-kernel) S 1 12345 12345 0 -1 4194304 100 0 0 0 250 150 0 0 20 0 8 0 1000 1048576 4096 18446744073709551615 0 0 0 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0 0 0 0 0 0 0 0 0";
        let (user_s, sys_s) = parse_proc_stat_cpu_times(stat).expect("parse cpu times");
        assert_eq!(user_s, 2.5);
        assert_eq!(sys_s, 1.5);
    }
}
