//! Thin CLI facade for OS substrate health observation.
//!
//! This is the operator surface for `cynic host os-health`. It is orchestration
//! only: observe (portable probes) -> decide (pure policy in
//! `cynic_host::os_health`) -> project (JSON or human). No mutation, no
//! hardcoded substrate. The decision layer lives in `cynic-host`, not here.

use cynic_host::os_health::{
    LocalOsHealthObserver, OsHealthAssessment, assess_os_health_default, os_health_human,
};
use serde::Serialize;

use crate::emit_json;

/// Build the assessment by observing the live host.
pub fn assess_host_os_health() -> OsHealthAssessment {
    assess_os_health_default(&LocalOsHealthObserver)
}

/// Project the assessment. Non-zero exit when unhealthy/degraded so operators
/// and scripts can gate on it.
pub fn run_os_health(json: bool) -> std::process::ExitCode {
    let assessment = assess_host_os_health();
    if json {
        let projected = serde_json::json!({
            "schema": "cynic.os_health.assessment.v1",
            "assessment": &assessment,
        });
        let failed = assessment.status == "unhealthy";
        return emit_json(&projected, failed);
    }
    print!("{}", os_health_human(&assessment));
    match assessment.status.as_str() {
        "unhealthy" => std::process::ExitCode::from(u8::MAX), // 255
        "degraded" => std::process::ExitCode::FAILURE,         // 1
        _ => std::process::ExitCode::SUCCESS,
    }
}

#[allow(dead_code)]
fn _assert_projectable(_: &impl Serialize) {}
