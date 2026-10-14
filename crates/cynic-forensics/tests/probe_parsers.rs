//! Integration tests for the pure forensics parsers.
//!
//! These tests lived inline in `src/probe.rs` and pushed it over the 400-line
//! hotspot threshold. The parsers themselves are unchanged; only the tests
//! moved, exercising the public API exactly as external consumers do.

use cynic_forensics::{
    CrashKind, parse_boot_id, parse_boot_table_line, parse_crash_dump_header,
    parse_dmesg_gpu_fault, parse_hung_task, parse_kernel_trap, parse_rcu_context_switch_warning,
    parse_rcu_stall, parse_service_failed_line,
};

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
    let line =
        "kernel: traps: chrome[334751] trap invalid opcode ip:5ebabc786724 sp:7ffd449e4d30 error:0";
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
    let (boot_id, first, last) = parse_boot_table_line(line).expect("should parse boot table line");
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

// ── RCU stall parser ──────────────────────────────────────────────────────

#[test]
fn parse_rcu_stall_preempt() {
    let line = "rcu: INFO: rcu_preempt detected stalls on CPUs/tasks:\n";
    let evidence = parse_rcu_stall(line).expect("should match RCU preempt stall");
    assert_eq!(evidence.kind, CrashKind::RcuStall);
    assert_eq!(evidence.severity, 85);
}

#[test]
fn parse_rcu_stall_sched() {
    let line = "rcu: INFO: rcu_sched detected stalls on CPUs/tasks:\n";
    let evidence = parse_rcu_stall(line).expect("should match RCU sched stall");
    assert_eq!(evidence.kind, CrashKind::RcuStall);
}

#[test]
fn parse_rcu_stall_tasks() {
    let line = "rcu: INFO: rcu_tasks detected stalls on tasks:\n";
    let evidence = parse_rcu_stall(line).expect("should match RCU tasks stall");
    assert_eq!(evidence.kind, CrashKind::RcuStall);
}

#[test]
fn parse_rcu_stall_tasks_trace() {
    let line = "rcu: INFO: rcu_tasks_trace detected stalls on tasks:\n";
    let evidence = parse_rcu_stall(line).expect("should match RCU tasks trace stall");
    assert_eq!(evidence.kind, CrashKind::RcuStall);
}

#[test]
fn parse_rcu_stall_expedited() {
    let line = "INFO: rcu: expedited RCU stall\n";
    let evidence = parse_rcu_stall(line).expect("should match expedited RCU stall");
    assert_eq!(evidence.kind, CrashKind::RcuStall);
}

#[test]
fn parse_rcu_stall_no_match() {
    let line = "rcu: INFO: rcu_preempt gp-kthread woke\n";
    assert!(parse_rcu_stall(line).is_none());
}

// ── Hung task parser ──────────────────────────────────────────────────────

#[test]
fn parse_hung_task_valid() {
    let line = "kernel: INFO: task kworker/0:0:1234 blocked for more than 120 seconds.\n";
    let evidence = parse_hung_task(line).expect("should match hung task");
    assert_eq!(evidence.kind, CrashKind::HungTask);
    assert_eq!(evidence.severity, 75);
}

#[test]
fn parse_hung_task_no_match() {
    let line = "kernel: INFO: task kworker/0:0:1234: state=S\n";
    assert!(parse_hung_task(line).is_none());
}

// ── RCU context switch warning ────────────────────────────────────────────

#[test]
fn parse_rcu_context_switch_warning_valid() {
    let line = "kernel: Voluntary context switch within RCU read-side critical section!\n";
    let evidence =
        parse_rcu_context_switch_warning(line).expect("should match RCU context switch warning");
    assert_eq!(evidence.kind, CrashKind::RcuStall);
    assert_eq!(evidence.severity, 55);
}

#[test]
fn parse_rcu_context_switch_warning_no_match() {
    let line = "kernel: RCU read-side critical section\n";
    assert!(parse_rcu_context_switch_warning(line).is_none());
}

// ── Hardlockup (NMI) ─────────────────────────────────────────────────────

#[test]
fn parse_kernel_trap_hardlockup() {
    let line = "NMI: Watchdog detected hard LOCKUP - CPU#0: hard lockup - CPU#0 stuck for 28s!\n";
    let evidence = parse_kernel_trap(line).expect("should match hardlockup");
    assert_eq!(evidence.kind, CrashKind::HardLockup);
    assert_eq!(evidence.severity, 90);
}
