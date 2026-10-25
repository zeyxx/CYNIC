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
use cynic_kernel::ports::external_evidence::ExternalEvidenceBatch;
use cynic_kernel::ports::external_evidence_adapter::LocalExternalEvidenceAdapter;
use cynic_node::usecases::external_project_audit_uc::{
    audit_external_project, ExternalProjectAuditRequest,
};
use cynic_vcs::{ExternalProvider, ExternalRepoRef, ExternalSourcePort};
use cynic_vcs::providers::{ForgejoExternalSourceAdapter, GitHubExternalSourceAdapter};
use cynic_types::paths::CynicPaths;

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

    // Resolve builder URL up-front so the source adapter clones on the builder.
    let resolved_builder_url = match builder_url {
        Some(url) => url.to_string(),
        None => match HttpExternalBuildAdapter::from_resolved_url() {
            Ok(adapter) => adapter.builder_url,
            Err(e) => return fail(e),
        },
    };

    // Instantiate source adapter (clone on builder when builder_url is set)
    let mut source_port: Box<dyn ExternalSourcePort> = match provider {
        ExternalProvider::GitHub => {
            let token = match github_token
                .map(|s| s.to_string())
                .or_else(|| cynic_secrets::env_var("github-token", "GITHUB_TOKEN"))
            {
                Some(t) => t,
                None => return fail("GITHUB_TOKEN is required for GitHub provider"),
            };
            let mut adapter = GitHubExternalSourceAdapter::new(
                "https://api.github.com".to_string(),
                token,
            );
            adapter.builder_url = Some(resolved_builder_url.clone());
            Box::new(adapter)
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
            let mut adapter = ForgejoExternalSourceAdapter::new(base, token);
            adapter.builder_url = Some(resolved_builder_url.clone());
            Box::new(adapter)
        }
        _ => return fail("unsupported provider"),
    };

    // Instantiate build adapter
    let build_port = HttpExternalBuildAdapter::new(resolved_builder_url);

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
    // Governance (serving tier): from normative registry + pins.
    pub serving_tier: Option<String>,
    pub checkout_root: Option<String>,
    pub pin_sha: Option<String>,
    pub pin_repo: Option<String>,
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

    // Detect schema: normative registry uses "slug", legacy seed uses "project_id".
    let is_normative = value
        .get("projects")
        .and_then(|x| x.as_array())
        .and_then(|arr| arr.first())
        .map(|p| p.get("slug").is_some())
        .unwrap_or(false);

    // Load pins from the sibling pins file (if present) to join SHA/repo.
    let pins_path = registry_path
        .parent()
        .map(|p| p.join("external-pins.json"))
        .unwrap_or_else(|| PathBuf::from("infra/external-pins.json"));
    let pins_value: Option<Value> = fs::read_to_string(&pins_path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok());
    let pin_for = |slug: &str| -> (Option<String>, Option<String>) {
        pins_value
            .as_ref()
            .and_then(|v| v.get("pins").and_then(|x| x.as_array()))
            .and_then(|arr| {
                arr.iter()
                    .find(|p| p.get("slug").and_then(|x| x.as_str()) == Some(slug))
            })
            .map(|p| {
                let sha = p.get("sha").and_then(|x| x.as_str()).map(str::to_string);
                let repo = p.get("repo").and_then(|x| x.as_str()).map(str::to_string);
                (sha, repo)
            })
            .unwrap_or((None, None))
    };

    let projects_arr = value
        .get("projects")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();

    let mut projects = Vec::new();
    for p in &projects_arr {
        if is_normative {
            let slug = str_field(p, "slug");
            if slug.is_empty() {
                continue;
            }
            let serving_tier = opt_str(p, "serving_tier");
            let checkout_root = p
                .get("roots")
                .and_then(|r| r.get("referential"))
                .and_then(|x| x.as_str())
                .map(str::to_string);
            let (pin_sha, pin_repo) = pin_for(&slug);
            let sensitivity = if str_field(p, "sensitivity").is_empty() {
                "public".to_string()
            } else {
                str_field(p, "sensitivity")
            };
            let mutation_policy = if str_field(p, "mutation_policy").is_empty() {
                "read_only".to_string()
            } else {
                str_field(p, "mutation_policy")
            };
            let display_name = if str_field(p, "display_name").is_empty() {
                slug.clone()
            } else {
                str_field(p, "display_name")
            };
            let repo_url = str_field(p, "repo_url");
            let repo_url = if repo_url.is_empty() {
                pin_repo
                    .as_ref()
                    .map(|r| format!("https://github.com/{}", r))
                    .unwrap_or_default()
            } else {
                repo_url
            };
            projects.push(ExternalProjectView {
                project_id: slug.clone(),
                display_name,
                repo_url,
                default_branch: str_field(p, "default_branch"),
                language_stack: str_field(p, "language_stack"),
                collaborator_ids: str_vec(p, "collaborator_ids"),
                mutation_policy,
                sensitivity,
                build_commands: str_vec(p, "build_commands"),
                test_commands: str_vec(p, "test_commands"),
                build_commands_status: opt_str(p, "build_commands_status"),
                ports_used: str_vec(p, "ports_used"),
                port_status: opt_str_map(p, "port_status"),
                port_wiring_status: opt_str(p, "port_wiring_status"),
                serving_tier,
                checkout_root,
                pin_sha,
                pin_repo,
            });
        } else {
            // Legacy seed schema (project_id).
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
                serving_tier: None,
                checkout_root: None,
                pin_sha: None,
                pin_repo: None,
            });
        }
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
            "  - {} ({}) [{}] {}  tier={}",
            p.project_id,
            if p.language_stack.is_empty() { "?" } else { &p.language_stack },
            p.sensitivity,
            p.mutation_policy,
            p.serving_tier.as_deref().unwrap_or("-")
        );
        if !p.repo_url.is_empty() {
            println!("      repo: {} ({})", p.repo_url, p.default_branch);
        }
        if let Some(root) = &p.checkout_root {
            println!("      checkout: {}", root);
        }
        if let Some(sha) = &p.pin_sha {
            println!(
                "      pin: {} ({})",
                sha,
                p.pin_repo.as_deref().unwrap_or("")
            );
        }
        if !p.ports_used.is_empty() {
            println!("      ports: {}", p.ports_used.join(", "));
        }
        if let Some(s) = &p.build_commands_status {
            println!("      build status: {}", s);
        }
        if let Some(ps) = &p.port_status {
            println!(
                "      port wiring: {}",
                p.port_wiring_status.as_deref().unwrap_or("unknown")
            );
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
    let default_path = root.join("infra/external-project-registry.json");
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
// ── promote ──────────────────────────────────────────────────────────────

fn tier_root(tier: &str) -> Option<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    match tier {
        "served" => Some(
            CynicPaths::state_dir()
                .join("source/repositories/external")
                .display()
                .to_string(),
        ),
        "dev" => Some(format!("{}/.cynic/external", home)),
        _ => None,
    }
}

