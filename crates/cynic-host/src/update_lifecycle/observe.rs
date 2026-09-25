//! Operational observation: interpret runner JSON into findings.
//!
//! No hard-wired `cynic` spawn — caller injects [`ArgvRunner`].

use std::path::Path;

use super::policy::phase_policy;
use super::port::{ArgvRunner, parse_json};
use super::state_store::load_state;
use super::time::now_ms;
use super::types::{
    FindingCounts, FindingSeverity, MutationClass, OpsFinding, OpsReport,
    UPDATE_LIFECYCLE_OPS_SCHEMA,
};

pub fn ops_report(state_path: &Path, runner: &dyn ArgvRunner) -> OpsReport {
    let state = load_state(state_path);
    let policy = phase_policy(&state.current_phase);
    let mut findings = Vec::new();

    if state.last_gate_ok.is_none() {
        findings.push(OpsFinding {
            id: "lifecycle.gate_missing".into(),
            severity: FindingSeverity::Warning,
            surface: "lifecycle".into(),
            summary: "No gate result for current phase".into(),
            evidence: format!("phase={}", state.current_phase.as_str()),
            repair_id: Some("run_gate".into()),
            repair_command: Some("cynic lifecycle gate --yes".into()),
            phase_allows_repair: true,
        });
    } else if state.last_gate_ok == Some(false) {
        findings.push(OpsFinding {
            id: "lifecycle.gate_failed".into(),
            severity: FindingSeverity::Error,
            surface: "lifecycle".into(),
            summary: "Last lifecycle gate failed".into(),
            evidence: "last_gate_ok=false".into(),
            repair_id: Some("run_gate".into()),
            repair_command: Some("cynic lifecycle gate --yes".into()),
            phase_allows_repair: true,
        });
    }

    let deps_allows = policy
        .allowed_mutations
        .contains(&MutationClass::DepsScripted);
    collect_deps_findings(runner, deps_allows, &mut findings);
    let inf_allows = policy
        .allowed_mutations
        .contains(&MutationClass::InferenceRuntime);
    collect_inference_findings(runner, inf_allows, &mut findings);
    collect_surface_findings(runner, &mut findings);

    let mut counts = FindingCounts {
        info: 0,
        warning: 0,
        error: 0,
        critical: 0,
        repairable: 0,
    };
    for f in &findings {
        match f.severity {
            FindingSeverity::Info => counts.info += 1,
            FindingSeverity::Warning => counts.warning += 1,
            FindingSeverity::Error => counts.error += 1,
            FindingSeverity::Critical => counts.critical += 1,
        }
        if f.phase_allows_repair && f.repair_command.is_some() {
            counts.repairable += 1;
        }
    }

    let black_box = findings.is_empty();
    let status = if counts.critical > 0 || counts.error > 0 {
        "degraded"
    } else if counts.warning > 0 {
        "attention"
    } else {
        "ok"
    };

    let mut next = vec![
        "cynic lifecycle report --json".into(),
        "cynic lifecycle repair".into(),
        "cynic lifecycle repair --yes".into(),
        "cynic lifecycle apply --yes".into(),
        "cynic lifecycle gate --yes".into(),
    ];
    if counts.repairable > 0 {
        next.insert(
            0,
            format!(
                "{} repairable finding(s) — cynic lifecycle repair --yes",
                counts.repairable
            ),
        );
    }

    OpsReport {
        schema: UPDATE_LIFECYCLE_OPS_SCHEMA,
        status,
        generated_at_unix_ms: now_ms(),
        state_path: state_path.display().to_string(),
        phase: state.current_phase.as_str(),
        authority: format!("{:?}", policy.harness.authority),
        black_box,
        findings,
        finding_counts: counts,
        next,
        rules: vec![
            "ops report is observe-only; repairs require --yes",
            "repairs are filtered by current lifecycle phase mutation classes",
            "black_box=false means at least one finding was observed",
            "no silent auto-repair",
            "observation uses ArgvRunner — no domain-owned shell composition",
        ],
    }
}

