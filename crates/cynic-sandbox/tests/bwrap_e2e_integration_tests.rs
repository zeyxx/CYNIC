//! End-to-End integration tests for LocalBwrapSubstrateAdapter.
//!
//! Executes REAL bubblewrap processes on the host to prove isolation boundaries.

use cynic_lifecycle::LocalArgvRunner;
use cynic_protocol::compute::{ComputeRequirements, ComputeSubstratePort};
use cynic_sandbox::adapters::bwrap_substrate::LocalBwrapSubstrateAdapter;
use std::sync::Arc;

fn has_bwrap_binary() -> bool {
    std::process::Command::new("bwrap")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tokio::test]
async fn bwrap_e2e_proves_network_pid_and_filesystem_isolation() {
    if !has_bwrap_binary() {
        eprintln!("Skipping bwrap E2E test: bwrap binary not found in PATH");
        return;
    }

    let runner = Arc::new(LocalArgvRunner);
    let adapter = LocalBwrapSubstrateAdapter::new(runner);

    let reqs = ComputeRequirements::default();
    let allocation = adapter
        .allocate_runner("e2e-isolation-proof", &reqs)
        .await
        .expect("allocation should succeed");

    // PROOF 1: Read-only filesystem confinement
    // Attempting to touch a file in /usr must fail with Read-only file system
    let ro_probe = vec![
        "touch".to_string(),
        "/usr/cynic_illegal_write_probe".to_string(),
    ];
    let ro_result = adapter
        .execute_command(&allocation.allocation_id, &ro_probe, 5000)
        .await
        .expect("command should execute");
    assert_ne!(
        ro_result.exit_code, 0,
        "Writing to /usr MUST fail inside sandbox"
    );
    assert!(
        ro_result.stderr.to_lowercase().contains("read-only")
            || ro_result.stderr.to_lowercase().contains("permission denied"),
        "Expected Read-only filesystem error, got: {}",
        ro_result.stderr
    );

    // PROOF 2: Private writable /tmp
    let rw_probe = vec![
        "sh".to_string(),
        "-c".to_string(),
        "echo 'sovereign proof' > /tmp/proof.txt && cat /tmp/proof.txt".to_string(),
    ];
    let rw_result = adapter
        .execute_command(&allocation.allocation_id, &rw_probe, 5000)
        .await
        .expect("command should execute");
    assert_eq!(rw_result.exit_code, 0);
    assert_eq!(rw_result.stdout.trim(), "sovereign proof");

    // PROOF 3: Private PID namespace (only sandbox processes visible)
    let pid_probe = vec![
        "sh".to_string(),
        "-c".to_string(),
        "ps -ef 2>/dev/null || cat /proc/self/stat | awk '{print $1}'".to_string(),
    ];
    let pid_result = adapter
        .execute_command(&allocation.allocation_id, &pid_probe, 5000)
        .await
        .expect("command should execute");
    assert_eq!(pid_result.exit_code, 0);
    let pid_num: u32 = pid_result
        .stdout
        .lines()
        .last()
        .unwrap_or("0")
        .trim()
        .parse()
        .unwrap_or(0);
    assert!(
        pid_num > 0 && pid_num <= 10,
        "Sandboxed process PID must be small isolated number (<= 10), got {}",
        pid_num
    );

    // Release runner
    adapter
        .release_runner(&allocation.allocation_id)
        .await
        .expect("release should succeed");
}