fn move_clone(old_root: &str, new_root: &str, slug: &str) -> Result<(), String> {
    let src = PathBuf::from(old_root).join(slug);
    let dst_parent = PathBuf::from(new_root);
    let dst = dst_parent.join(slug);
    if !src.exists() {
        return Err(format!("source clone not found: {}", src.display()));
    }
    if dst.exists() {
        return Err(format!("target clone already exists: {}", dst.display()));
    }
    fs::create_dir_all(&dst_parent).map_err(|e| format!("mkdir {}: {}", dst_parent.display(), e))?;
    // Try rename first (same filesystem fast path).
    if fs::rename(&src, &dst).is_ok() {
        return Ok(());
    }
    // Fallback: copy tree then remove source.
    copy_dir_recursive(&src, &dst).map_err(|e| format!("copy: {}", e))?;
    fs::remove_dir_all(&src).map_err(|e| format!("rm source: {}", e))?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        let to = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn promote(
    json: bool,
    slug: &str,
    to: Option<&str>,
    yes: bool,
    registry: Option<&str>,
) -> ExitCode {
    let root = rust_source::workspace_root();
    let default_path = root.join("infra/external-project-registry.json");
    let registry_path = registry.map(PathBuf::from).unwrap_or(default_path);

    let target = match to {
        Some(t) => t,
        None => {
            eprintln!("blocked: --to <served|dev|contribute> is required");
            return ExitCode::FAILURE;
        }
    };
    if !matches!(target, "served" | "dev" | "contribute") {
        eprintln!("blocked: unknown target tier '{}'", target);
        return ExitCode::FAILURE;
    }

    // Read registry.
    let raw = match fs::read_to_string(&registry_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("blocked: cannot read registry: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let mut value: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("blocked: invalid registry JSON: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let projects = match value.get_mut("projects").and_then(|x| x.as_array_mut()) {
        Some(a) => a,
        None => {
            eprintln!("blocked: registry has no projects array");
            return ExitCode::FAILURE;
        }
    };
    let idx = match projects
        .iter()
        .position(|p| p.get("slug").and_then(|x| x.as_str()) == Some(slug))
    {
        Some(i) => i,
        None => {
            eprintln!("blocked: slug '{}' not found in registry", slug);
            return ExitCode::FAILURE;
        }
    };

    let current_tier = projects[idx]
        .get("serving_tier")
        .and_then(|x| x.as_str())
        .unwrap_or("dev")
        .to_string();
    let current_mutation = projects[idx]
        .get("mutation_policy")
        .and_then(|x| x.as_str())
        .unwrap_or("read_only")
        .to_string();
    let current_root = projects[idx]
        .get("roots")
        .and_then(|r| r.get("referential"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    // Plan.
    let mut plan: Vec<String> = Vec::new();
    let mut clone_move: Option<(String, String)> = None; // (old_root, new_root)
    let mut new_tier = current_tier.clone();
    let mut new_mutation = current_mutation.clone();

    if target == "served" || target == "dev" {
        if target != current_tier {
            let new_root = tier_root(target).unwrap();
            clone_move = Some((current_root.clone(), new_root.clone()));
            new_tier = target.to_string();
            plan.push(format!(
                "serving_tier: {} -> {} (clone move {} -> {})",
                current_tier, target, current_root, new_root
            ));
        } else {
            plan.push(format!("serving_tier already {} (no clone move)", current_tier));
        }
    } else {
        // contribute: mutation_policy read_only -> branch_pr
        if current_mutation == "read_only" {
            new_mutation = "branch_pr".to_string();
            plan.push(format!(
                "mutation_policy: {} -> branch_pr (contribute mode)",
                current_mutation
            ));
        } else {
            plan.push(format!(
                "mutation_policy already {} (no change)",
                current_mutation
            ));
        }
    }

    // Dry-run: print plan and exit.
    if !yes {
        if json {
            let out = serde_json::json!({
                "schema": "cynic.audit.external.promote.v1",
                "status": "dry_run",
                "slug": slug,
                "target": target,
                "plan": plan,
                "requires_yes": true,
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
        } else {
            println!("promote {} -> {} (dry run)", slug, target);
            for p in &plan {
                println!("  - {}", p);
            }
            println!("re-run with --yes to apply");
        }
        return ExitCode::SUCCESS;
    }

    // Apply clone move if needed.
    if let Some((old_root, new_root)) = &clone_move {
        // Writability check for served target.
        if new_root.starts_with("/var/lib") {
            let parent = PathBuf::from(new_root);
            if !parent.exists() || fs::metadata(&parent).map(|m| m.permissions().readonly()).unwrap_or(true) {
                eprintln!(
                    "blocked: target root {} is not writable by current user; run with sudo",
                    new_root
                );
                return ExitCode::FAILURE;
            }
        }
        match move_clone(old_root, new_root, slug) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("blocked: clone move failed: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Update registry JSON.
    if target == "served" || target == "dev" {
        if new_tier != current_tier {
            projects[idx]["serving_tier"] = Value::String(new_tier.clone());
            let new_root = tier_root(&new_tier).unwrap();
            if let Some(roots) = projects[idx].get_mut("roots") {
                roots["referential"] = Value::String(format!("{}/{}", new_root, slug));
                roots["state"] = Value::String(new_root);
            }
        }
    } else {
        if new_mutation != current_mutation {
            projects[idx]["mutation_policy"] = Value::String(new_mutation.clone());
        }
    }

    // Write registry back.
    let out_str = match serde_json::to_string_pretty(&value) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("blocked: serialize registry: {}", e);
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = fs::write(&registry_path, out_str) {
        eprintln!("blocked: write registry: {}", e);
        return ExitCode::FAILURE;
    }

    // Read-back verification.
    let verify_raw = fs::read_to_string(&registry_path).unwrap_or_default();
    let verify: Value = serde_json::from_str(&verify_raw).unwrap_or(Value::Null);
    let verified_tier = verify
        .get("projects")
        .and_then(|x| x.as_array())
        .and_then(|a| {
            a.iter()
                .find(|p| p.get("slug").and_then(|x| x.as_str()) == Some(slug))
        })
        .and_then(|p| p.get("serving_tier").and_then(|x| x.as_str()))
        .unwrap_or("")
        .to_string();
    let verified_root = verify
        .get("projects")
        .and_then(|x| x.as_array())
        .and_then(|a| {
            a.iter()
                .find(|p| p.get("slug").and_then(|x| x.as_str()) == Some(slug))
        })
        .and_then(|p| {
            p.get("roots")
                .and_then(|r| r.get("referential"))
                .and_then(|x| x.as_str())
        })
        .unwrap_or("")
        .to_string();

    // Clone exists check.
    let clone_ok = if target == "served" || target == "dev" {
        let nr = tier_root(target).unwrap();
        PathBuf::from(&nr).join(slug).is_dir()
    } else {
        true
    };

    let ok = clone_ok
        && (target != "served" && target != "dev" || verified_tier == target)
        && (target != "served" && target != "dev" || !verified_root.is_empty());

    if json {
        let out = serde_json::json!({
            "schema": "cynic.audit.external.promote.v1",
            "status": if ok { "ok" } else { "blocked" },
            "slug": slug,
            "target": target,
            "verified_tier": verified_tier,
            "verified_root": verified_root,
            "clone_present": clone_ok,
            "plan": plan,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        println!("promote {} -> {} : {}", slug, target, if ok { "OK" } else { "BLOCKED" });
        for p in &plan {
            println!("  - {}", p);
        }
        if clone_ok {
            println!("  clone present: yes");
        } else {
            println!("  clone present: NO");
        }
        println!("  verified tier: {}", verified_tier);
        println!("  verified root: {}", verified_root);
    }

    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