fn collect_deps_findings(
    runner: &dyn ArgvRunner,
    deps_allows: bool,
    findings: &mut Vec<OpsFinding>,
) {
    let out = runner.run(&["cynic", "deps", "--json"]);
    if let Some(err) = out.spawn_error {
        findings.push(OpsFinding {
            id: "deps.unreachable".into(),
            severity: FindingSeverity::Error,
            surface: "deps".into(),
            summary: "failed to observe cynic deps".into(),
            evidence: err,
            repair_id: None,
            repair_command: None,
            phase_allows_repair: false,
        });
        return;
    }
    let Some(v) = parse_json(&out.stdout) else {
        findings.push(OpsFinding {
            id: "deps.parse".into(),
            severity: FindingSeverity::Warning,
            surface: "deps".into(),
            summary: "deps JSON missing/invalid".into(),
            evidence: out.stdout.chars().take(120).collect(),
            repair_id: None,
            repair_command: None,
            phase_allows_repair: false,
        });
        return;
    };
    let Some(sum) = v.get("summary") else {
        return;
    };
    let missing = sum.get("missing").and_then(|x| x.as_u64()).unwrap_or(0);
    let desired = sum
        .get("desired_upgrade")
        .and_then(|x| x.as_u64())
        .unwrap_or(0);
    let blocked = sum
        .get("required_blocked")
        .and_then(|x| x.as_u64())
        .unwrap_or(0);

    if blocked > 0 {
        findings.push(OpsFinding {
            id: "deps.required_blocked".into(),
            severity: FindingSeverity::Critical,
            surface: "deps".into(),
            summary: format!("{blocked} required dep(s) blocked (missing/stale)"),
            evidence: sum.to_string(),
            repair_id: Some("deps_upgrade".into()),
            repair_command: Some("cynic deps upgrade --yes".into()),
            phase_allows_repair: deps_allows,
        });
    }
    if desired > 0 {
        findings.push(OpsFinding {
            id: "deps.desired_upgrade".into(),
            severity: FindingSeverity::Warning,
            surface: "deps".into(),
            summary: format!("{desired} dep(s) below desired pin"),
            evidence: sum.to_string(),
            repair_id: Some("deps_upgrade".into()),
            repair_command: Some("cynic deps upgrade --yes".into()),
            phase_allows_repair: deps_allows,
        });
    }
    if missing > 0 && blocked == 0 {
        findings.push(OpsFinding {
            id: "deps.optional_missing".into(),
            severity: FindingSeverity::Info,
            surface: "deps".into(),
            summary: format!("{missing} optional/missing dep(s)"),
            evidence: sum.to_string(),
            repair_id: Some("deps_upgrade".into()),
            repair_command: Some("cynic deps upgrade --yes".into()),
            phase_allows_repair: deps_allows,
        });
    }
    if let Some(arr) = v.get("deps").and_then(|d| d.as_array()) {
        for d in arr {
            let id = d.get("id").and_then(|x| x.as_str()).unwrap_or("?");
            let st = d.get("status").and_then(|x| x.as_str()).unwrap_or("?");
            if matches!(st, "missing" | "stale" | "desired_upgrade") {
                findings.push(OpsFinding {
                    id: format!("deps.item.{id}"),
                    severity: if st == "missing" {
                        FindingSeverity::Warning
                    } else {
                        FindingSeverity::Info
                    },
                    surface: "deps".into(),
                    summary: format!("dep `{id}` status={st}"),
                    evidence: d.to_string().chars().take(240).collect(),
                    repair_id: Some(format!("deps_only_{id}")),
                    repair_command: Some(format!("cynic deps upgrade --yes --only {id}")),
                    phase_allows_repair: deps_allows,
                });
            }
        }
    }
}

