use serde::Serialize;

use crate::hardware_baseline::HardwareBaselinePlan;
use crate::hardware_baseline_apply::{
    HardwareBaselineBundleProof, HardwareBaselineBundleStorePort, build_hardware_baseline_bundle,
};
use crate::runtime_layout::HostRuntimeLayout;
use crate::update_lifecycle::ArgvRunner;

pub const HARDWARE_BASELINE_RECONCILE_SCHEMA: &str =
    "cynic.host.hardware_baseline_reconcile_request.v1";

#[derive(Debug, Serialize)]
pub struct HardwareBaselineReconcileReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub helper_argv: Vec<String>,
    pub bundle: Option<HardwareBaselineBundleProof>,
    pub helper_status: Option<String>,
    pub blockers: Vec<String>,
}

pub fn reconcile_hardware_baseline(
    plan: &HardwareBaselinePlan,
    requested_apply: bool,
    layout: &HostRuntimeLayout,
    store: &impl HardwareBaselineBundleStorePort,
    runner: &impl ArgvRunner,
) -> HardwareBaselineReconcileReport {
    let helper_argv = vec![
        layout.sudo_bin(),
        "-n".to_string(),
        layout.host_exec_helper_path(),
        "hardware-baseline-enroll".to_string(),
        "--apply".to_string(),
    ];
    if plan.status == "converged" {
        return report(
            "converged",
            false,
            requested_apply,
            helper_argv,
            None,
            None,
            Vec::new(),
        );
    }
    if plan.status != "ready" || !plan.mutates {
        return report(
            "blocked",
            false,
            requested_apply,
            helper_argv,
            None,
            None,
            vec!["hardware_baseline_plan_not_ready".to_string()],
        );
    }
    if !requested_apply {
        return report("planned", false, false, helper_argv, None, None, Vec::new());
    }
    let bundle = match build_hardware_baseline_bundle(plan) {
        Ok(bundle) => bundle,
        Err(blockers) => {
            return report("blocked", false, true, helper_argv, None, None, blockers);
        }
    };
    let bundle_proof =
        match store.persist_absent_or_matching(&layout.hardware_baseline_bundle_path(), &bundle) {
            Ok(proof) => proof,
            Err(error) => {
                return report("blocked", false, true, helper_argv, None, None, vec![error]);
            }
        };
    let argv = helper_argv.iter().map(String::as_str).collect::<Vec<_>>();
    let output = runner.run(&argv);
    let helper_json = serde_json::from_str::<serde_json::Value>(output.stdout.trim()).ok();
    let helper_status = helper_json
        .as_ref()
        .and_then(|value| value.get("status"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let helper_schema_valid = helper_json
        .as_ref()
        .and_then(|value| value.get("schema"))
        .and_then(serde_json::Value::as_str)
        == Some("cynic.host.hardware_baseline_reconcile.v1");
    if output.success() && helper_schema_valid && helper_status.as_deref() == Some("completed") {
        return report(
            "completed",
            true,
            true,
            helper_argv,
            Some(bundle_proof),
            helper_status,
            Vec::new(),
        );
    }
    let mut blockers = Vec::new();
    if !output.success() {
        blockers.push(format!(
            "helper_execution_failed:exit_code={:?}",
            output.exit_code
        ));
    }
    if let Some(error) = output.spawn_error {
        blockers.push(error);
    }
    if !output.stderr.trim().is_empty() {
        blockers.push(format!("helper_stderr:{}", output.stderr.trim()));
    }
    if !helper_schema_valid {
        blockers.push("helper_report_schema_invalid".to_string());
    }
    if helper_status.as_deref() != Some("completed") {
        blockers.push("helper_did_not_complete".to_string());
    }
    report(
        "blocked",
        bundle_proof.created,
        true,
        helper_argv,
        Some(bundle_proof),
        helper_status,
        blockers,
    )
}

fn report(
    status: &'static str,
    mutates: bool,
    requested_apply: bool,
    helper_argv: Vec<String>,
    bundle: Option<HardwareBaselineBundleProof>,
    helper_status: Option<String>,
    blockers: Vec<String>,
) -> HardwareBaselineReconcileReport {
    HardwareBaselineReconcileReport {
        schema: HARDWARE_BASELINE_RECONCILE_SCHEMA,
        status,
        mutates,
        requested_apply,
        helper_argv,
        bundle,
        helper_status,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use crate::hardware_baseline::plan_hardware_baseline;
    use crate::hardware_baseline_apply::{
        HardwareBaselineBundle, HardwareBaselineBundleProof, HardwareBaselineBundleStorePort,
    };
    use crate::hardware_conformance::{
        EvidenceState, HardwareDevice, HardwareInventory, HardwareObservation, assess_hardware,
    };
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};

    struct Store;

    impl HardwareBaselineBundleStorePort for Store {
        fn persist_absent_or_matching(
            &self,
            path: &Path,
            _bundle: &HardwareBaselineBundle,
        ) -> Result<HardwareBaselineBundleProof, String> {
            Ok(HardwareBaselineBundleProof {
                path: path.display().to_string(),
                bundle_digest: "sha256:fixture".to_string(),
                created: true,
            })
        }
    }

    fn plan() -> HardwareBaselinePlan {
        let report = assess_hardware(HardwareObservation {
            inventory: HardwareInventory {
                board_name: Some("fixture".into()),
                cpu_model: Some("fixture".into()),
                devices: vec![HardwareDevice {
                    bus: "fixture".into(),
                    address: "0".into(),
                    class: None,
                    vendor_id: None,
                    device_id: None,
                    model: None,
                    firmware: None,
                    driver: None,
                }],
                ..Default::default()
            },
            baseline_digest: None,
            firmware_version: Some("1".into()),
            max_temperature_millicelsius: Some(40_000),
            corrected_memory_errors: Some(0),
            uncorrected_memory_errors: Some(0),
            storage_device_count: 1,
            offline_memory_test: EvidenceState::Unknown,
            memory_tuning: EvidenceState::Unknown,
            runtime_tuning_overrides: Vec::new(),
        });
        plan_hardware_baseline(
            "fixture",
            "operator",
            Path::new("/etc/cynic/hardware-baseline.json"),
            &report,
        )
    }

    fn layout() -> HostRuntimeLayout {
        HostRuntimeLayout {
            config_dir: "/etc/cynic".to_string(),
            helper_dir: "/usr/local/libexec/cynic".to_string(),
            sudoers_dir: "/etc/sudoers.d".to_string(),
            service_unit_dir: "/etc/systemd/system".to_string(),
            runtime_home_dir: "/var/lib/cynic".to_string(),
            state_dir: "/var/lib/cynic".to_string(),
            log_dir: "/var/log/cynic".to_string(),
            host_exec_bundle_dir: "/var/lib/cynic/staging/host-exec".to_string(),
            network_actuator_bundle_dir: "/var/lib/cynic/staging/network".to_string(),
            dns_bundle_dir: "/var/lib/cynic/staging/dns".to_string(),
            provider_fallback_bundle_dir: "/var/lib/cynic/staging/provider".to_string(),
        }
    }

    #[test]
    fn dry_run_only_projects_the_bounded_helper_argv() {
        let report = reconcile_hardware_baseline(
            &plan(),
            false,
            &layout(),
            &Store,
            &MapArgvRunner::default(),
        );
        assert_eq!(report.status, "planned");
        assert_eq!(report.bundle, None);
        assert_eq!(
            report.helper_argv,
            vec![
                "sudo",
                "-n",
                "/usr/local/libexec/cynic/cynic-host-exec",
                "hardware-baseline-enroll",
                "--apply"
            ]
        );
    }

    #[test]
    fn apply_accepts_only_the_exact_completed_helper_report() {
        let expected = vec![
            "sudo".to_string(),
            "-n".to_string(),
            "/usr/local/libexec/cynic/cynic-host-exec".to_string(),
            "hardware-baseline-enroll".to_string(),
            "--apply".to_string(),
        ];
        let runner = MapArgvRunner {
            responses: vec![(
                expected,
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: r#"{"schema":"cynic.host.hardware_baseline_reconcile.v1","status":"completed"}"#.to_string(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let report = reconcile_hardware_baseline(&plan(), true, &layout(), &Store, &runner);
        assert_eq!(report.status, "completed");
        assert!(report.mutates);
    }
}
