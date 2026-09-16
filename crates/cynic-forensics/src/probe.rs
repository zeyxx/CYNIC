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
pub fn parse_kernel_trap(line: &str) -> Option<ForensicEvidence> {
    let lower = line.to_lowercase();
    let matched = lower.contains("trap invalid opcode") || lower.contains("softlockup");
    if !matched {
        return None;
    }
    Some(ForensicEvidence {
        source: EvidenceSource::Dmesg {
            timestamp: String::new(),
        },
        kind: CrashKind::SoftLockup,
        snippet: line.trim().to_string(),
        severity: 70,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dmesg_gpu_fault_page_fault() {
        let line =
            "amdgpu 0000:30:00.0: [gfxhub0] no-retry page fault (src_id:0 ring:40 vmid:0 pasid:0)";
        let evidence = parse_dmesg_gpu_fault(line).expect("should match GPU page fault");
        assert_eq!(evidence.kind, CrashKind::GpuHang);
        assert_eq!(evidence.severity, 80);
        assert!(evidence.snippet.contains("page fault"));
    }

    #[test]
    fn parse_dmesg_gpu_fault_ring_timeout() {
        let line = "amdgpu 0000:30:00.0: ring gfx timeout";
        let evidence = parse_dmesg_gpu_fault(line).expect("should match ring gfx timeout");
        assert_eq!(evidence.kind, CrashKind::GpuHang);
    }

    #[test]
    fn parse_dmesg_gpu_fault_reset_failed() {
        let line = "amdgpu: reset failed";
        let evidence = parse_dmesg_gpu_fault(line).expect("should match reset failed");
        assert_eq!(evidence.kind, CrashKind::GpuHang);
    }

    #[test]
    fn parse_dmesg_gpu_fault_no_match() {
        let line = "usb 1-1: new high-speed USB device number 2 using xhci_hcd";
        assert!(parse_dmesg_gpu_fault(line).is_none());
    }

    #[test]
    fn parse_kernel_trap_invalid_opcode() {
        let line = "kernel: traps: chrome[334751] trap invalid opcode ip:5ebabc786724 sp:7ffd449e4d30 error:0";
        let evidence = parse_kernel_trap(line).expect("should match trap invalid opcode");
        assert_eq!(evidence.kind, CrashKind::SoftLockup);
        assert_eq!(evidence.severity, 70);
        assert!(evidence.snippet.contains("trap invalid opcode"));
    }

    #[test]
    fn parse_kernel_trap_softlockup() {
        let line = "kernel: watchdog: BUG: softlockup - CPU#3 stuck for 22s!";
        let evidence = parse_kernel_trap(line).expect("should match softlockup");
        assert_eq!(evidence.kind, CrashKind::SoftLockup);
    }

    #[test]
    fn parse_kernel_trap_no_match() {
        let line = "kernel: CPU0: Temperature above threshold, clock throttled";
        assert!(parse_kernel_trap(line).is_none());
    }

    #[test]
    fn parse_boot_id_valid() {
        let input = "66515a0e-43dc-4a9b-860a-f1625e78b103";
        let boot_id = parse_boot_id(input).expect("should parse boot_id");
        assert_eq!(boot_id, "66515a0e-43dc-4a9b-860a-f1625e78b103");
    }

    #[test]
    fn parse_boot_id_with_newline() {
        let input = "66515a0e-43dc-4a9b-860a-f1625e78b103\n";
        let boot_id = parse_boot_id(input).expect("should parse boot_id with trailing newline");
        assert_eq!(boot_id, "66515a0e-43dc-4a9b-860a-f1625e78b103");
    }

    #[test]
    fn parse_boot_id_empty() {
        assert!(parse_boot_id("").is_none());
        assert!(parse_boot_id("   \n").is_none());
    }

    #[test]
    fn parse_boot_table_line_valid() {
        let line = " -1 66515a0e-43dc-4a9b-860a-f1625e78b103 1724000000000000 1724100000000000 Sat 2026-08-30 20:00:00 CEST—Sat 2026-08-30 22:45:00 CEST";
        let (boot_id, first, last) =
            parse_boot_table_line(line).expect("should parse boot table line");
        assert_eq!(boot_id, "66515a0e-43dc-4a9b-860a-f1625e78b103");
        assert_eq!(first, 1724000000000000);
        assert_eq!(last, 1724100000000000);
    }

    #[test]
    fn parse_boot_table_line_invalid() {
        assert!(parse_boot_table_line("").is_none());
        assert!(parse_boot_table_line("invalid line").is_none());
    }

    #[test]
    fn parse_service_failed_line_exit_code() {
        let line = "août 30 22:40:39 ubuntu systemd[2920]: discord-pac-server.service: Failed with result 'exit-code'.";
        let sl = parse_service_failed_line(line).expect("should parse service failed line");
        assert_eq!(sl.unit, "discord-pac-server");
        assert_eq!(sl.exit_status, 203);
    }

    #[test]
    fn parse_service_failed_line_failure() {
        let line = "systemd[1]: foobar.service: Failed with result 'failure'.";
        let sl = parse_service_failed_line(line).expect("should parse service failure");
        assert_eq!(sl.unit, "foobar");
        assert_eq!(sl.exit_status, 1);
    }

    #[test]
    fn parse_service_failed_line_no_match() {
        let line = "systemd[1]: Started User Manager for UID 1000.";
        assert!(parse_service_failed_line(line).is_none());
    }

    #[test]
    fn parse_crash_dump_header_valid() {
        let content = "ProblemType: Crash\nDate: Sat Aug 30 22:45:00 2026\nExecutablePath: /usr/bin/some-app\nProcCmdline: some-app --flag\n";
        let (exec_path, date) =
            parse_crash_dump_header(content).expect("should parse crash dump header");
        assert_eq!(exec_path, "/usr/bin/some-app");
        assert_eq!(date, "Sat Aug 30 22:45:00 2026");
    }

    #[test]
    fn parse_crash_dump_header_missing_fields() {
        let content = "ProblemType: Crash\nProcCmdline: some-app\n";
        assert!(parse_crash_dump_header(content).is_none());
    }
}