fn collect_inference_findings(
    runner: &dyn ArgvRunner,
    inf_allows: bool,
    findings: &mut Vec<OpsFinding>,
) {
    let out = runner.run(&["cynic", "runtime", "inference", "--json"]);
    if out.spawn_error.is_some() {
        findings.push(OpsFinding {
            id: "inference.unreachable".into(),
            severity: FindingSeverity::Error,
            surface: "inference".into(),
            summary: "failed to observe inference config".into(),
            evidence: out.spawn_error.unwrap_or_default(),
            repair_id: None,
            repair_command: None,
            phase_allows_repair: false,
        });
        return;
    }
    if let Some(v) = parse_json(&out.stdout) {
        let st = v
            .get("status")
            .and_then(|x| x.as_str())
            .unwrap_or("unknown");
        let path = v.get("config_path").and_then(|x| x.as_str()).unwrap_or("-");
        if st == "not_found" || st == "error" || st == "blocked" {
            findings.push(OpsFinding {
                id: "inference.config".into(),
                severity: FindingSeverity::Error,
                surface: "inference".into(),
                summary: format!("inference SSOT status={st}"),
                evidence: format!("path={path}"),
                repair_id: Some("inference_discover".into()),
                repair_command: Some("cynic runtime inference-discover --json".into()),
                phase_allows_repair: inf_allows,
            });
        } else {
            let n = v
                .get("backends")
                .and_then(|b| b.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            if n == 0 {
                findings.push(OpsFinding {
                    id: "inference.no_backends".into(),
                    severity: FindingSeverity::Warning,
                    surface: "inference".into(),
                    summary: "inference config has zero backends".into(),
                    evidence: format!("path={path}"),
                    repair_id: Some("inference_discover".into()),
                    repair_command: Some("cynic runtime inference-discover --json".into()),
                    phase_allows_repair: inf_allows,
                });
            } else {
                findings.push(OpsFinding {
                    id: "inference.ok".into(),
                    severity: FindingSeverity::Info,
                    surface: "inference".into(),
                    summary: format!("inference SSOT ok backends={n}"),
                    evidence: format!("path={path}"),
                    repair_id: None,
                    repair_command: None,
                    phase_allows_repair: false,
                });
            }
        }
    }

    let out = runner.run(&["cynic", "runtime", "inference-status", "--json"]);
    if let Some(v) = parse_json(&out.stdout) {
        let st = v.get("state").and_then(|x| x.as_str()).unwrap_or("?");
        if st == "inactive" || st == "failed" {
            // Frictionless: auto ensure+select inside inference-start (no product model hardcode).
            findings.push(OpsFinding {
                id: "inference.unit_inactive".into(),
                severity: FindingSeverity::Warning,
                surface: "inference".into(),
                summary: format!("native inference unit state={st}"),
                evidence: v.to_string().chars().take(200).collect(),
                repair_id: Some("inference_start_auto".into()),
                repair_command: Some(
                    "CYNIC_INFERENCE_SUPERVISOR=process cynic runtime inference-start auto --json"
                        .into(),
                ),
                phase_allows_repair: inf_allows,
            });
        }
    }
}

fn collect_surface_findings(runner: &dyn ArgvRunner, findings: &mut Vec<OpsFinding>) {
    let out = runner.run(&["cynic", "dev", "surface-audit", "--json"]);
    let Some(v) = parse_json(&out.stdout) else {
        return;
    };
    let st = v.get("status").and_then(|x| x.as_str()).unwrap_or("?");
    let blocked = v.get("blocked_count").and_then(|x| x.as_u64()).unwrap_or(0);
    if st != "ok" || blocked > 0 {
        findings.push(OpsFinding {
            id: "surfaces.blocked".into(),
            severity: FindingSeverity::Error,
            surface: "surface_audit".into(),
            summary: format!("surface-audit status={st} blocked={blocked}"),
            evidence: format!("exit={:?}", out.exit_code),
            repair_id: None,
            repair_command: Some("cynic dev surface-audit --json".into()),
            phase_allows_repair: false,
        });
    } else {
        findings.push(OpsFinding {
            id: "surfaces.ok".into(),
            severity: FindingSeverity::Info,
            surface: "surface_audit".into(),
            summary: "canonical surfaces ok".into(),
            evidence: format!("exit={:?}", out.exit_code),
            repair_id: None,
            repair_command: None,
            phase_allows_repair: false,
        });
    }
}
