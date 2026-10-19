//! Pure parsers for the forensics vertical — no I/O, fully deterministic.

use crate::types::{CrashKind, EvidenceSource, ForensicEvidence, ServiceLoop};

/// Parse a dmesg/journalctl line for GPU fault signatures.
pub fn parse_dmesg_gpu_fault(line: &str) -> Option<ForensicEvidence> {
    let lower = line.to_lowercase();
    let matched = lower.contains("amdgpu") && lower.contains("page fault")
        || lower.contains("ring gfx timeout")
        || lower.contains("reset failed");
    if !matched {
        return None;
    }
    Some(ForensicEvidence {
        source: EvidenceSource::Dmesg {
            timestamp: String::new(),
        },
        kind: CrashKind::GpuHang,
        snippet: line.trim().to_string(),
        severity: 80,
    })
}

/// Parse a kernel log line for trap signatures.
/// Matches: `trap invalid opcode`, `softlockup`, `hardlockup` (NMI watchdog).
pub fn parse_kernel_trap(line: &str) -> Option<ForensicEvidence> {
    let lower = line.to_lowercase();
    if lower.contains("trap invalid opcode") || lower.contains("softlockup") {
        return Some(ForensicEvidence {
            source: EvidenceSource::Dmesg {
                timestamp: String::new(),
            },
            kind: CrashKind::SoftLockup,
            snippet: line.trim().to_string(),
            severity: 70,
        });
    }
    if lower.contains("hardlockup")
        || lower.contains("hard lockup")
        || lower.contains("nmi watchdog")
    {
        return Some(ForensicEvidence {
            source: EvidenceSource::Dmesg {
                timestamp: String::new(),
            },
            kind: CrashKind::HardLockup,
            snippet: line.trim().to_string(),
            severity: 90,
        });
    }
    None
}

/// Parse an RCU grace period stall line from dmesg/journalctl.
/// Matches: `rcu_preempt detected stalls`, `rcu_sched detected stalls`,
/// `rcu_tasks detected stalls`, `rcu_tasks_trace detected stalls`,
/// `expedited RCU stall`.
pub fn parse_rcu_stall(line: &str) -> Option<ForensicEvidence> {
    let lower = line.to_lowercase();
    let matched = lower.contains("rcu_preempt detected stalls")
        || lower.contains("rcu_sched detected stalls")
        || lower.contains("rcu_tasks detected stalls")
        || lower.contains("rcu_tasks_trace detected stalls")
        || lower.contains("expedited rcu stall");
    if !matched {
        return None;
    }
    Some(ForensicEvidence {
        source: EvidenceSource::Dmesg {
            timestamp: String::new(),
        },
        kind: CrashKind::RcuStall,
        snippet: line.trim().to_string(),
        severity: 85,
    })
}

/// Parse a hung task line from dmesg/journalctl.
/// Matches: `hung_task: blocked for N seconds`, `hung task: blocked for N seconds`,
/// `task <name>:<pid> blocked for more than N seconds`.
pub fn parse_hung_task(line: &str) -> Option<ForensicEvidence> {
    let lower = line.to_lowercase();
    let is_hung = lower.contains("hung_task")
        || lower.contains("hung task")
        || (lower.contains("task ") && lower.contains("blocked for"));
    if !is_hung {
        return None;
    }
    if !lower.contains("blocked for") {
        return None;
    }
    Some(ForensicEvidence {
        source: EvidenceSource::Dmesg {
            timestamp: String::new(),
        },
        kind: CrashKind::HungTask,
        snippet: line.trim().to_string(),
        severity: 75,
    })
}

/// Parse an RCU read-side critical section context switch warning.
/// Precursor signal: `Voluntary context switch within RCU read-side critical section`.
pub fn parse_rcu_context_switch_warning(line: &str) -> Option<ForensicEvidence> {
    let lower = line.to_lowercase();
    if !lower.contains("voluntary context switch within rcu") {
        return None;
    }
    Some(ForensicEvidence {
        source: EvidenceSource::Dmesg {
            timestamp: String::new(),
        },
        kind: CrashKind::RcuStall,
        snippet: line.trim().to_string(),
        severity: 55,
    })
}

/// Parse boot_id from `/proc/sys/kernel/random/boot_id` content.
pub fn parse_boot_id(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

/// Parse a line from `journalctl --list-boots` output.
pub fn parse_boot_table_line(line: &str) -> Option<(String, i64, i64)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }
    let boot_id = parts[1].to_string();
    let first_entry: i64 = parts[2].parse().ok()?;
    let last_entry: i64 = parts[3].parse().ok()?;
    Some((boot_id, first_entry, last_entry))
}

/// Parse a journalctl line for systemd service failure signatures.
pub fn parse_service_failed_line(line: &str) -> Option<ServiceLoop> {
    let trimmed = line.trim();
    if !trimmed.contains("Failed with result") {
        return None;
    }
    let unit = trimmed.split_whitespace().find_map(|tok| {
        tok.strip_suffix(".service:")
            .or_else(|| tok.strip_suffix(".service"))
            .map(String::from)
    })?;
    let exit_status = if trimmed.contains("exit-code") {
        203
    } else if trimmed.contains("failure") {
        1
    } else if trimmed.contains("invalid-arg") || trimmed.contains("invalidarg") {
        2
    } else {
        0
    };
    Some(ServiceLoop {
        unit,
        restart_count: 1,
        exit_status,
        window_secs: 3600,
    })
}

/// Parse the header of an Apport crash dump file.
pub fn parse_crash_dump_header(content: &str) -> Option<(String, String)> {
    let mut executable_path = None;
    let mut date = None;
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("ExecutablePath:") {
            executable_path = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("Date:") {
            date = Some(val.trim().to_string());
        }
    }
    match (executable_path, date) {
        (Some(exec_path), Some(dt)) => Some((exec_path, dt)),
        _ => None,
    }
}
