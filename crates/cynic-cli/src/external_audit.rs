//! Read-back surface for the external (non-CYNIC) project registry.
//!
//! WHY: projects outside the CYNIC repository (aegisscan, aegis-core, ai-bus)
//! are governed through the same ports as CYNIC itself. This surface projects
//! `infra/external-projects.json` (SSOT) so an operator or agent can list and
//! inspect them without reading raw JSON.
//! HOW: read the declarative registry, project its fields into a typed report,
//! emit JSON via emit_json or text via render_text.
//! NEGATIVE BOUNDARY: read-only. Never mutates the registry, never clones,
//! builds, or deploys an external project. No network I/O.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{emit_json, fail, rust_source};

// External audit orchestration imports
use cynic_build::{ExternalBuildKind, HttpExternalBuildAdapter};
use cynic_host::adapters::local_evidence_store::LocalContentAddressedEvidenceStore;
use cynic_kernel::ports::external_evidence::{ExternalEvidenceBatch, ExternalEvidencePort};
use cynic_kernel::ports::external_evidence_adapter::LocalExternalEvidenceAdapter;
use cynic_node::usecases::external_project_audit_uc::{
    audit_external_project, ExternalProjectAuditRequest,
};
use cynic_vcs::{ExternalProvider, ExternalRepoRef, ExternalSourcePort};
use cynic_vcs_providers::{ForgejoExternalSourceAdapter, GitHubExternalSourceAdapter};

// ── External project audit orchestration ───────────────────────────────────────

