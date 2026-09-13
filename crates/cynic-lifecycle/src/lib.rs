//! Update lifecycle: harness phases as promotion membrane + ops repair.
//!
//! Layering (not a monolith):
//! - [`types`] — report/state vocabulary
//! - [`time`] — clock helper
//! - [`port`] — [`ArgvRunner`] (domain never owns `Command::new("cynic")`)
//! - [`policy`] — phase → gates/mutations (pure tables)
//! - [`state_store`] — JSON persistence paths
//! - [`plan`] — status + planned actions
//! - [`gate`] / [`promote`] — gate execute + promote
//! - [`observe`] — ops findings from runner JSON
//! - [`repair`] / [`apply`] — phase-filtered mutations
//!
//! CLI injects [`LocalArgvRunner`]; tests inject [`MapArgvRunner`].

mod apply;
pub mod gate;
pub mod harness_context;
pub mod kernel_release;
mod observe;
mod plan;
mod policy;
mod port;
mod promote;
mod repair;
mod state_store;
mod time;
mod types;

pub use apply::apply_phase;
pub use gate::run_gates;
pub use kernel_release::{
    KernelReleaseReport, KernelReleaseSpec, reconcile_kernel_release,
    reconcile_runtime_artifact_release,
};
pub use observe::ops_report;
pub use plan::{plan_report, promote_eligibility, status_report};
pub use policy::phase_policy;
pub use port::{ArgvOutput, ArgvRunner, LocalArgvRunner, MapArgvRunner, parse_json, split_command};
pub use promote::promote;
pub use repair::{repair_apply, repair_plan};
pub use state_store::{default_state_path, load_state, save_state};
pub use types::*;

use std::path::Path;

/// Convenience: status with default local runner (no subprocess for pure status).
pub fn status_report_default(state_path: &Path) -> LifecycleStatusReport {
    status_report(state_path)
}

/// Convenience wrappers that inject [`LocalArgvRunner`] for thin CLI call sites.
pub fn ops_report_local(state_path: &Path) -> OpsReport {
    ops_report(state_path, &LocalArgvRunner)
}

pub fn run_gates_local(state_path: &Path, execute: bool) -> LifecycleGateReport {
    run_gates(state_path, execute, &LocalArgvRunner)
}

pub fn repair_plan_local(state_path: &Path) -> RepairReport {
    repair_plan(state_path, &LocalArgvRunner)
}

pub fn repair_apply_local(state_path: &Path, yes: bool) -> RepairReport {
    repair_apply(state_path, yes, &LocalArgvRunner)
}

