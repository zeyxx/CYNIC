use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::bootstrap_surface::ProfileArgs;
use crate::bootstrap_surface::selected_host_context;
use crate::fs_json::write_pretty_json_file;
use crate::kernel_boundary_audit::audit_kernel_surfaces;
use crate::ownership_audit::audit_ownership_surfaces;
use crate::surface_audit::{SurfaceAuditReport, audit_canonical_surfaces};
use crate::{emit_json, fail};

#[derive(Serialize)]
pub(crate) struct SurfaceAuditMatrixReport {
    pub(crate) schema: &'static str,
    pub(crate) generated_at_unix_ms: u128,
    pub(crate) status: &'static str,
    pub(crate) source: &'static str,
    pub(crate) binary: String,
    pub(crate) summary: SurfaceAuditMatrixSummary,
    pub(crate) surfaces: Vec<crate::surface_audit::SurfaceAuditEntry>,
    pub(crate) external_projection_findings: Vec<crate::surface_audit::ExternalProjectionFinding>,
    pub(crate) agent_exposure: crate::agent_exposure_audit::AgentExposureAuditReport,
    pub(crate) ownership_audit: OwnershipAuditBundle,
    pub(crate) runtime_phases: Vec<RuntimePhase>,
    pub(crate) kernel_audit: Option<KernelAuditBundle>,
    pub(crate) markdown: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct SurfaceAuditMatrixSummary {
    pub(crate) surface_count: usize,
    pub(crate) ok_count: usize,
    pub(crate) blocked_count: usize,
    pub(crate) owner_count: usize,
    pub(crate) runtime_phase_count: usize,
    pub(crate) runtime_phase_ok_count: usize,
    pub(crate) status: &'static str,
}

#[derive(Serialize)]
pub(crate) struct OwnershipAuditBundle {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) summary: crate::ownership_audit::OwnershipAuditSummary,
    pub(crate) surfaces: Vec<crate::ownership_audit::OwnershipSurfaceAudit>,
}

#[derive(Serialize)]
pub(crate) struct KernelAuditBundle {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) total_rust_file_count: usize,
    pub(crate) covered_file_count: usize,
    pub(crate) uncovered_file_count: usize,
    pub(crate) next_step_count: usize,
    pub(crate) extract_count: usize,
    pub(crate) stabilize_count: usize,
}

#[derive(Serialize)]
pub(crate) struct RuntimePhase {
    pub(crate) phase: &'static str,
    pub(crate) status: String,
    pub(crate) schema: String,
    pub(crate) mutates: Option<bool>,
    pub(crate) command: &'static str,
    pub(crate) exit_code: i32,
    pub(crate) parse_error: Option<String>,
}

#[derive(Debug)]
enum PhaseResult {
    Json {
        status: String,
        schema: String,
        mutates: Option<bool>,
    },
}

const MATRIX_DEFAULT_DIR: &str = "/var/lib/cynic/reports";
const MATRIX_FALLBACK_DIR: &str = "/tmp/cynic/reports";
const MATRIX_MATRIX_BASENAME: &str = "cynic-surface-audit-matrix";

pub(crate) fn execute_surface_matrix(
    json: bool,
    output: Option<PathBuf>,
    persist: bool,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }

    let report = build_surface_matrix();
    let markdown = render_matrix_markdown(&report);
    let should_fail = report.status != "ok";

    if let Err(error) = maybe_persist_matrix_report(output, &report, &markdown, persist) {
        return fail(error);
    }

    emit_json(&matrix_report_with_markdown(report, markdown), should_fail)
}

