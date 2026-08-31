//! Pure parsers for the memory vertical — no I/O, fully deterministic.
//!
//! Each parser takes raw substrate text (from /proc/meminfo, `free -b`, or
//! /proc/pressure/memory) and returns the canonical protocol types. They are
//! testable without a host and reused by both LocalMemoryAdapter and
//! RemoteMemoryAdapter (SSH).

use crate::domain::memory::types::{MemoryPressure, MemoryReport, MemoryUsage, MemoryVolume};

fn parse_kb_line(key: &str, lines: &[&str]) -> Option<u64> {
    for line in lines {
        let mut parts = line.split_whitespace();
        // /proc/meminfo keys are "MemTotal:" with a trailing colon; strip it so
        // the lookup key stays the bare field name ("MemTotal"). Regression
        // proof: parse_meminfo_basic_layers fails before this fix.
        let first = parts.next()?.trim_end_matches(':');
        if first == key {
            let val: u64 = parts.next()?.parse().ok()?;
            // /proc/meminfo is in kB; convert to bytes.
            return Some(val * 1024);
        }
    }
    None
}

/// Parse `/proc/meminfo` text into a 4-layer MemoryReport.
pub fn parse_meminfo(text: &str) -> Result<MemoryReport, String> {
    let lines: Vec<&str> = text.lines().collect();
    let total = parse_kb_line("MemTotal", &lines).ok_or("MemTotal missing")?;
    let free = parse_kb_line("MemFree", &lines).unwrap_or(0);
    let available = parse_kb_line("MemAvailable", &lines).unwrap_or(free);
    let buffers = parse_kb_line("Buffers", &lines).unwrap_or(0);
    let cached = parse_kb_line("Cached", &lines).unwrap_or(0);
    let sreclaim = parse_kb_line("SReclaimable", &lines).unwrap_or(0);
    let mem_used = total
        .saturating_sub(free)
        .saturating_sub(buffers)
        .saturating_sub(cached);
    let reclaimable = cached + sreclaim;

    let swap_total = parse_kb_line("SwapTotal", &lines).unwrap_or(0);
    let swap_free = parse_kb_line("SwapFree", &lines).unwrap_or(0);
    let swap_used = swap_total.saturating_sub(swap_free);

    let headline = lines.first().map(|l| l.trim().to_string());

    let physical = MemoryUsage {
        total_bytes: total,
        free_bytes: free,
        available_bytes: available,
        used_bytes: mem_used,
        reclaimable_bytes: reclaimable,
    };
    let volume = MemoryVolume {
        page_size_bytes: 4096,
        swappiness: None,
        hugepages_reserved: None,
        hugepage_size_bytes: None,
    };
    let swap_usage = MemoryUsage {
        total_bytes: swap_total,
        free_bytes: swap_free,
        available_bytes: swap_free,
        used_bytes: swap_used,
        reclaimable_bytes: 0,
    };

    Ok(MemoryReport {
        schema: "cynic.host_memory_report.v1".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        physical,
        volume,
        swap: swap_usage,
        pressure: None,
        meminfo_headline: headline,
        residual_unknowns: Vec::new(),
    })
}

/// Parse `free -b` (bytes) output into MemoryUsage + swap. Sample:
/// ```text
///                total        used        free      shared  buff/cache   available
/// Mem:     29308112896  8558247936  5606895616  2206539776 17598664704 20749864960
/// Swap:    8589930496    88518656  8501411840
/// ```
pub fn parse_free_output(text: &str) -> Result<(MemoryUsage, MemoryUsage), String> {
    let mut mem = None;
    let mut swp = None;
    for line in text.lines() {
        let cols: Vec<u64> = line
            .split_whitespace()
            .filter_map(|s| s.parse::<u64>().ok())
            .collect();
        if line.starts_with("Mem:") && cols.len() >= 6 {
            mem = Some(MemoryUsage {
                total_bytes: cols[0],
                free_bytes: cols[2],
                available_bytes: cols[5],
                used_bytes: cols[1],
                reclaimable_bytes: cols[4],
            });
        } else if line.starts_with("Swap:") && cols.len() >= 3 {
            swp = Some(MemoryUsage {
                total_bytes: cols[0],
                free_bytes: cols[2],
                available_bytes: cols[2],
                used_bytes: cols[1],
                reclaimable_bytes: 0,
            });
        }
    }
    let mem = mem.ok_or("Mem: line missing")?;
    let swp = swp.unwrap_or(MemoryUsage {
        total_bytes: 0,
        free_bytes: 0,
        available_bytes: 0,
        used_bytes: 0,
        reclaimable_bytes: 0,
    });
    Ok((mem, swp))
}

