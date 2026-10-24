use cynic_intel::soc::*;

#[test]
fn test_gpu_fault_detection() {
    let t = SocThresholds::default();
    assert!(detect_gpu_fault(&t, 0).is_none());
    let a = detect_gpu_fault(&t, 1).expect("alert on 1 fault");
    assert_eq!(a.kind, "soc_gpu_hang");
    assert_eq!(a.severity, "critical");
}

#[test]
fn test_softlockup_detection() {
    let t = SocThresholds::default();
    assert!(detect_softlockup(&t, 1).is_none());
    let a = detect_softlockup(&t, 2).expect("alert on 2 traps");
    assert_eq!(a.kind, "soc_softlockup");
    assert_eq!(a.severity, "critical");
}

#[test]
fn test_rcu_stall_detection() {
    let t = SocThresholds::default();
    assert!(detect_rcu_stall(&t, 0).is_none());
    let a = detect_rcu_stall(&t, 1).expect("alert on 1 stall");
    assert_eq!(a.kind, "soc_rcu_stall");
    assert_eq!(a.severity, "critical");
}

#[test]
fn test_hung_task_detection() {
    let t = SocThresholds::default();
    assert!(detect_hung_task(&t, 1).is_none());
    let a = detect_hung_task(&t, 2).expect("alert on 2 hung tasks");
    assert_eq!(a.kind, "soc_hung_task");
    assert_eq!(a.severity, "warning");
}

#[test]
fn test_service_cascade_detection() {
    let t = SocThresholds::default();
    assert!(detect_service_cascade(&t, 500).is_none());
    let a = detect_service_cascade(&t, 1000).expect("alert on 1000 exec delta");
    assert_eq!(a.kind, "soc_service_cascade");
    assert_eq!(a.severity, "critical");
}

#[test]
fn test_canary_disconnect_detection() {
    let t = SocThresholds::default();
    assert!(detect_canary_disconnect(&t, 2).is_none());
    let a = detect_canary_disconnect(&t, 3).expect("alert on 3 disconnects");
    assert_eq!(a.kind, "soc_canary_disconnect");
    assert_eq!(a.severity, "warning");
}

#[test]
fn test_check_all_clean_snapshot() {
    let t = SocThresholds::default();
    let snap = ForensicsSnapshot::default();
    let alerts = check_all(&t, &snap);
    assert!(alerts.is_empty());
}

#[test]
fn test_check_all_multiple_failures() {
    let t = SocThresholds::default();
    let snap = ForensicsSnapshot {
        boot_id: "test-boot".to_string(),
        gpu_faults: 2,
        kernel_traps: 5,
        service_cascades: 1500,
        canary_disconnects: 4,
        watchdog_failures: 6,
        service_restart_rate: 600.0,
        dbus_timeouts: 3,
        rcu_stalls: 1,
        hung_tasks: 3,
        observed_at_unix_s: 0, // test: epoch 0 = always fresh
    };
    let alerts = check_all(&t, &snap);
    assert_eq!(alerts.len(), 9);
}
