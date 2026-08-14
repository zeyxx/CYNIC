use cynic_host::domain::shared::BoundaryKind;

use crate::runtime_surface::report::types::{InferenceReportSummary, SovereignGitSummary};

pub(crate) fn inference_summary() -> InferenceReportSummary {
    match cynic_inference::load_inference_config() {
        Ok(loaded) => {
            let sovereign_count = loaded
                .backends
                .iter()
                .filter(|backend| backend.sovereign)
                .count();
            InferenceReportSummary {
                schema: "cynic.inference_config.v1",
                status: "ok",
                config_path: loaded.path.map(|path| path.to_string_lossy().to_string()),
                backend_count: loaded.backends.len(),
                sovereign_count,
                error: None,
            }
        }
        Err(cynic_inference::InferenceConfigError::NotFound { path }) => InferenceReportSummary {
            schema: "cynic.inference_config.v1",
            status: "not_found",
            config_path: Some(path.to_string_lossy().to_string()),
            backend_count: 0,
            sovereign_count: 0,
            error: None,
        },
        Err(error) => InferenceReportSummary {
            schema: "cynic.inference_config.v1",
            status: "error",
            config_path: None,
            backend_count: 0,
            sovereign_count: 0,
            error: Some(error.to_string()),
        },
    }
}

pub(crate) fn sovereign_git_summary() -> SovereignGitSummary {
    let report = cynic_host::repo_access::current_repo_access_status();
    let forgejo = report
        .sovereign_remotes
        .iter()
        .find(|remote| remote.id == "forgejo");
    match forgejo {
        Some(forgejo) => {
            let blocked = forgejo.service_health_status != "reachable";
            SovereignGitSummary {
                schema: "cynic.runtime.sovereign_git.v1",
                status: if blocked { "blocked" } else { "ok" },
                forgejo_remote_present: true,
                forgejo_service_health: Some(forgejo.service_health_status.clone()),
                forgejo_repo_auth_readiness: Some(forgejo.repo_auth_readiness.clone()),
                forgejo_host_resolution_status: Some(forgejo.host_resolution_status.clone()),
                forgejo_tcp_reachability_status: Some(forgejo.tcp_reachability_status.clone()),
                forgejo_url: Some(forgejo.url.clone()),
                next_boundary_kind: if blocked {
                    BoundaryKind::RepairService
                } else {
                    BoundaryKind::Observe
                },
                next_boundary_reason: if blocked {
                    "forgejo sovereign remote is declared but service reachability is not yet proven"
                        .to_string()
                } else {
                    "forgejo sovereign remote is declared and service reachability is proven"
                        .to_string()
                },
                next_command: if blocked {
                    Some("cynic runtime repo-access --json".to_string())
                } else {
                    None
                },
            }
        }
        None => SovereignGitSummary {
            schema: "cynic.runtime.sovereign_git.v1",
            status: "blocked",
            forgejo_remote_present: false,
            forgejo_service_health: None,
            forgejo_repo_auth_readiness: None,
            forgejo_host_resolution_status: None,
            forgejo_tcp_reachability_status: None,
            forgejo_url: None,
            next_boundary_kind: BoundaryKind::Adopt,
            next_boundary_reason:
                "forgejo sovereign remote authority is absent from runtime repo-access projection"
                    .to_string(),
            next_command: Some("cynic runtime repo-access --json".to_string()),
        },
    }
}
