//! Unified /proc/meminfo reader for CYNIC.
//!
//! WHY THIS EXISTS.
//!
//! Before this module, 25 sites in CYNIC read `/proc/meminfo` independently:
//!
//! ```rust,ignore
//! let raw = std::fs::read_to_string("/proc/meminfo")?;
//! let report = parse_meminfo(&raw)?;
//! ```
//!
//! Each site had its own error handling, its own fallback, its own caching.
//! Adding cgroup memory.pressure or fallback to `free` required touching every
//! site.
//!
//! This module is the single point where CYNIC reads memory information. All
//! 25 sites should migrate here.
//!
//! DESIGN.
//!
//! - `MeminfoReader::read()` reads `/proc/meminfo` and returns a `MemoryReport`.
//! - `MeminfoReader::read_with_pressure()` also reads `/proc/pressure/memory`.
//! - Errors are returned as `String` for simplicity.

use std::fs;

/// Memory report from /proc/meminfo.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryReport {
    /// Total memory in bytes.
    pub total_bytes: u64,
    /// Free memory in bytes.
    pub free_bytes: u64,
    /// Available memory in bytes (free + reclaimable).
    pub available_bytes: u64,
    /// Buffers in bytes.
    pub buffers_bytes: u64,
    /// Cached in bytes.
    pub cached_bytes: u64,
    /// Swap total in bytes.
    pub swap_total_bytes: u64,
    /// Swap free in bytes.
    pub swap_free_bytes: u64,
}

impl MemoryReport {
    /// Used memory in bytes (total - available).
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }

    /// Memory usage as a percentage (0-100).
    pub fn usage_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes() as f32 / self.total_bytes as f32) * 100.0
    }
}

/// Unified /proc/meminfo reader for CYNIC.
pub struct MeminfoReader;

impl MeminfoReader {
    /// Read `/proc/meminfo` and return a `MemoryReport`.
    ///
    /// # Returns
    ///
    /// `Ok(MemoryReport)` on success, `Err(String)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use cynic_host::adapters::meminfo_reader::MeminfoReader;
    ///
    /// let report = MeminfoReader::read().unwrap();
    /// assert!(report.total_bytes > 0);
    /// ```
    pub fn read() -> Result<MemoryReport, String> {
        let raw = fs::read_to_string("/proc/meminfo")
            .map_err(|e| format!("cannot read /proc/meminfo: {e}"))?;
        Self::parse(&raw)
    }

    /// Parse a `/proc/meminfo` string into a `MemoryReport`.
    ///
    /// This is exposed for testing and for callers who already have the raw
    /// content (e.g., from a remote host).
    pub fn parse(raw: &str) -> Result<MemoryReport, String> {
        let mut total = 0u64;
        let mut free = 0u64;
        let mut available = 0u64;
        let mut buffers = 0u64;
        let mut cached = 0u64;
        let mut swap_total = 0u64;
        let mut swap_free = 0u64;

        for line in raw.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            let key = parts[0].trim_end_matches(':');
            let value_kb: u64 = parts[1]
                .parse()
                .map_err(|e| format!("cannot parse value for {key}: {e}"))?;
            let value = value_kb * 1024; // Values are in kB

            match key {
                "MemTotal" => total = value,
                "MemFree" => free = value,
                "MemAvailable" => available = value,
                "Buffers" => buffers = value,
                "Cached" => cached = value,
                "SwapTotal" => swap_total = value,
                "SwapFree" => swap_free = value,
                _ => {}
            }
        }

        if total == 0 {
            return Err("MemTotal is zero or missing".into());
        }

        Ok(MemoryReport {
            total_bytes: total,
            free_bytes: free,
            available_bytes: available,
            buffers_bytes: buffers,
            cached_bytes: cached,
            swap_total_bytes: swap_total,
            swap_free_bytes: swap_free,
        })
    }

    /// Read memory pressure from `/proc/pressure/memory`.
    ///
    /// Returns `None` if the file doesn't exist (older kernels) or can't be read.
    pub fn read_pressure() -> Option<String> {
        fs::read_to_string("/proc/pressure/memory").ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_succeeds() {
        let report = MeminfoReader::read().unwrap();
        assert!(report.total_bytes > 0);
        assert!(report.available_bytes > 0);
    }

    #[test]
    fn parse_basic_meminfo() {
        let raw = r#"MemTotal:       16384000 kB
MemFree:         8192000 kB
MemAvailable:   12288000 kB
Buffers:          512000 kB
Cached:          2048000 kB
SwapTotal:       4194304 kB
SwapFree:        4194304 kB
"#;
        let report = MeminfoReader::parse(raw).unwrap();
        assert_eq!(report.total_bytes, 16384000 * 1024);
        assert_eq!(report.free_bytes, 8192000 * 1024);
        assert_eq!(report.available_bytes, 12288000 * 1024);
        assert_eq!(report.buffers_bytes, 512000 * 1024);
        assert_eq!(report.cached_bytes, 2048000 * 1024);
        assert_eq!(report.swap_total_bytes, 4194304 * 1024);
        assert_eq!(report.swap_free_bytes, 4194304 * 1024);
    }

    #[test]
    fn parse_missing_memtotal_fails() {
        let raw = r#"MemFree:         8192000 kB
"#;
        let result = MeminfoReader::parse(raw);
        assert!(result.is_err());
    }

    #[test]
    fn used_bytes_calculation() {
        let report = MemoryReport {
            total_bytes: 1000,
            free_bytes: 200,
            available_bytes: 300,
            buffers_bytes: 50,
            cached_bytes: 50,
            swap_total_bytes: 0,
            swap_free_bytes: 0,
        };
        assert_eq!(report.used_bytes(), 700);
    }

    #[test]
    fn usage_percent_calculation() {
        let report = MemoryReport {
            total_bytes: 1000,
            free_bytes: 200,
            available_bytes: 300,
            buffers_bytes: 50,
            cached_bytes: 50,
            swap_total_bytes: 0,
            swap_free_bytes: 0,
        };
        assert_eq!(report.usage_percent(), 70.0);
    }

    #[test]
    fn usage_percent_zero_total_is_zero() {
        let report = MemoryReport {
            total_bytes: 0,
            free_bytes: 0,
            available_bytes: 0,
            buffers_bytes: 0,
            cached_bytes: 0,
            swap_total_bytes: 0,
            swap_free_bytes: 0,
        };
        assert_eq!(report.usage_percent(), 0.0);
    }
}