/// Parse `/proc/pressure/memory` into MemoryPressure. Lines:
/// ```text
/// some avg10=0.00 avg60=0.07 avg300=0.06 total=1246798
/// full avg10=0.00 avg60=0.07 avg300=0.06 total=1242169
/// ```
pub fn parse_pressure_memory(text: &str) -> Result<MemoryPressure, String> {
    let parse_avg = |key: &str, label: &str| -> Result<f64, String> {
        for line in text.lines() {
            if line.starts_with(label) {
                if let Some(v) = line.split_whitespace().find_map(|tok| {
                    tok.strip_prefix(key)
                        .and_then(|s| s.strip_prefix("="))
                        .and_then(|n| n.parse::<f64>().ok())
                }) {
                    return Ok(v);
                }
            }
        }
        Ok(0.0)
    };

    let some = |key: &str| parse_avg(key, "some").unwrap_or(0.0);
    let full = |key: &str| parse_avg(key, "full").unwrap_or(0.0);

    let some_avg10 = some("avg10");
    let some_avg60 = some("avg60");
    let some_avg300 = some("avg300");
    let full_avg10 = full("avg10");
    let full_avg60 = full("avg60");
    let full_avg300 = full("avg300");

    let parse_total = |label: &str, field: &str| -> u64 {
        for line in text.lines() {
            if line.starts_with(label) {
                for tok in line.split_whitespace() {
                    if let Some(n) = tok
                        .strip_prefix(field)
                        .and_then(|s| s.strip_prefix("="))
                        .and_then(|n| n.parse::<u64>().ok())
                    {
                        return n;
                    }
                }
            }
        }
        0
    };
    let some_total = parse_total("some", "total");
    let full_total = parse_total("full", "total");

    Ok(MemoryPressure {
        some_avg10,
        some_avg60,
        some_avg300,
        some_total_us: some_total,
        full_avg10,
        full_avg60,
        full_avg300,
        full_total_us: full_total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MEMINFO: &str = "\
MemTotal:       28621204 kB
MemFree:         5475484 kB
MemAvailable:   20263524 kB
Buffers:          552700 kB
Cached:         14832604 kB
SwapCached:          192 kB
SwapTotal:       8589930496 kB
SwapFree:         4711940 kB
Shmem:          20911964 kB
SReclaimable:     807936 kB
";

    #[test]
    fn parse_meminfo_basic_layers() {
        let rep = parse_meminfo(SAMPLE_MEMINFO).unwrap();
        assert_eq!(rep.schema, "cynic.host_memory_report.v1");
        assert_eq!(rep.physical.total_bytes, 28621204 * 1024);
        assert_eq!(rep.physical.free_bytes, 5475484 * 1024);
        assert_eq!(rep.physical.available_bytes, 20263524 * 1024);
        // Swap layer (level 3)
        assert_eq!(rep.swap.total_bytes, 8589930496 * 1024);
        assert_eq!(rep.swap.free_bytes, 4711940 * 1024);
        // used = total - free - buffers - cached
        let expected_used = (28621204 - 5475484 - 552700 - 14832604) * 1024;
        assert_eq!(rep.physical.used_bytes, expected_used);
        assert_eq!(
            rep.meminfo_headline.as_deref(),
            Some("MemTotal:       28621204 kB")
        );
    }

    const SAMPLE_FREE: &str = "\
               total        used        free      shared  buff/cache   available
Mem:     29308112896  8558247936  5606895616  2206539776 17598664704 20749864960
Swap:    8589930496    88518656  8501411840
";

    #[test]
    fn parse_free_output_columns() {
        let (mem, swp) = parse_free_output(SAMPLE_FREE).unwrap();
        assert_eq!(mem.total_bytes, 29308112896);
        assert_eq!(mem.used_bytes, 8558247936);
        assert_eq!(mem.free_bytes, 5606895616);
        assert_eq!(mem.reclaimable_bytes, 17598664704); // buff/cache
        assert_eq!(mem.available_bytes, 20749864960);
        assert_eq!(swp.total_bytes, 8589930496);
        assert_eq!(swp.free_bytes, 8501411840);
        assert_eq!(swp.used_bytes, 88518656);
    }

    #[test]
    fn parse_free_without_swap() {
        let no_swp = "Mem:     1000      400       600         0        0      600\n";
        let (mem, swp) = parse_free_output(no_swp).unwrap();
        assert_eq!(mem.total_bytes, 1000);
        assert_eq!(swp.total_bytes, 0);
        assert_eq!(swp.used_bytes, 0);
    }

    const SAMPLE_PRESSURE: &str = "\
some avg10=0.00 avg60=0.07 avg300=0.06 total=1246798
full avg10=0.00 avg60=0.07 avg300=0.06 total=1242169
";

    #[test]
    fn parse_pressure_memory_values() {
        let p = parse_pressure_memory(SAMPLE_PRESSURE).unwrap();
        assert_eq!(p.some_avg10, 0.0);
        assert_eq!(p.some_avg60, 0.07);
        assert_eq!(p.some_avg300, 0.06);
        assert_eq!(p.some_total_us, 1246798);
        assert_eq!(p.full_avg60, 0.07);
        assert_eq!(p.full_total_us, 1242169);
    }
}
