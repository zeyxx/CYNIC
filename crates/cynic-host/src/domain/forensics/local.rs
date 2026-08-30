//! Local forensic observation adapter.
//!
//! Reads real crash evidence from the host using argv-based external calls
//! (`journalctl`, `systemctl`) and direct filesystem reads (`/proc`,
//! `/var/crash`). All evidence is parsed via the pure parsers in `probe.rs`.
//!
//! This adapter follows the same pattern as `LocalCpuAdapter` and
//! `LocalMemoryAdapter`: stateless reads on each call, no caching, best-effort
//! collection where a missing source becomes a residual unknown rather than a
//! hard failure.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::domain::forensics::port::{ForensicError, ForensicObservationPort};
use crate::domain::forensics::types::{
    CrashKind, EvidenceSource, FORENSIC_TIMELINE_SCHEMA, ForensicEvidence, ForensicTimeline,
    PostMortemReport, ServiceLoop, TimelineEntry,
};
use crate::domain::forensics::{
    parse_boot_id, parse_boot_table_line, parse_crash_dump_header, parse_dmesg_gpu_fault,
    parse_kernel_trap, parse_service_failed_line,
};

/// Local forensic adapter. Stateless — reads host evidence on each call.
pub struct LocalForensicAdapter;

impl LocalForensicAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Read the current boot_id from `/proc/sys/kernel/random/boot_id`.
    pub fn read_current_boot_id() -> Option<String> {
        let content = fs::read_to_string(Path::new("/proc/sys/kernel/random/boot_id")).ok()?;
        parse_boot_id(&content)
    }

    /// Read journalctl output for a given boot (`-k` for kernel only).
    fn read_journal(boot_id: &str, kernel_only: bool) -> Option<String> {
        let mut cmd = Command::new("journalctl");
        cmd.arg("--boot").arg(boot_id);
        if kernel_only {
            cmd.arg("-k");
        }
        let output = cmd.output().ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    }

    /// Run `systemctl --failed` and return raw output.
    fn read_systemctl_failed() -> Option<String> {
        let output = Command::new("systemctl")
            .arg("--failed")
            .arg("--no-pager")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    }

    /// List `/var/crash/*.crash` files that are readable by the current user.
    fn list_crash_dumps() -> Vec<PathBuf> {
        let crash_dir = Path::new("/var/crash");
        let mut dumps = Vec::new();
        if let Ok(entries) = fs::read_dir(crash_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("crash") {
                    // Only include if user-readable (best-effort check)
                    if fs::metadata(&path).is_ok() {
                        dumps.push(path);
                    }
                }
            }
        }
        dumps
    }

    /// Read up to `limit` bytes from the head of a file.
    fn read_file_head(path: &Path, limit: usize) -> Option<String> {
        let content = fs::read_to_string(path).ok()?;
        if content.len() > limit {
            Some(content[..limit].to_string())
        } else {
            Some(content)
        }
    }

    /// Run `journalctl --list-boots` and return raw output.
    pub fn read_boot_table() -> Option<String> {
        let output = Command::new("journalctl")
            .arg("--list-boots")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    }

    /// Get uptime in seconds from `/proc/uptime`.
    pub fn read_uptime_secs() -> Option<u64> {
        let content = fs::read_to_string(Path::new("/proc/uptime")).ok()?;
        let first_word = content.split_whitespace().next()?;
        let secs: f64 = first_word.parse().ok()?;
        Some(secs as u64)
    }
}

impl Default for LocalForensicAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ForensicObservationPort for LocalForensicAdapter {
    fn collect_evidence(&self, boot_id: &str) -> Result<Vec<ForensicEvidence>, ForensicError> {
        let mut evidence = Vec::new();

        // 1. Kernel evidence from journalctl --boot <id> -k
        if let Some(journal) = Self::read_journal(boot_id, true) {
            for line in journal.lines() {
                if let Some(e) = parse_dmesg_gpu_fault(line) {
                    evidence.push(e);
                } else if let Some(e) = parse_kernel_trap(line) {
                    evidence.push(e);
                }
            }
        }

        // 2. Service failure evidence from journalctl --boot <id>
        if let Some(journal) = Self::read_journal(boot_id, false) {
            for line in journal.lines() {
                if let Some(_loop) = parse_service_failed_line(line) {
                    evidence.push(ForensicEvidence {
                        source: EvidenceSource::SystemdFailed {
                            unit: _loop.unit.clone(),
                            timestamp: 0,
                        },
                        kind: CrashKind::ServiceCascade,
                        snippet: line.trim().to_string(),
                        severity: 50,
                    });
                }
            }
        }

        // 3. Crash dump evidence from /var/crash/*.crash
        for dump_path in Self::list_crash_dumps() {
            if let Some(content) = Self::read_file_head(&dump_path, 4096) {
                if let Some((executable, _date)) = parse_crash_dump_header(&content) {
                    let metadata = fs::metadata(&dump_path).ok();
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    let mtime = metadata
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    evidence.push(ForensicEvidence {
                        source: EvidenceSource::CoreDump {
                            path: dump_path.to_string_lossy().to_string(),
                            size,
                            executable,
                            mtime,
                        },
                        kind: CrashKind::Unknown,
                        snippet: format!("crash dump: {}", dump_path.display()),
                        severity: 60,
                    });
                }
            }
        }

        Ok(evidence)
    }

