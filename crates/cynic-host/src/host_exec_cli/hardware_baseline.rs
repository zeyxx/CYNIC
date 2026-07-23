use std::process::ExitCode;

use cynic_protocol::{ExecutionAuthorization, PolicyAuthority};

use crate::adapters::local_hardware_baseline::LocalHardwareBaselineInstaller;
use crate::adapters::local_observation::LocalHostObservationAdapter;
use crate::hardware_baseline_apply::{HardwareBaselineBundle, apply_hardware_baseline};
use crate::hardware_conformance::observe_hardware_conformance;
use crate::host_exec_cli::args::has_flag;
use crate::host_exec_cli::io::emit_json;
use crate::runtime_layout::current_runtime_layout;

pub(crate) fn execute(args: &[String]) -> Result<ExitCode, String> {
    if !has_flag(args, "--apply") || args.len() != 1 {
        return Err("hardware-baseline-enroll requires exactly --apply".to_string());
    }
    let layout = current_runtime_layout();
    let bundle_path = layout.hardware_baseline_bundle_path();
    let payload = std::fs::read(&bundle_path)
        .map_err(|error| format!("read hardware baseline bundle: {error}"))?;
    let bundle = serde_json::from_slice::<HardwareBaselineBundle>(&payload)
        .map_err(|error| format!("parse hardware baseline bundle: {error}"))?;
    if bundle.target_path != layout.hardware_baseline_path().display().to_string() {
        return Err("hardware baseline bundle target does not match runtime authority".to_string());
    }
    let authorization = ExecutionAuthorization::new(
        bundle.admission.intent.actor.clone(),
        PolicyAuthority::HumanAdmin,
        true,
        bundle.admission.mutation.preconditions.clone(),
    );
    let apply = apply_hardware_baseline(&bundle, &authorization, &LocalHardwareBaselineInstaller);
    let hardware = observe_hardware_conformance(&LocalHostObservationAdapter);
    let completed = apply.status == "applied_pending_reobservation"
        && hardware.identity_state == cynic_protocol::SecurityEvidenceState::Proven;
    emit_json(
        &serde_json::json!({
            "schema": "cynic.host.hardware_baseline_reconcile.v1",
            "status": if completed { "completed" } else { "blocked" },
            "apply": apply,
            "hardware": hardware,
        }),
        !completed,
    )
}