pub(crate) fn build_surface_matrix() -> SurfaceAuditMatrixReport {
    let start = std::env::current_exe()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| "cynic".to_string());
    let surface_report = audit_canonical_surfaces();
    let ownership_report = audit_ownership_surfaces();
    let kernel_report = audit_kernel_surfaces();

    let runtime_phases = run_runtime_phases();
    let runtime_phase_ok_count = runtime_phases
        .iter()
        .filter(|phase| phase.status == "ok")
        .count();
    let surface_summary = SurfaceAuditMatrixSummary {
        surface_count: surface_report.surface_count,
        ok_count: surface_report.ok_count,
        blocked_count: surface_report.blocked_count,
        owner_count: count_surface_owners(&surface_report),
        runtime_phase_count: runtime_phases.len(),
        runtime_phase_ok_count,
        status: if surface_report.status == "ok"
            && ownership_report.status == "ok"
            && ownership_report.summary.project_root_dependency_count == 0
            && runtime_phases
                .iter()
                .all(|phase| phase.status == "ok" && phase.exit_code == 0)
            && kernel_report.status == "ok"
        {
            "ok"
        } else {
            "blocked"
        },
    };

    SurfaceAuditMatrixReport {
        schema: "cynic.cli.surface_audit_matrix.v1",
        generated_at_unix_ms: now_unix_ms(),
        status: surface_summary.status,
        source: "binary",
        binary: start,
        summary: surface_summary,
        surfaces: surface_report.surfaces,
        external_projection_findings: surface_report.external_projection_findings,
        agent_exposure: surface_report.agent_exposure,
        ownership_audit: OwnershipAuditBundle {
            schema: ownership_report.schema,
            status: ownership_report.status,
            summary: ownership_report.summary,
            surfaces: ownership_report.surfaces,
        },
        runtime_phases,
        kernel_audit: Some(KernelAuditBundle {
            schema: kernel_report.schema,
            status: kernel_report.status,
            total_rust_file_count: kernel_report.total_rust_file_count,
            covered_file_count: kernel_report.covered_file_count,
            uncovered_file_count: kernel_report.uncovered_file_count,
            next_step_count: kernel_report.next_steps.len(),
            extract_count: kernel_report
                .areas
                .iter()
                .filter(|area| area.status == "extract")
                .count(),
            stabilize_count: kernel_report
                .areas
                .iter()
                .filter(|area| area.status == "stabilize")
                .count(),
        }),
        markdown: None,
    }
}

fn run_runtime_phases() -> Vec<RuntimePhase> {
    let profile = ProfileArgs {
        profile: None,
        node: None,
        registry: None,
        json: false,
    };

    vec![
        run_runtime_phase("runtime_harness", "cynic runtime harness --json", || {
            cynic_host::runtime_harness::runtime_harness_catalog(None)
                .map(
                    |report: cynic_host::runtime_harness::RuntimeHarnessCatalogReport| {
                        let schema = report.schema.to_string();
                        PhaseResult::Json {
                            status: report.status.to_string(),
                            schema,
                            mutates: None,
                        }
                    },
                )
                .map_err(|error| error.to_string())
        }),
        run_runtime_phase(
            "runtime_conformance",
            "cynic runtime conformance --json",
            || {
                let (selected, resolution) =
                    selected_host_context(&profile).map_err(|error| error.to_string())?;
                cynic_host::conformance::observe_host_conformance(selected, resolution)
                    .map(|report| {
                        let schema = report.schema.to_string();
                        let status = report.status.to_string();
                        PhaseResult::Json {
                            status,
                            schema,
                            mutates: Some(false),
                        }
                    })
                    .map_err(|error| error.to_string())
            },
        ),
        run_runtime_phase("runtime_metrics", "cynic runtime metrics --json", || {
            let (selected, resolution) =
                selected_host_context(&profile).map_err(|error| error.to_string())?;
            let conformance =
                cynic_host::conformance::observe_host_conformance(selected, resolution)
                    .map_err(|error| error.to_string())?;
            let metrics = cynic_host::runtime_metrics::host_runtime_metrics(&conformance);
            let schema = metrics.schema.to_string();
            let status = metrics.status.to_string();
            Ok(PhaseResult::Json {
                status,
                schema,
                mutates: Some(false),
            })
        }),
    ]
}

fn run_runtime_phase(
    phase: &'static str,
    command: &'static str,
    mut run: impl FnMut() -> Result<PhaseResult, String>,
) -> RuntimePhase {
    match run() {
        Ok(PhaseResult::Json {
            status,
            schema,
            mutates,
        }) => {
            let normalized_status = if status.trim().is_empty() {
                "ok"
            } else {
                status.as_str()
            };
            RuntimePhase {
                phase,
                status: normalized_status.to_string(),
                schema,
                mutates,
                command,
                exit_code: 0,
                parse_error: None,
            }
        }
        Err(error) => RuntimePhase {
            phase,
            status: "blocked".to_string(),
            schema: "unknown".to_string(),
            mutates: None,
            command,
            exit_code: 1,
            parse_error: Some(error),
        },
    }
}

fn count_surface_owners(report: &SurfaceAuditReport) -> usize {
    let mut owners = BTreeMap::<String, usize>::new();
    for surface in &report.surfaces {
        *owners.entry(surface.owner_crate.clone()).or_insert(0) += 1;
    }
    owners.len()
}

fn matrix_report_with_markdown(
    mut report: SurfaceAuditMatrixReport,
    markdown: String,
) -> SurfaceAuditMatrixReport {
    report.markdown = Some(markdown);
    report
}

