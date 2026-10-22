//! Read-back and mutation surface for collaborator trust tier governance.
//!
//! WHY: OQ-4 policy (docs/design/COLLABORATOR_TRUST_PROMOTION.md) requires
//! enforcement of trust tier checks for external project collaborators.
//! This surface projects `infra/collaborators.json` and provides promotion/
//! demotion commands that wire into `cynic-node`'s pure trust promotion
//! usecase, which uses `cynic-vcs::external_source::admit_external_source_operation`.
//!
//! HOW: reads the declarative registries, delegates to the pure usecase,
//! emits JSON or text. Mutation requires explicit `--yes` flag.
//! NEGATIVE BOUNDARY: does not bypass trust tier admission, does not
//! perform network I/O, does not spawn processes.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use cynic_node::usecases::collaborator_watch_uc::diff_watch;
use cynic_vcs::{ExternalProvider, ExternalRepoRef, ExternalSourcePort};
use cynic_vcs_providers::{ForgejoExternalSourceAdapter, GitHubExternalSourceAdapter};

use crate::{emit_json, fail, rust_source};

#[derive(Subcommand)]
pub(crate) enum CollaboratorCommand {
    /// Read-only projection of the collaborator registry.
    Status {
        #[arg(long)]
        json: bool,
        /// Path to the collaborators registry (default: infra/collaborators.json).
        #[arg(long)]
        registry: Option<String>,
    },
    /// Promote a collaborator to a higher trust tier (requires --yes).
    Promote {
        /// Collaborator ID (e.g. "robinho100").
        collaborator_id: String,
        /// Target trust tier (1=Maintainer, 2=Contributor, 3=Observer).
        #[arg(long)]
        target_tier: u8,
        /// Explicit operator approval (human-in-the-loop).
        #[arg(long)]
        yes: bool,
        /// Evidence references (repeatable).
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        /// External source operation to check trust tier against.
        #[arg(long, default_value = "clone")]
        operation: String,
        /// Path to the collaborators registry.
        #[arg(long)]
        registry: Option<String>,
    },
    /// Demote a collaborator to a lower trust tier (requires --yes).
    Demote {
        /// Collaborator ID.
        collaborator_id: String,
        /// Target trust tier (1=Maintainer, 2=Contributor, 3=Observer).
        #[arg(long)]
        target_tier: u8,
        /// Explicit operator approval (human-in-the-loop).
        #[arg(long)]
        yes: bool,
        /// Evidence references (repeatable).
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        /// External source operation to check trust tier against.
        #[arg(long, default_value = "clone")]
        operation: String,
        /// Path to the collaborators registry.
        #[arg(long)]
        registry: Option<String>,
    },
    /// Onboard a new collaborator at observed trust (requires --yes).
    Onboard {
        /// Collaborator ID (e.g. "yansigit"). Must be unique in the registry.
        collaborator_id: String,
        /// GitHub handle.
        #[arg(long)]
        handle: String,
        /// Display name (defaults to the collaborator ID).
        #[arg(long)]
        display_name: Option<String>,
        /// Project slugs to attach (repeatable).
        #[arg(long = "project")]
        projects: Vec<String>,
        /// Note recorded on the entry.
        #[arg(long, default_value = "")]
        note: String,
        /// Explicit operator approval (human-in-the-loop).
        #[arg(long)]
        yes: bool,
        /// Path to the collaborators registry.
        #[arg(long)]
        registry: Option<String>,
    },
    /// Watch observed forge access against declared registries (read-only).
    Watch {
        /// Project slug; omit to watch every external project.
        #[arg(long)]
        project: Option<String>,
        /// Forgejo base URL (required for Forgejo-hosted projects).
        #[arg(long)]
        forgejo_url: Option<String>,
        /// GitHub token override.
        #[arg(long)]
        github_token: Option<String>,
        /// Forgejo token override.
        #[arg(long)]
        forgejo_token: Option<String>,
        /// Emit JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct CollaboratorsRegistry {
    schema: String,
    collaborators: Vec<CollaboratorEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CollaboratorEntry {
    collaborator_id: String,
    display_name: String,
    #[serde(default)]
    github_handle: String,
    #[serde(default)]
    trust_level: String,
    #[serde(default)]
    role: String,
    #[serde(default)]
    projects: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExternalProjectsRegistry {
    #[serde(default)]
    projects: Vec<ExternalProjectEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExternalProjectEntry {
    project_id: String,
    #[serde(default)]
    collaborator_ids: Vec<String>,
    #[serde(default)]
    mutation_policy: String,
    #[serde(default)]
    sensitivity: String,
}

#[derive(Debug, Serialize)]
struct CollaboratorStatusReport {
    schema: String,
    status: String,
    registry_path: String,
    collaborator_count: usize,
    collaborators: Vec<CollaboratorView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    residual_unknowns: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CollaboratorView {
    collaborator_id: String,
    display_name: String,
    trust_level: String,
    trust_tier: u8,
    role: String,
    projects: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PromotionReport {
    schema: String,
    collaborator_id: String,
    action: String,
    target_tier: u8,
    admitted: bool,
    blockers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_collaborator: Option<CollaboratorEntry>,
}

pub(crate) fn execute_collaborator(command: &CollaboratorCommand) -> ExitCode {
    match command {
        CollaboratorCommand::Status { json, registry } => {
            let root = rust_source::workspace_root();
            let default_path = root.join("infra/collaborators.json");
            let registry_path = registry
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or(default_path);
            let report = scan_status(&registry_path);
            if *json {
                emit_json(&report, report.status != "ok")
            } else {
                render_status_text(&report);
                if report.status != "ok" {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
        }
        CollaboratorCommand::Promote {
            collaborator_id,
            target_tier,
            yes,
            evidence_refs,
            operation,
            registry,
        } => execute_promotion(
            collaborator_id,
            *target_tier,
            *yes,
            evidence_refs,
            operation,
            registry.as_deref(),
            true,
        ),
        CollaboratorCommand::Demote {
            collaborator_id,
            target_tier,
            yes,
            evidence_refs,
            operation,
            registry,
        } => execute_promotion(
            collaborator_id,
            *target_tier,
            *yes,
            evidence_refs,
            operation,
            registry.as_deref(),
            false,
        ),
        CollaboratorCommand::Onboard {
            collaborator_id,
            handle,
            display_name,
            projects,
            note,
            yes,
            registry,
        } => execute_onboard(
            collaborator_id,
            handle,
            display_name.as_deref(),
            projects,
            note,
            *yes,
            registry.as_deref(),
        ),
        CollaboratorCommand::Watch {
            project,
            forgejo_url,
            github_token,
            forgejo_token,
            json,
        } => execute_watch(
            project.as_deref(),
            forgejo_url.as_deref(),
            github_token.as_deref(),
            forgejo_token.as_deref(),
            *json,
        ),
    }
}

fn scan_status(registry_path: &Path) -> CollaboratorStatusReport {
    let mut residual = Vec::new();
    let registry_path_str = registry_path.display().to_string();

    if !registry_path.exists() {
        residual.push(format!("registry not found: {registry_path_str}"));
        return CollaboratorStatusReport {
            schema: "cynic.cli.collaborator_status.v1".to_string(),
            status: "blocked".to_string(),
            registry_path: registry_path_str,
            collaborator_count: 0,
            collaborators: vec![],
            residual_unknowns: residual,
        };
    }

    let raw = match fs::read_to_string(registry_path) {
        Ok(s) => s,
        Err(e) => {
            residual.push(format!("cannot read registry {registry_path_str}: {e}"));
            return CollaboratorStatusReport {
                schema: "cynic.cli.collaborator_status.v1".to_string(),
                status: "blocked".to_string(),
                registry_path: registry_path_str,
                collaborator_count: 0,
                collaborators: vec![],
                residual_unknowns: residual,
            };
        }
    };

    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            residual.push(format!("invalid JSON in {registry_path_str}: {e}"));
            return CollaboratorStatusReport {
                schema: "cynic.cli.collaborator_status.v1".to_string(),
                status: "blocked".to_string(),
                registry_path: registry_path_str,
                collaborator_count: 0,
                collaborators: vec![],
                residual_unknowns: residual,
            };
        }
    };

    let collaborators_arr = value
        .get("collaborators")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();

    let mut collaborators = Vec::new();
    for c in &collaborators_arr {
        let trust_level = c
            .get("trust_level")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        collaborators.push(CollaboratorView {
            collaborator_id: c
                .get("collaborator_id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            display_name: c
                .get("display_name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            trust_level: trust_level.clone(),
            trust_tier: tier_from_level(&trust_level),
            role: c
                .get("role")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            projects: c
                .get("projects")
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        });
    }

    let status = if residual.is_empty() {
        "ok".to_string()
    } else {
        "blocked".to_string()
    };

    CollaboratorStatusReport {
        schema: "cynic.cli.collaborator_status.v1".to_string(),
        status,
        registry_path: registry_path_str,
        collaborator_count: collaborators.len(),
        collaborators,
        residual_unknowns: residual,
    }
}

fn render_status_text(report: &CollaboratorStatusReport) {
    println!(
        "Collaborators: {} (status: {})",
        report.collaborator_count, report.status
    );
    println!("Registry: {}", report.registry_path);
    for c in &report.collaborators {
        println!(
            "  - {} ({}) [tier {}] {}",
            c.collaborator_id, c.display_name, c.trust_tier, c.role
        );
        if !c.projects.is_empty() {
            println!("      projects: {}", c.projects.join(", "));
        }
    }
    if !report.residual_unknowns.is_empty() {
        println!("Residual unknowns:");
        for u in &report.residual_unknowns {
            println!("  - {u}");
        }
    }
}

fn execute_promotion(
    collaborator_id: &str,
    target_tier: u8,
    yes: bool,
    evidence_refs: &[String],
    operation: &str,
    registry: Option<&str>,
    is_promotion: bool,
) -> ExitCode {
    if !yes {
        eprintln!("Refusing to change trust tier without --yes flag");
        return ExitCode::FAILURE;
    }

    let root = rust_source::workspace_root();
    let default_collaborators_path = root.join("infra/collaborators.json");
    let default_projects_path = root.join("infra/external-projects.json");

    let collaborators_path = registry
        .map(PathBuf::from)
        .unwrap_or(default_collaborators_path);
    let projects_path = default_projects_path;

    // Read collaborators registry.
    let collaborators_raw = match fs::read_to_string(&collaborators_path) {
        Ok(s) => s,
        Err(e) => {
            return fail(format!(
                "cannot read collaborators registry {}: {e}",
                collaborators_path.display()
            ));
        }
    };
    let collaborators_value: serde_json::Value = match serde_json::from_str(&collaborators_raw) {
        Ok(v) => v,
        Err(e) => {
            return fail(format!(
                "invalid JSON in collaborators registry: {e}"
            ));
        }
    };

    // Read external projects registry.
    let projects_raw = match fs::read_to_string(&projects_path) {
        Ok(s) => s,
        Err(e) => {
            return fail(format!(
                "cannot read external projects registry {}: {e}",
                projects_path.display()
            ));
        }
    };
    let projects_value: serde_json::Value = match serde_json::from_str(&projects_raw) {
        Ok(v) => v,
        Err(e) => {
            return fail(format!(
                "invalid JSON in external projects registry: {e}"
            ));
        }
    };

    // Parse into the usecase types.
    let collaborators = parse_collaborators(&collaborators_value);
    let projects = parse_projects(&projects_value);

    let request = cynic_node::usecases::collaborator_trust_uc::TrustPromotionRequest {
        collaborator_id: collaborator_id.to_string(),
        target_tier,
        operator_approval: yes,
        evidence_refs: evidence_refs.to_vec(),
    };

    let report = cynic_node::usecases::collaborator_trust_uc::evaluate_trust_promotion(
        &collaborators,
        &projects,
        &request,
        operation,
    );

    if !report.admitted {
        let blockers = report.blockers.join("; ");
        return fail(format!(
            "trust tier change blocked for '{}': {blockers}",
            collaborator_id
        ));
    }

    // Write the updated collaborator back to the registry.
    if let Some(updated) = &report.updated_collaborator {
        let mut updated_value = collaborators_value.clone();
        if let Some(arr) = updated_value
            .get_mut("collaborators")
            .and_then(|x| x.as_array_mut())
        {
            for entry in arr.iter_mut() {
                if entry.get("collaborator_id").and_then(|x| x.as_str())
                    == Some(&updated.collaborator_id)
                {
                    if let Some(obj) = entry.as_object_mut() {
                        obj.insert(
                            "trust_level".to_string(),
                            serde_json::Value::String(updated.trust_level.clone()),
                        );
                    }
                }
            }
        }
        match serde_json::to_string_pretty(&updated_value) {
            Ok(json) => {
                if let Err(e) = fs::write(&collaborators_path, json) {
                    return fail(format!(
                        "cannot write collaborators registry {}: {e}",
                        collaborators_path.display()
                    ));
                }
            }
            Err(e) => {
                return fail(format!("serialization failed: {e}"));
            }
        }
    }

    // Emit JSON report.
    let promotion_report = PromotionReport {
        schema: "cynic.cli.collaborator_promotion.v1".to_string(),
        collaborator_id: collaborator_id.to_string(),
        action: if is_promotion {
            "promote".to_string()
        } else {
            "demote".to_string()
        },
        target_tier,
        admitted: true,
        blockers: vec![],
        updated_collaborator: report.updated_collaborator.clone().map(|c| CollaboratorEntry {
            collaborator_id: c.collaborator_id,
            display_name: c.display_name,
            github_handle: c.github_handle,
            trust_level: c.trust_level,
            role: c.role,
            projects: c.projects,
        }),
    };
    emit_json(&promotion_report, false)
}

#[derive(Debug, Serialize)]
struct OnboardReport {
    schema: String,
    collaborator_id: String,
    action: String,
    trust_level: String,
    admitted: bool,
    registry_path: String,
}

/// One-click onboarding: append a new collaborator at observed trust.
///
/// Refuses without --yes, with a blank id/handle, or when the id already
/// exists (use promote for trust changes). New entries always start at
/// `observed` per doctrine — trust from evidence, never reputation.
/// Pure admission for onboarding: validates the id/handle and builds the new
/// `observed` entry, or explains the refusal. No file I/O — the surface
/// reads the registry, calls this, then writes the returned entry.
fn admit_onboard(
    collaborators: &serde_json::Value,
    collaborator_id: &str,
    handle: &str,
    display_name: Option<&str>,
    projects: &[String],
    note: &str,
) -> Result<serde_json::Value, String> {
    let collaborator_id = collaborator_id.trim();
    let handle = handle.trim();
    if collaborator_id.is_empty() || collaborator_id.chars().any(char::is_whitespace) {
        return Err("collaborator_id must be a non-empty single token".to_string());
    }
    if handle.is_empty() {
        return Err("github handle must not be empty".to_string());
    }
    let arr = collaborators
        .get("collaborators")
        .and_then(|x| x.as_array())
        .ok_or_else(|| "collaborators registry has no collaborators array".to_string())?;
    if arr.iter().any(|c| {
        c.get("collaborator_id").and_then(|x| x.as_str()) == Some(collaborator_id)
    }) {
        return Err(format!(
            "collaborator '{collaborator_id}' is already onboarded; use promote for trust changes"
        ));
    }
    Ok(serde_json::json!({
        "collaborator_id": collaborator_id,
        "display_name": display_name.unwrap_or(collaborator_id),
        "github_handle": handle,
        "trust_level": "observed",
        "projects": projects,
        "notes": note,
    }))
}

#[allow(clippy::too_many_arguments)]
fn execute_onboard(
    collaborator_id: &str,
    handle: &str,
    display_name: Option<&str>,
    projects: &[String],
    note: &str,
    yes: bool,
    registry: Option<&str>,
) -> ExitCode {
    if !yes {
        eprintln!("Refusing to onboard without --yes flag");
        return ExitCode::FAILURE;
    }
    let collaborator_id = collaborator_id.trim();
    let handle = handle.trim();
    if collaborator_id.is_empty() || collaborator_id.chars().any(char::is_whitespace) {
        return fail("collaborator_id must be a non-empty single token".to_string());
    }
    if handle.is_empty() {
        return fail("github handle must not be empty".to_string());
    }

    let root = rust_source::workspace_root();
    let default_path = root.join("infra/collaborators.json");
    let registry_path = registry
        .map(PathBuf::from)
        .unwrap_or(default_path);

    let raw = match fs::read_to_string(&registry_path) {
        Ok(s) => s,
        Err(e) => {
            return fail(format!(
                "cannot read collaborators registry {}: {e}",
                registry_path.display()
            ));
        }
    };
    let mut value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            return fail(format!("invalid JSON in collaborators registry: {e}"));
        }
    };

    let entry = match admit_onboard(&value, collaborator_id, handle, display_name, projects, note) {
        Ok(entry) => entry,
        Err(reason) => return fail(reason),
    };
    match value
        .get_mut("collaborators")
        .and_then(|x| x.as_array_mut())
    {
        Some(arr) => arr.push(entry),
        None => {
            return fail("collaborators registry has no collaborators array".to_string());
        }
    };

    match serde_json::to_string_pretty(&value) {
        Ok(json) => {
            if let Err(e) = fs::write(&registry_path, json) {
                return fail(format!(
                    "cannot write collaborators registry {}: {e}",
                    registry_path.display()
                ));
            }
        }
        Err(e) => {
            return fail(format!("serialization failed: {e}"));
        }
    }

    let report = OnboardReport {
        schema: "cynic.cli.collaborator_onboard.v1".to_string(),
        collaborator_id: collaborator_id.to_string(),
        action: "onboard".to_string(),
        trust_level: "observed".to_string(),
        admitted: true,
        registry_path: registry_path.display().to_string(),
    };
    emit_json(&report, false)
}

#[derive(Debug, Serialize)]
struct WatchCliReport {
    schema: String,
    projects: Vec<cynic_node::usecases::collaborator_watch_uc::WatchReport>,
    drift_found: bool,
}

/// Read-only watch: compare forge access against declared registries.
///
/// Never mutates. Reads `infra/external-projects.json` and
/// `infra/collaborators.json`, lists collaborators on each forge, and returns
/// the pure diff with proposed signed gestures. Non-zero exit when drift is
/// found so a cron can alert.
#[allow(clippy::too_many_arguments)]
fn execute_watch(
    project: Option<&str>,
    forgejo_url: Option<&str>,
    github_token: Option<&str>,
    forgejo_token: Option<&str>,
    json: bool,
) -> ExitCode {
    let root = rust_source::workspace_root();
    let collaborators_raw = match fs::read_to_string(root.join("infra/collaborators.json")) {
        Ok(s) => s,
        Err(e) => return fail(format!("cannot read collaborators registry: {e}")),
    };
    let projects_raw = match fs::read_to_string(root.join("infra/external-projects.json")) {
        Ok(s) => s,
        Err(e) => return fail(format!("cannot read external projects registry: {e}")),
    };
    let collaborators_value: serde_json::Value = match serde_json::from_str(&collaborators_raw) {
        Ok(v) => v,
        Err(e) => return fail(format!("invalid JSON in collaborators registry: {e}")),
    };
    let projects_value: serde_json::Value = match serde_json::from_str(&projects_raw) {
        Ok(v) => v,
        Err(e) => return fail(format!("invalid JSON in external projects registry: {e}")),
    };

    let fiches = parse_collaborators(&collaborators_value);
    let fiche_ids: Vec<String> = fiches.iter().map(|c| c.collaborator_id.clone()).collect();
    let fiche_handles: Vec<String> = fiches.iter().map(|c| c.github_handle.clone()).collect();
    let projects = parse_projects(&projects_value);

    let mut reports = Vec::new();
    for p in &projects {
        if let Some(wanted) = project {
            if p.project_id != wanted {
                continue;
            }
        }
        let provider = if p.repo_url.contains("github.com") {
            ExternalProvider::GitHub
        } else if forgejo_url.is_some() {
            ExternalProvider::Forgejo
        } else {
            return fail(format!(
                "cannot determine provider for '{}' (use --forgejo-url for Forgejo projects)",
                p.project_id
            ));
        };
        let repo = ExternalRepoRef {
            project_id: p.project_id.clone(),
            remote_url: p.repo_url.clone(),
            default_branch: p.default_branch.clone(),
            provider,
        };
        let observed: Vec<String> = match repo.provider {
            ExternalProvider::GitHub => {
                let token = match github_token
                    .map(|s| s.to_string())
                    .or_else(|| cynic_secrets::env_var("github-token", "GITHUB_TOKEN"))
                {
                    Some(t) => t,
                    None => return fail("GITHUB_TOKEN is required for GitHub projects".to_string()),
                };
                match GitHubExternalSourceAdapter::new("https://api.github.com".to_string(), token)
                    .list_collaborators(&repo)
                {
                    Ok(h) => h,
                    Err(e) => return fail(format!("watch '{}' failed: {e}", p.project_id)),
                }
            }
            _ => {
                let base = match forgejo_url {
                    Some(url) => url.to_string(),
                    None => {
                        return fail("--forgejo-url is required for Forgejo projects".to_string())
                    }
                };
                let token = match forgejo_token
                    .map(|s| s.to_string())
                    .or_else(|| cynic_secrets::env_var("forgejo-token", "FORGEJO_TOKEN"))
                {
                    Some(t) => t,
                    None => return fail("FORGEJO_TOKEN is required for Forgejo projects".to_string()),
                };
                match ForgejoExternalSourceAdapter::new(base, token).list_collaborators(&repo) {
                    Ok(h) => h,
                    Err(e) => return fail(format!("watch '{}' failed: {e}", p.project_id)),
                }
            }
        };
        reports.push(diff_watch(
            &p.project_id,
            &observed,
            &p.collaborator_ids,
            &fiche_ids,
            &fiche_handles,
        ));
    }

    let drift_found = reports.iter().any(|r| !r.clean);
    if json {
        let report = WatchCliReport {
            schema: "cynic.cli.collaborator_watch.v1".to_string(),
            projects: reports,
            drift_found,
        };
        emit_json(&report, drift_found)
    } else {
        for r in &reports {
            if r.clean {
                println!("{}: clean", r.project_id);
            } else {
                println!("{}: {} drift item(s)", r.project_id, r.drift.len());
                for d in &r.drift {
                    println!("  [{}] {} -> {} ({})", d.class, d.subject, d.proposed_gesture, d.reason);
                }
            }
        }
        if drift_found {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}

fn parse_collaborators(value: &serde_json::Value) -> Vec<cynic_node::usecases::collaborator_trust_uc::Collaborator> {
    value
        .get("collaborators")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .map(|c| cynic_node::usecases::collaborator_trust_uc::Collaborator {
                    collaborator_id: c
                        .get("collaborator_id")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    display_name: c
                        .get("display_name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    github_handle: c
                        .get("github_handle")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    trust_level: c
                        .get("trust_level")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    role: c
                        .get("role")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    projects: c
                        .get("projects")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                    owns: c
                        .get("owns")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                    maintains: c
                        .get("maintains")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                    notes: c
                        .get("notes")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_projects(value: &serde_json::Value) -> Vec<cynic_node::usecases::collaborator_trust_uc::ExternalProject> {
    value
        .get("projects")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .map(|p| cynic_node::usecases::collaborator_trust_uc::ExternalProject {
                    project_id: p
                        .get("project_id")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    display_name: p
                        .get("display_name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    repo_url: p
                        .get("repo_url")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    default_branch: p
                        .get("default_branch")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    collaborator_ids: p
                        .get("collaborator_ids")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                    mutation_policy: p
                        .get("mutation_policy")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    sensitivity: p
                        .get("sensitivity")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn tier_from_level(level: &str) -> u8 {
    match level {
        "core" => 1,
        "trusted" => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod onboard_tests {
    use super::*;

    fn empty_registry() -> serde_json::Value {
        serde_json::json!({ "collaborators": [] })
    }

    fn registry_with(id: &str) -> serde_json::Value {
        serde_json::json!({ "collaborators": [{ "collaborator_id": id }] })
    }

    #[test]
    fn newcomer_is_admitted_at_observed() {
        let entry = admit_onboard(&empty_registry(), "yansigit", "yansigit", None, &[], "")
            .expect("newcomer must be admitted");
        assert_eq!(entry["collaborator_id"], "yansigit");
        assert_eq!(entry["trust_level"], "observed");
        assert_eq!(entry["github_handle"], "yansigit");
    }

    #[test]
    fn duplicate_id_is_refused() {
        let err = admit_onboard(&registry_with("yansigit"), "yansigit", "yansigit", None, &[], "")
            .expect_err("duplicate must be refused");
        assert!(err.contains("already onboarded"), "unexpected: {err}");
    }

    #[test]
    fn blank_id_is_refused() {
        assert!(admit_onboard(&empty_registry(), "   ", "yansigit", None, &[], "").is_err());
    }

    #[test]
    fn spaced_id_is_refused() {
        assert!(admit_onboard(&empty_registry(), "yan sigit", "yansigit", None, &[], "").is_err());
    }

    #[test]
    fn blank_handle_is_refused() {
        assert!(admit_onboard(&empty_registry(), "yansigit", "", None, &[], "").is_err());
    }
}
