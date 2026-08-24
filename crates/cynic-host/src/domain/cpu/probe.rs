//! Pure CPU parsers (no I/O). Each takes a captured substrate string and
//! returns typed layers of `cynic_protocol::cpu`. Unit tests are deterministic
//! fixtures, not host reads.

use crate::domain::cpu::{CpuPhysical, CpuPressure, CpuUsage, CpuVolume};

/// Parse `/proc/cpuinfo` into the physical + volume layer pieces.
///
/// `/proc/cpuinfo` repeats a `processor : N` block per logical CPU; we count
/// blocks for `logical_cpus`, and take `model name` / `cpu cores` from the
/// first block. Frequency (bogomips) is observed but treated as auxiliary
/// evidence (not an authority for core count).
pub fn parse_cpuinfo(text: &str) -> (CpuPhysical, CpuVolume, Vec<String>) {
    let mut unknowns: Vec<String> = Vec::new();
    let mut model_name = String::from("unknown");
    let mut logical_cpus = 0usize;
    let mut cores: Option<usize> = None;
    let mut sockets: Option<usize> = None;
    let mut bogomips: Option<f64> = None;

    let mut in_first_block = true;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            // blank line separates cpuinfo "processor" blocks.
            in_first_block = false;
            continue;
        }
        // Only count "processor :" entries as logical CPU markers; the
        // header field repeats once per block.
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "processor" if in_first_block => {
                    // logical_cpus is counted below via blocks.
                }
                "processor" => {
                    logical_cpus += 1;
                }
                "model name" if model_name == "unknown" => {
                    model_name = value.to_string();
                }
                "cpu cores" => {
                    cores = value.parse::<usize>().ok();
                }
                "physical id" => {
                    // physical id increments per socket; track max+1.
                    if let Ok(id) = value.parse::<usize>() {
                        sockets = Some(sockets.map_or(id + 1, |s: usize| s.max(id + 1)));
                    }
                }
                "bogomips" => {
                    bogomips = value.parse::<f64>().ok();
                }
                _ => {}
            }
        }
    }
    // The first CPU has processor=0 counted as a block too — recompute count
    // robustly: number of "processor :" lines.
    let re_count = text.match_indices("processor\t:").count().max(
        text.match_indices("processor :").count(),
    );
    if re_count != 0 {
        logical_cpus = re_count;
    }
    if cores.is_none() {
        unknowns.push("cpu_cores_not_observed".to_string());
    }
    if sockets.is_none() {
        unknowns.push("sockets_not_observed".to_string());
    }

    let physical = CpuPhysical {
        model_name,
        logical_cpus,
        cores,
        sockets,
        max_bogomips_or_hz: bogomips,
    };
    let volume = CpuVolume {
        governor: None,
        isolated_mask: None,
        online_cpus: logical_cpus,
        possible_cpus: logical_cpus,
    };
    (physical, volume, unknowns)
}

/// Parse `/proc/pressure/cpu` (only the `some` line; cpu PSI has no `full`).
pub fn parse_pressure_cpu(text: &str) -> Option<CpuPressure> {
    let mut some_avg10: Option<f64> = None;
    let mut softirq_steal: Option<u64> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("some ") {
            some_avg10 = rest.split_whitespace().find_map(|t| {
                t.strip_prefix("avg10=").and_then(|s| s.parse::<f64>().ok())
            });
            softirq_steal = rest.split_whitespace().find_map(|t| {
                t.strip_prefix("total=").and_then(|s| s.parse::<u64>().ok())
            });
        }
    }
    if some_avg10.is_none() && softirq_steal.is_none() {
        return None;
    }
    Some(CpuPressure {
        some_avg10,
        // cpu PSI has no "full" line.
        full_avg10: None,
        softirq_steal,
    })
}

/// Parse `/proc/stat` aggregate `cpu` line into jiffies (level 3 usage).
///
/// Line shape: `cpu  user nice system idle iowait irq softirq steal guest guest_nice`
pub fn parse_stat(text: &str) -> Option<CpuUsage> {
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with("cpu ") {
            continue;
        }
        let fields: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|f| f.parse::<u64>().ok())
            .collect();
        if fields.len() < 4 {
            return None;
        }
        return Some(CpuUsage {
            user: fields[0],
            system: fields[2],
            idle: fields[3],
            iowait: fields.get(4).copied().unwrap_or(0),
            softirq: fields.get(6).copied().unwrap_or(0),
            steal: fields.get(7).copied().unwrap_or(0),
            util_pct: None,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CPUINFO: &str = "\
processor\t: 0
vendor_id\t: AuthenticAMD
cpu family\t: 25
model\t: 1
model name\t: AMD EPYC 7R13
stepping\t: 1
cpu MHz\t: 2445.360
cache size\t: 512 KB
physical id\t: 0
siblings\t: 2
core id\t: 0
cpu cores\t: 1
...
processor\t: 1
vendor_id\t: AuthenticAMD
model name\t: AMD EPYC 7R13
cpu MHz\t: 2445.360
cache size\t: 512 KB
physical id\t: 0
core id\t: 0
cpu cores\t: 1
";

    const SAMPLE_STAT: &str = "\
cpu  9432416 1234 3215683 47561286 156789 0 223456 100 0 0
intr 123456789
ctxt 987654321
";

    const SAMPLE_PRESSURE: &str = "\
some avg10=5.23 avg60=3.14 avg300=1.00 total=9432416
full avg10=4.50 avg60=2.00 avg300=0.10 total=123456
";

    #[test]
    fn parse_meminfo_counts_two_logical_cpus() {
        let (physical, _, _) = parse_cpuinfo(SAMPLE_CPUINFO);
        assert_eq!(physical.model_name, "AMD EPYC 7R13");
        assert_eq!(physical.logical_cpus, 2);
    }

    #[test]
    fn parse_stat_extracts_jiffies() {
        let usage = parse_stat(SAMPLE_STAT).expect("cpu line parsed");
        assert_eq!(usage.user, 9432416);
        assert_eq!(usage.system, 3215683);
        assert_eq!(usage.idle, 47561286);
        assert_eq!(usage.iowait, 156789);
        assert_eq!(usage.steal, 100);
    }

    #[test]
    fn parse_pressure_cpu_some_and_total() {
        let p = parse_pressure_cpu(SAMPLE_PRESSURE).expect("psi parsed");
        assert_eq!(p.some_avg10, Some(5.23));
        // /proc/pressure/cpu has no "full" line.
        assert_eq!(p.full_avg10, None);
        assert_eq!(p.softirq_steal, Some(9432416));
    }
}
