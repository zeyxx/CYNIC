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

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{emit_json, rust_source};

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