pub fn apply_phase_local(state_path: &Path, yes: bool) -> ApplyReport {
    apply_phase(state_path, yes, &LocalArgvRunner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_state() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("cynic-lifecycle-test-{}-{}", std::process::id(), n));
        let _ = std::fs::create_dir_all(&dir);
        dir.join("update-state.json")
    }

    #[test]
    fn phase_progression_is_linear() {
        let mut p = LifecyclePhase::ClosedLocalLab;
        assert_eq!(p.next(), Some(LifecyclePhase::Conformance));
        p = LifecyclePhase::Conformance;
        assert_eq!(p.next(), Some(LifecyclePhase::Staging));
        p = LifecyclePhase::Staging;
        assert_eq!(p.next(), Some(LifecyclePhase::Canary));
        p = LifecyclePhase::Canary;
        assert_eq!(p.next(), Some(LifecyclePhase::Production));
        assert!(LifecyclePhase::Production.next().is_none());
    }

    #[test]
    fn promote_requires_green_gate() {
        let path = tmp_state();
        let _ = std::fs::remove_file(&path);
        let r = promote(&path, Some("conformance"), true).unwrap();
        assert_eq!(r.status, "blocked");
        assert!(r.message.contains("gate"));
    }

    #[test]
    fn promote_one_step_after_gate() {
        let path = tmp_state();
        let state = UpdateLifecycleState {
            last_gate_ok: Some(true),
            ..UpdateLifecycleState::default()
        };
        save_state(&path, &state).unwrap();
        let r = promote(&path, None, true).unwrap();
        assert_eq!(r.status, "ok");
        assert_eq!(r.to, Some("conformance"));
        let loaded = load_state(&path);
        assert_eq!(loaded.current_phase, LifecyclePhase::Conformance);
        assert_eq!(loaded.last_gate_ok, None);
    }

    #[test]
    fn lab_policy_does_not_allow_fleet() {
        let p = phase_policy(&LifecyclePhase::ClosedLocalLab);
        assert!(!p.allowed_mutations.contains(&MutationClass::FleetNetwork));
        assert!(p.allowed_mutations.contains(&MutationClass::DepsScripted));
    }

    #[test]
    fn canary_allows_fleet_mutations() {
        let p = phase_policy(&LifecyclePhase::Canary);
        assert!(p.allowed_mutations.contains(&MutationClass::FleetNetwork));
    }

    #[test]
    fn gate_plan_does_not_spawn() {
        let path = tmp_state();
        let _ = std::fs::remove_file(&path);
        let runner = MapArgvRunner::default();
        let r = run_gates(&path, false, &runner);
        assert_eq!(r.status, "planned");
        assert!(!r.all_required_ok);
        assert!(r.results.iter().all(|g| !g.ran));
    }

    #[test]
    fn gate_execute_uses_runner_not_hardwired_cynic() {
        let path = tmp_state();
        let _ = std::fs::remove_file(&path);
        let mut runner = MapArgvRunner::default();
        // prefix match any cynic runtime validate
        runner.responses.push((
            vec![
                "cynic".into(),
                "runtime".into(),
                "validate".into(),
                "--json".into(),
            ],
            ArgvOutput {
                exit_code: Some(0),
                stdout: r#"{"status":"ok"}"#.into(),
                stderr: String::new(),
                spawn_error: None,
            },
        ));
        runner.responses.push((
            vec!["cynic".into(), "deps".into(), "--json".into()],
            ArgvOutput {
                exit_code: Some(0),
                stdout: r#"{"status":"ok","summary":{"missing":0,"desired_upgrade":0,"required_blocked":0}}"#.into(),
                stderr: String::new(),
                spawn_error: None,
            },
        ));
        runner.responses.push((
            vec![
                "cynic".into(),
                "runtime".into(),
                "harness".into(),
                "--json".into(),
            ],
            ArgvOutput {
                exit_code: Some(0),
                stdout: r#"{"status":"ok"}"#.into(),
                stderr: String::new(),
                spawn_error: None,
            },
        ));
        let r = run_gates(&path, true, &runner);
        assert!(r.results.iter().all(|g| g.ran));
        assert_eq!(r.status, "ok");
        assert!(r.all_required_ok);
        let st = load_state(&path);
        assert_eq!(st.last_gate_ok, Some(true));
    }

    #[test]
    fn gate_rejects_soft_fail_json_status() {
        let path = tmp_state();
        let _ = std::fs::remove_file(&path);
        let mut runner = MapArgvRunner::default();
        // All lab gates return exit 0 but metrics-like insufficient_evidence on validate
        runner.responses.push((
            vec![
                "cynic".into(),
                "runtime".into(),
                "validate".into(),
                "--json".into(),
            ],
            ArgvOutput {
                exit_code: Some(0),
                stdout: r#"{"status":"insufficient_evidence"}"#.into(),
                stderr: String::new(),
                spawn_error: None,
            },
        ));
        runner.responses.push((
            vec!["cynic".into(), "deps".into(), "--json".into()],
            ArgvOutput {
                exit_code: Some(0),
                stdout: r#"{"status":"ok"}"#.into(),
                stderr: String::new(),
                spawn_error: None,
            },
        ));
        runner.responses.push((
            vec![
                "cynic".into(),
                "runtime".into(),
                "harness".into(),
                "--json".into(),
            ],
            ArgvOutput {
                exit_code: Some(0),
                stdout: r#"{"status":"ok"}"#.into(),
                stderr: String::new(),
                spawn_error: None,
            },
        ));
        let r = run_gates(&path, true, &runner);
        assert_eq!(r.status, "blocked");
        assert!(!r.all_required_ok);
        assert!(
            !r.results
                .iter()
                .find(|g| g.id == "runtime_validate")
                .unwrap()
                .ok
        );
    }

    #[test]
    fn ops_report_builds_findings_from_runner_json() {
        let path = tmp_state();
        let _ = std::fs::remove_file(&path);
        let mut runner = MapArgvRunner::default();
        runner.responses.push((
            vec!["cynic".into(), "deps".into(), "--json".into()],
            ArgvOutput {
                exit_code: Some(0),
                stdout: r#"{"summary":{"missing":1,"desired_upgrade":2,"required_blocked":0},"deps":[{"id":"ods","status":"desired_upgrade"}]}"#.into(),
                stderr: String::new(),
                spawn_error: None,
            },
        ));
        runner.responses.push((
            vec![
                "cynic".into(),
                "runtime".into(),
                "inference".into(),
                "--json".into(),
            ],
            ArgvOutput {
                exit_code: Some(0),
                stdout: r#"{"status":"ok","config_path":"/tmp/x","backends":[{"id":"a"}]}"#.into(),
                stderr: String::new(),
                spawn_error: None,
            },
        ));
        runner.responses.push((
            vec![
                "cynic".into(),
                "runtime".into(),
                "inference-status".into(),
                "--json".into(),
            ],
            ArgvOutput {
                exit_code: Some(0),
                stdout: r#"{"state":"active"}"#.into(),
                stderr: String::new(),
                spawn_error: None,
            },
        ));
        runner.responses.push((
            vec![
                "cynic".into(),
                "dev".into(),
                "surface-audit".into(),
                "--json".into(),
            ],
            ArgvOutput {
                exit_code: Some(0),
                stdout: r#"{"status":"ok","blocked_count":0}"#.into(),
                stderr: String::new(),
                spawn_error: None,
            },
        ));
        let report = ops_report(&path, &runner);
        assert!(!report.black_box);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "deps.desired_upgrade")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "lifecycle.gate_missing")
        );
    }

    #[test]
    fn repair_plan_filters_by_phase_and_uses_runner() {
        let path = tmp_state();
        let _ = std::fs::remove_file(&path);
        let mut runner = MapArgvRunner::default();
        runner.responses.push((
            vec!["cynic".into(), "deps".into(), "--json".into()],
            ArgvOutput {
                exit_code: Some(0),
                stdout: r#"{"summary":{"missing":0,"desired_upgrade":1,"required_blocked":0},"deps":[]}"#.into(),
                stderr: String::new(),
                spawn_error: None,
            },
        ));
        // empty/defaults for other observes
        for argv in [
            vec!["cynic", "runtime", "inference", "--json"],
            vec!["cynic", "runtime", "inference-status", "--json"],
            vec!["cynic", "dev", "surface-audit", "--json"],
        ] {
            runner.responses.push((
                argv.into_iter().map(String::from).collect(),
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: r#"{"status":"ok","backends":[{"id":"x"}],"state":"active","blocked_count":0}"#.into(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            ));
        }
        let plan = repair_plan(&path, &runner);
        assert!(plan.actions.iter().any(|a| a.id == "run_gate"));
        assert!(plan.actions.iter().any(|a| a.id == "deps_upgrade"));
        // plan only — no spawn of upgrades
        assert_eq!(plan.status, "planned");
        assert!(!plan.mutates);
    }

    #[test]
    fn apply_phase_plan_only_without_yes() {
        let path = tmp_state();
        let _ = std::fs::remove_file(&path);
        let runner = MapArgvRunner::default();
        let r = apply_phase(&path, false, &runner);
        assert_eq!(r.status, "planned");
        assert!(!r.mutates);
        assert!(r.actions.iter().any(|a| a.id == "deps_upgrade"));
    }
}