/// Instantiate adapters and run the full external project audit pipeline.
///
/// FCIS: this is a thin surface. It reads the registry, instantiates the
/// three port adapters, constructs the request, and delegates to the pure
/// orchestration usecase. No business logic lives here.
#[allow(clippy::too_many_arguments)]
pub fn execute_run(
    json: bool,
    project: Option<&str>,
    builder_url: Option<&str>,
    forgejo_url: Option<&str>,
    forgejo_token: Option<&str>,
    github_token: Option<&str>,
    evidence_dir: Option<&str>,
) -> ExitCode {
    let root = rust_source::workspace_root();
    let registry_path = root.join("infra/external-projects.json");

    // Read the registry
    let raw = match fs::read_to_string(&registry_path) {
        Ok(s) => s,
        Err(e) => return fail(format!("cannot read registry {}: {}", registry_path.display(), e)),
    };

    let value: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => return fail(format!("invalid JSON in registry: {}", e)),
    };

    // Find the specified project
    let project_id = match project {
        Some(p) => p.to_string(),
        None => return fail("--project is required with --run"),
    };

    let projects = value
        .get("projects")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let project_data = projects
        .iter()
        .find(|p| p.get("project_id").and_then(|x| x.as_str()) == Some(project_id.as_str()));

    let project_data = match project_data {
        Some(p) => p,
        None => return fail(format!("project not found in registry: {}", project_id)),
    };

    // Extract project fields
    let repo_url = str_field(project_data, "repo_url");
    let default_branch = str_field(project_data, "default_branch");
    let mut build_commands = str_vec(project_data, "build_commands");
    let test_commands = str_vec(project_data, "test_commands");
    build_commands.extend(test_commands);

    // Determine provider from repo URL
    let provider = if repo_url.contains("github.com") {
        ExternalProvider::GitHub
    } else if forgejo_url.is_some() {
        ExternalProvider::Forgejo
    } else {
        return fail("cannot determine provider from repo URL (use --forgejo-url for Forgejo)");
    };

    // Instantiate source adapter
    let source_port: Box<dyn ExternalSourcePort> = match provider {
        ExternalProvider::GitHub => {
            let token = match github_token
                .map(|s| s.to_string())
                .or_else(|| cynic_secrets::env_var("github-token", "GITHUB_TOKEN"))
            {
                Some(t) => t,
                None => return fail("GITHUB_TOKEN is required for GitHub provider"),
            };
            Box::new(GitHubExternalSourceAdapter::new(
                "https://api.github.com".to_string(),
                token,
            ))
        }
        ExternalProvider::Forgejo => {
            let base = match forgejo_url {
                Some(url) => url.to_string(),
                None => return fail("--forgejo-url is required for Forgejo provider"),
            };
            let token = match forgejo_token
                .map(|s| s.to_string())
                .or_else(|| cynic_secrets::env_var("forgejo-token", "FORGEJO_TOKEN"))
            {
                Some(t) => t,
                None => return fail("FORGEJO_TOKEN is required for Forgejo provider"),
            };
            Box::new(ForgejoExternalSourceAdapter::new(base, token))
        }
        _ => return fail("unsupported provider"),
    };

    // Instantiate build adapter
    let build_port = match builder_url {
        Some(url) => HttpExternalBuildAdapter::new(url.to_string()),
        None => match HttpExternalBuildAdapter::from_resolved_url() {
            Ok(adapter) => adapter,
            Err(e) => return fail(e),
        },
    };

    // Instantiate evidence adapter
    let evidence_path = evidence_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".cynic").join("evidence"));
    let store = LocalContentAddressedEvidenceStore::new(&evidence_path);
    let evidence_port = LocalExternalEvidenceAdapter::new(store);

    // Construct evidence batches from registry observation data
    let evidence_batches = construct_evidence_batches(project_data, &project_id);

    // Build the audit request
    let req = ExternalProjectAuditRequest {
        project_id: project_id.clone(),
        repo: ExternalRepoRef {
            project_id: project_id.clone(),
            remote_url: repo_url,
            default_branch,
            provider,
        },
        build_commands,
        build_kind: ExternalBuildKind::Build,
        builder_node_id: "cynic-builder".to_string(),
        evidence_batches,
    };

    // Call the pure orchestration usecase
    let report = audit_external_project(source_port.as_ref(), &build_port, &evidence_port, &req);

    // Emit the report
    let blocked = !report.overall_success;
    if json {
        emit_json(&report, blocked)
    } else {
        println!(
            "External project audit: {} (overall: {})",
            project_id,
            if report.overall_success { "SUCCESS" } else { "FAILURE" }
        );
        println!(
            "  Clone: {}",
            if report.clone_result.success { "ok" } else { "failed" }
        );
        println!(
            "  Build: {}/{} commands passed",
            report.build_proof.commands_passed, report.build_proof.commands_run
        );
        println!(
            "  Evidence: {}/{} batches ingested",
            report.evidence_proof.batches_ingested,
            report.evidence_proof.batches_ingested + report.evidence_proof.batches_failed
        );
        if blocked {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}

/// Construct evidence batches from the registry's observation data.
fn construct_evidence_batches(project_data: &Value, project_id: &str) -> Vec<ExternalEvidenceBatch> {
    if let Some(obs) = project_data.get("observation") {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        vec![ExternalEvidenceBatch {
            project_id: project_id.to_string(),
            producer: "cynic.audit.external.v1".to_string(),
            subject: "build-observation".to_string(),
            observed_at_unix_ms: now_ms,
            provenance: vec![str_field(project_data, "repo_url")],
            payload: obs.clone(),
        }]
    } else {
        vec![]
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalProjectView {
    pub project_id: String,
    pub display_name: String,
    pub repo_url: String,
    pub default_branch: String,
    pub language_stack: String,
    pub collaborator_ids: Vec<String>,
    pub mutation_policy: String,
    pub sensitivity: String,
    pub build_commands: Vec<String>,
    pub test_commands: Vec<String>,
    pub build_commands_status: Option<String>,
    pub ports_used: Vec<String>,
    pub port_status: Option<BTreeMap<String, String>>,
    pub port_wiring_status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalProjectsReport {
    pub schema: String,
    pub status: String,
    pub registry_path: String,
    pub project_count: usize,
    pub projects: Vec<ExternalProjectView>,
    pub residual_unknowns: Vec<String>,
}

fn str_field(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn str_vec(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn opt_str(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(str::to_string)
}

fn opt_str_map(v: &Value, key: &str) -> Option<BTreeMap<String, String>> {
    v.get(key).and_then(|x| x.as_object()).map(|obj| {
        obj.iter()
            .filter_map(|(k, val)| val.as_str().map(|s| (k.clone(), s.to_string())))
            .collect()
    })
}

pub fn scan(registry_path: &Path) -> ExternalProjectsReport {
    let mut residual: Vec<String> = Vec::new();
    let registry_path_str = registry_path.display().to_string();

    if !registry_path.exists() {
        residual.push(format!("registry not found: {}", registry_path_str));
        return ExternalProjectsReport {
            schema: "cynic.audit.external.v1".to_string(),
            status: "blocked".to_string(),
            registry_path: registry_path_str,
            project_count: 0,
            projects: vec![],
            residual_unknowns: residual,
        };
    }

    let raw = match fs::read_to_string(registry_path) {
        Ok(s) => s,
        Err(e) => {
            residual.push(format!("cannot read registry {}: {}", registry_path_str, e));
            return ExternalProjectsReport {
                schema: "cynic.audit.external.v1".to_string(),
                status: "blocked".to_string(),
                registry_path: registry_path_str,
                project_count: 0,
                projects: vec![],
                residual_unknowns: residual,
            };
        }
    };

    let value: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            residual.push(format!("invalid JSON in {}: {}", registry_path_str, e));
            return ExternalProjectsReport {
                schema: "cynic.audit.external.v1".to_string(),
                status: "blocked".to_string(),
                registry_path: registry_path_str,
                project_count: 0,
                projects: vec![],
                residual_unknowns: residual,
            };
        }
    };

    let projects_arr = value
        .get("projects")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();

    let mut projects = Vec::new();
    for p in &projects_arr {
        projects.push(ExternalProjectView {
            project_id: str_field(p, "project_id"),
            display_name: str_field(p, "display_name"),
            repo_url: str_field(p, "repo_url"),
            default_branch: str_field(p, "default_branch"),
            language_stack: str_field(p, "language_stack"),
            collaborator_ids: str_vec(p, "collaborator_ids"),
            mutation_policy: str_field(p, "mutation_policy"),
            sensitivity: str_field(p, "sensitivity"),
            build_commands: str_vec(p, "build_commands"),
            test_commands: str_vec(p, "test_commands"),
            build_commands_status: opt_str(p, "build_commands_status"),
            ports_used: str_vec(p, "ports_used"),
            port_status: opt_str_map(p, "port_status"),
            port_wiring_status: opt_str(p, "port_wiring_status"),
        });
    }

    let status = if residual.is_empty() {
        "ok".to_string()
    } else {
        "blocked".to_string()
    };

    ExternalProjectsReport {
        schema: "cynic.audit.external.v1".to_string(),
        status,
        registry_path: registry_path_str,
        project_count: projects.len(),
        projects,
        residual_unknowns: residual,
    }
}

pub fn render_text(report: &ExternalProjectsReport) {
    println!(
        "External projects: {} (status: {})",
        report.project_count, report.status
    );
    println!("Registry: {}", report.registry_path);
    for p in &report.projects {
        println!(
            "  - {} ({}) [{}] {}",
            p.project_id, p.language_stack, p.sensitivity, p.mutation_policy
        );
        println!("      repo: {} ({})", p.repo_url, p.default_branch);
        if !p.ports_used.is_empty() {
            println!("      ports: {}", p.ports_used.join(", "));
        }
        if let Some(s) = &p.build_commands_status {
            println!("      build status: {}", s);
        }
        if let Some(ps) = &p.port_status {
            println!("      port wiring: {}", p.port_wiring_status.as_deref().unwrap_or("unknown"));
            for (port, status) in ps {
                println!("        {} => {}", port, status);
            }
        }
    }
    if !report.residual_unknowns.is_empty() {
        println!("Residual unknowns:");
        for u in &report.residual_unknowns {
            println!("  - {}", u);
        }
    }
}

pub fn execute(json: bool, registry: Option<&str>) -> ExitCode {
    let root = rust_source::workspace_root();
    let default_path = root.join("infra/external-projects.json");
    let registry_path = registry.map(PathBuf::from).unwrap_or(default_path);
    let report = scan(&registry_path);
    let blocked = report.status != "ok" || !report.residual_unknowns.is_empty();
    if json {
        emit_json(&report, blocked)
    } else {
        render_text(&report);
        if blocked {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}