    fn timeline(&self, boot_id: &str) -> Result<ForensicTimeline, ForensicError> {
        let evidence = self.collect_evidence(boot_id)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let entries: Vec<TimelineEntry> = evidence
            .into_iter()
            .enumerate()
            .map(|(i, e)| TimelineEntry {
                boot_id: boot_id.to_string(),
                offset_secs: i as f64,
                evidence: e,
            })
            .collect();

        Ok(ForensicTimeline {
            schema: FORENSIC_TIMELINE_SCHEMA.to_string(),
            boot_id: boot_id.to_string(),
            entries,
            generated_at: now,
        })
    }

    fn post_mortem(&self, boot_id: &str) -> Result<PostMortemReport, ForensicError> {
        let evidence = self.collect_evidence(boot_id)?;
        let service_loops = self.service_loops()?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Classify crash from collected evidence
        let gpu_fault_count = evidence
            .iter()
            .filter(|e| matches!(e.kind, CrashKind::GpuHang))
            .count() as u32;
        let softlockup_trap_count = evidence
            .iter()
            .filter(|e| matches!(e.kind, CrashKind::SoftLockup))
            .count() as u32;
        let cynic_canary_disconnections = evidence
            .iter()
            .filter(|e| matches!(e.source, EvidenceSource::CynicCanary { .. }))
            .count() as u32;

        let core_dumps: Vec<String> = evidence
            .iter()
            .filter_map(|e| match &e.source {
                EvidenceSource::CoreDump { path, .. } => Some(path.clone()),
                _ => None,
            })
            .collect();

        let dev_coredumps: Vec<String> = evidence
            .iter()
            .filter_map(|e| match &e.source {
                EvidenceSource::DevCoredump { path, .. } => Some(path.clone()),
                _ => None,
            })
            .collect();

        // Determine crash kind from evidence
        let crash = if gpu_fault_count > 0 {
            Some(CrashKind::GpuHang)
        } else if softlockup_trap_count > 0 {
            Some(CrashKind::SoftLockup)
        } else if !service_loops.is_empty() {
            Some(CrashKind::ServiceCascade)
        } else {
            None
        };

        // Find previous boot from boot table
        let previous_boot_id = Self::read_boot_table().and_then(|table| {
            let mut prev: Option<String> = None;
            for line in table.lines() {
                if let Some((bid, _, _)) = parse_boot_table_line(line) {
                    if bid == boot_id {
                        return prev;
                    }
                    prev = Some(bid);
                }
            }
            None
        });

        let uptime_before_crash_secs = Self::read_uptime_secs();

        Ok(PostMortemReport {
            boot_id: boot_id.to_string(),
            previous_boot_id,
            uptime_before_crash_secs,
            crash,
            gpu_fault_count,
            softlockup_trap_count,
            service_loops,
            cynic_canary_disconnections,
            core_dumps,
            dev_coredumps,
            generated_at: now,
        })
    }

    fn service_loops(&self) -> Result<Vec<ServiceLoop>, ForensicError> {
        let mut loops = Vec::new();

        // Use systemctl --failed to find active crash loops
        if let Some(output) = Self::read_systemctl_failed() {
            for line in output.lines() {
                // systemctl --failed output format:
                // UNIT                    LOAD   ACTIVE SUB     DESCRIPTION
                // discord-pac-server.service loaded failed failed Discord PAC Server
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with("UNIT") || trimmed.starts_with("LOAD")
                {
                    continue;
                }
                if let Some(unit) = trimmed.split_whitespace().next() {
                    if unit.ends_with(".service") {
                        let unit_name = unit.trim_end_matches(".service").to_string();
                        loops.push(ServiceLoop {
                            unit: unit_name,
                            restart_count: 0,
                            exit_status: 0,
                            window_secs: 3600,
                        });
                    }
                }
            }
        }

        // Also scan journal for service failure lines
        if let Some(boot_id) = Self::read_current_boot_id() {
            if let Some(journal) = Self::read_journal(&boot_id, false) {
                for line in journal.lines() {
                    if let Some(sl) = parse_service_failed_line(line) {
                        loops.push(sl);
                    }
                }
            }
        }

        Ok(loops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: ensure the adapter binds the port trait.
    fn _assert_port<T: ForensicObservationPort>() {}
    #[test]
    fn local_forensic_adapts_observation_port() {
        _assert_port::<LocalForensicAdapter>();
    }
}