fn maybe_persist_matrix_report(
    output: Option<PathBuf>,
    report: &SurfaceAuditMatrixReport,
    markdown: &str,
    persist: bool,
) -> Result<(), String> {
    if !persist && output.is_none() {
        return Ok(());
    }

    let explicit_output = output.is_some();
    let report_dir_env = std::env::var("CYNIC_SURFACE_AUDIT_REPORT_DIR").ok();
    let uses_default_dir = report_dir_env.is_none();
    let default_dir = report_dir_env.as_deref().unwrap_or(MATRIX_DEFAULT_DIR);

    let (json_path, md_path) = matrix_output_paths(output.as_deref(), default_dir);
    if let Err(error) = write_matrix_files(&json_path, &md_path, report, markdown) {
        if explicit_output {
            return Err(error);
        }

        if !uses_default_dir || output.is_some() {
            return Err(error);
        }

        let fallback_dir = Path::new(MATRIX_FALLBACK_DIR);
        let (fallback_json, fallback_md) =
            matrix_output_paths(Some(fallback_dir), MATRIX_FALLBACK_DIR);
        write_matrix_files(&fallback_json, &fallback_md, report, markdown).map_err(
            |fallback_error| {
                format!(
                    "{error}; fallback {} failed: {fallback_error}",
                    fallback_dir.display()
                )
            },
        )
    } else {
        Ok(())
    }
}

fn matrix_output_paths(output: Option<&Path>, default_dir: &str) -> (PathBuf, PathBuf) {
    output
        .filter(|path| is_file_like_path(path))
        .map(output_file_paths)
        .unwrap_or_else(|| {
            let directory = if let Some(path) = output {
                path.to_path_buf()
            } else {
                PathBuf::from(default_dir)
            };
            (
                directory.join(format!("{MATRIX_MATRIX_BASENAME}.json")),
                directory.join(format!("{MATRIX_MATRIX_BASENAME}.md")),
            )
        })
}

fn output_file_paths(path: &Path) -> (PathBuf, PathBuf) {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("md") => {
            let mut json_path = path.to_path_buf();
            json_path.set_extension("json");
            (json_path, path.to_path_buf())
        }
        _ => {
            let mut md_path = path.to_path_buf();
            md_path.set_extension("md");
            (path.to_path_buf(), md_path)
        }
    }
}

fn is_file_like_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| matches!(ext.to_str(), Some("json") | Some("md")))
        || path.is_file()
}

fn write_matrix_files(
    json_path: &Path,
    md_path: &Path,
    report: &SurfaceAuditMatrixReport,
    markdown: &str,
) -> Result<(), String> {
    write_pretty_json_file(json_path, report, "matrix report JSON")?;

    std::fs::write(md_path, markdown).map_err(|error| {
        format!(
            "failed to write matrix report markdown {}: {error}",
            md_path.display()
        )
    })?;

    Ok(())
}

fn render_matrix_markdown(report: &SurfaceAuditMatrixReport) -> String {
    let mut lines = vec![
        "# Cynic Surface Matrix".to_string(),
        String::new(),
        format!("- Generated at unix ms: {}", report.generated_at_unix_ms),
        format!("- Source: `{}` via `{}`", report.source, report.binary),
        format!(
            "- Summary: surfaces={} ok={} blocked={}",
            report.summary.surface_count, report.summary.ok_count, report.summary.blocked_count
        ),
        format!(
            "- Runtime phases: {}/{} ok",
            report.summary.runtime_phase_ok_count, report.summary.runtime_phase_count
        ),
        String::new(),
        "## Runtime Phases".to_string(),
        "| Phase | Status | Schema | Mutates | Command |".to_string(),
        "| --- | --- | --- | --- | --- |".to_string(),
    ];

    for phase in &report.runtime_phases {
        lines.push(format!(
            "| {} | {} | {} | {} | {} |",
            phase.phase,
            phase.status,
            phase.schema,
            phase
                .mutates
                .map(|value| if value { "true" } else { "false" })
                .unwrap_or("unknown"),
            phase.command
        ));
    }

    lines.push(String::new());
    lines.push("## Ownership Matrix".to_string());
    lines.push("| Owner Crate | Surface Count |".to_string());
    lines.push("| --- | ---: |".to_string());

    let mut owner_counts = BTreeMap::<String, usize>::new();
    for surface in &report.surfaces {
        *owner_counts.entry(surface.owner_crate.clone()).or_insert(0) += 1;
    }
    for (owner, count) in owner_counts {
        lines.push(format!("| {} | {} |", owner, count));
    }

    lines.join("\n")
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
