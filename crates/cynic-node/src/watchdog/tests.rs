use super::sampler::watchdog_status;

#[test]
fn watchdog_status_requires_kernel_and_clear_network() {
    assert_eq!(watchdog_status(true, false), "ok");
    assert_eq!(watchdog_status(true, true), "degraded");
    assert_eq!(watchdog_status(false, false), "degraded");
    assert_eq!(watchdog_status(false, true), "degraded");
}
