use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::domain::shared::{BoundaryKind, LifecycleDisposition, LifecycleStatus};
use crate::process_observation::command_json;
use crate::repo_access::{current_project_root, current_repo_access_status};
use crate::runtime_harness::{RuntimeHarnessContext, conformance_context};
use crate::runtime_layout::HostRuntimeLayout;
use crate::runtime_network_preflight::{
    RuntimeNetworkPreflightReport, observe_runtime_network_preflight,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeExecutionContextReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub context: RuntimeHarnessContext,
    pub lifecycle_status: LifecycleStatus,
    pub lifecycle_disposition: LifecycleDisposition,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    pub next_command: Option<String>,
    pub execution_boundary: ExecutionBoundary,
    pub process: ProcessIdentity,
    pub paths: RuntimePathEvidence,
    pub environment: EnvironmentEvidence,
    pub repo_access: RepoAccessEvidence,
    pub network_preflight: RuntimeNetworkPreflightReport,
    pub classification: Vec<&'static str>,
    pub rules: Vec<&'static str>,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExecutionBoundary {
    pub kind: &'static str,
    pub confidence: &'static str,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub current_exe: Option<String>,
    pub current_dir: Option<String>,
    pub user: Option<String>,
    pub home: Option<String>,
    pub uid: CommandSummary,
    pub euid: CommandSummary,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommandSummary {
    pub command: String,
    pub args: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimePathEvidence {
    pub installed_cli: PathStatus,
    pub config_root: PathStatus,
    pub state_root: PathStatus,
    pub log_root: PathStatus,
    pub runtime_home: PathStatus,
    pub host_exec_helper: PathStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PathStatus {
    pub path: String,
    pub exists: bool,
    pub is_dir: bool,
    pub is_file: bool,
    pub readonly_metadata: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EnvironmentEvidence {
    pub cynic_env: BTreeMap<String, String>,
    pub sandbox_env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RepoAccessEvidence {
    pub project_root: Option<String>,
    pub profile_path: String,
    pub profile_exists: bool,
    pub known_repo_count: usize,
}

pub fn observe_runtime_execution_context(
    network_targets: &[String],
) -> RuntimeExecutionContextReport {
    let process = observe_process_identity();
    let layout = HostRuntimeLayout::from_env();
    let paths = observe_runtime_paths(&layout);
    let environment = observe_environment();
    let repo_access_status = current_repo_access_status();
    let repo_access = RepoAccessEvidence {
        project_root: current_project_root().map(path_to_string),
        profile_path: repo_access_status.profile_path,
        profile_exists: repo_access_status.profile_exists,
        known_repo_count: repo_access_status.known_repos.len(),
    };
    let network_preflight = observe_runtime_network_preflight(network_targets);
    let execution_boundary =
        classify_execution_boundary(&process, &paths, &environment, &repo_access);
    let classification = classify_blockers(&execution_boundary, &environment, &network_preflight);
    let status = if classification.iter().any(|item| {
        matches!(
            *item,
            "blocked_by_network_or_sandbox"
                | "blocked_by_missing_installed_runtime"
                | "blocked_by_worktree_context"
                | "blocked_by_unknown_execution_context"
                | "blocked_by_sandbox_network_policy"
        )
    }) {
        "blocked"
    } else {
        "ready"
    };
    let (
        lifecycle_status,
        lifecycle_disposition,
        next_boundary_kind,
        next_boundary_reason,
        next_command,
    ) = projection(&execution_boundary, &network_preflight, status);

    RuntimeExecutionContextReport {
        schema: "cynic.host.runtime_execution_context.v1",
        status,
        mutates: false,
        context: conformance_context(),
        lifecycle_status,
        lifecycle_disposition,
        next_boundary_kind,
        next_boundary_reason,
        next_command: next_command.clone(),
        execution_boundary,
        process,
        paths,
        environment,
        repo_access,
        network_preflight,
        classification,
        rules: vec![
            "execution context is read-only and must not mutate the host",
            "sandbox evidence must not be promoted as installed-runtime authority",
            "worktree context can build and audit but cannot prove host runtime authority alone",
            "network failures must be classified against the observed execution boundary before remediation",
        ],
        next_actions: next_actions(status, next_command.as_deref()),
    }
}

fn observe_process_identity() -> ProcessIdentity {
    ProcessIdentity {
        current_exe: std::env::current_exe().ok().map(path_to_string),
        current_dir: std::env::current_dir().ok().map(path_to_string),
        user: std::env::var("USER").ok(),
        home: std::env::var("HOME").ok(),
        uid: command_summary(command_json("id", &["-u"])),
        euid: command_summary(command_json("id", &["-ru"])),
    }
}

fn observe_runtime_paths(layout: &HostRuntimeLayout) -> RuntimePathEvidence {
    RuntimePathEvidence {
        installed_cli: path_status("/usr/local/bin/cynic"),
        config_root: path_status(&layout.config_dir),
        state_root: path_status(&layout.state_dir),
        log_root: path_status(&layout.log_dir),
        runtime_home: path_status(&layout.runtime_home_dir),
        host_exec_helper: path_status(layout.host_exec_helper_path()),
    }
}

fn observe_environment() -> EnvironmentEvidence {
    let mut cynic_env = BTreeMap::new();
    let mut sandbox_env = BTreeMap::new();
    for (key, value) in std::env::vars() {
        if key.starts_with("CYNIC_") {
            cynic_env.insert(key, value);
        } else if key.contains("SANDBOX") || key.starts_with("CODEX") || key.starts_with("OPENAI") {
            sandbox_env.insert(key, value);
        }
    }

    EnvironmentEvidence {
        cynic_env,
        sandbox_env,
    }
}

fn classify_execution_boundary(
    process: &ProcessIdentity,
    paths: &RuntimePathEvidence,
    environment: &EnvironmentEvidence,
    repo_access: &RepoAccessEvidence,
) -> ExecutionBoundary {
    let exe = process.current_exe.as_deref().unwrap_or_default();
    if !environment.sandbox_env.is_empty() {
        return ExecutionBoundary {
            kind: "sandbox_or_orchestrated",
            confidence: "high",
            reason: "sandbox-related environment variables are present",
        };
    }
    if exe == "/usr/local/bin/cynic" && paths.config_root.exists && paths.state_root.exists {
        return ExecutionBoundary {
            kind: "installed_runtime",
            confidence: "high",
            reason: "current executable and runtime paths match installed authority",
        };
    }
    if repo_access.project_root.is_some() {
        return ExecutionBoundary {
            kind: "dev_worktree",
            confidence: "high",
            reason: "a project root is discoverable from cwd or executable",
        };
    }
    if paths.installed_cli.exists {
        return ExecutionBoundary {
            kind: "installed_cli_without_runtime_context",
            confidence: "medium",
            reason: "installed CLI exists but runtime authority paths are incomplete or current executable differs",
        };
    }
    ExecutionBoundary {
        kind: "unknown",
        confidence: "low",
        reason: "no sandbox marker, installed runtime, or project root was observed",
    }
}

fn classify_blockers(
    execution_boundary: &ExecutionBoundary,
    environment: &EnvironmentEvidence,
    network_preflight: &RuntimeNetworkPreflightReport,
) -> Vec<&'static str> {
    let mut classification = Vec::new();
    match execution_boundary.kind {
        "dev_worktree" => classification.push("blocked_by_worktree_context"),
        "unknown" => classification.push("blocked_by_unknown_execution_context"),
        "installed_cli_without_runtime_context" => {
            classification.push("blocked_by_missing_installed_runtime")
        }
        _ => classification.push("execution_boundary_observed"),
    }
    if network_preflight.status != "ready"
        && environment
            .sandbox_env
            .contains_key("CODEX_SANDBOX_NETWORK_DISABLED")
    {
        classification.push("blocked_by_sandbox_network_policy");
    } else if network_preflight.status != "ready" {
        classification.push("blocked_by_network_or_sandbox");
    }
    classification
}

fn projection(
    execution_boundary: &ExecutionBoundary,
    network_preflight: &RuntimeNetworkPreflightReport,
    status: &str,
) -> (
    LifecycleStatus,
    LifecycleDisposition,
    BoundaryKind,
    String,
    Option<String>,
) {
    if status == "ready" {
        return (
            LifecycleStatus::Ready,
            LifecycleDisposition::ObservableCandidate,
            BoundaryKind::Observe,
            "execution context is suitable for current fetch and runtime evidence".to_string(),
            None,
        );
    }

    if execution_boundary.kind == "dev_worktree" || execution_boundary.kind == "unknown" {
        return (
            LifecycleStatus::Missing,
            LifecycleDisposition::NotDiscovered,
            BoundaryKind::Adopt,
            "execution context is not an installed runtime authority boundary".to_string(),
            Some(
                "compare this report with the same command run from the installed /usr/local/bin/cynic"
                    .to_string(),
            ),
        );
    }

    if network_preflight.status != "ready" {
        return (
            LifecycleStatus::ConnectionRequired,
            LifecycleDisposition::RequiresConnection,
            BoundaryKind::Connect,
            "execution context is present but network preflight is not yet ready".to_string(),
            Some("cynic runtime resolver-plan --json".to_string()),
        );
    }

    (
        LifecycleStatus::PrincipalMismatch,
        LifecycleDisposition::RequiresPrincipalAdoption,
        BoundaryKind::Adopt,
        "installed CLI or runtime authority paths are incomplete for this execution context"
            .to_string(),
        Some(
            "compare this report with the same command run from the installed /usr/local/bin/cynic"
                .to_string(),
        ),
    )
}

fn next_actions(status: &str, canonical_next_command: Option<&str>) -> Vec<String> {
    if status == "ready" {
        return vec!["use this execution context as current fetch/runtime evidence".to_string()];
    }
    let mut actions = vec![
        "compare this report with the same command run from the installed /usr/local/bin/cynic"
            .to_string(),
        "do not mutate host networking from a dev worktree or sandbox report".to_string(),
        "if installed runtime also reports blocked network, proceed to runtime resolver-plan before any apply surface".to_string(),
    ];
    if let Some(command) = canonical_next_command {
        actions.push(command.to_string());
    }
    actions.sort();
    actions.dedup();
    actions
}

fn command_summary(value: crate::process_observation::CommandObservation) -> CommandSummary {
    CommandSummary {
        command: value.command,
        args: value.args,
        exit_code: value.exit_code,
        stdout: value.stdout,
        stderr: value.stderr,
    }
}

fn path_status(path: impl AsRef<Path>) -> PathStatus {
    let path = path.as_ref();
    match std::fs::metadata(path) {
        Ok(metadata) => PathStatus {
            path: path_to_string(path),
            exists: true,
            is_dir: metadata.is_dir(),
            is_file: metadata.is_file(),
            readonly_metadata: Some(metadata.permissions().readonly()),
        },
        Err(_) => PathStatus {
            path: path_to_string(path),
            exists: false,
            is_dir: false,
            is_file: false,
            readonly_metadata: None,
        },
    }
}

fn path_to_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(exe: &str) -> ProcessIdentity {
        ProcessIdentity {
            current_exe: Some(exe.to_string()),
            current_dir: Some("/tmp".to_string()),
            user: Some("user".to_string()),
            home: Some("/tmp/home".to_string()),
            uid: CommandSummary {
                command: "id".to_string(),
                args: vec!["-u".to_string()],
                exit_code: Some(0),
                stdout: "1000".to_string(),
                stderr: String::new(),
            },
            euid: CommandSummary {
                command: "id".to_string(),
                args: vec!["-ru".to_string()],
                exit_code: Some(0),
                stdout: "1000".to_string(),
                stderr: String::new(),
            },
        }
    }

    fn paths(installed: bool) -> RuntimePathEvidence {
        RuntimePathEvidence {
            installed_cli: PathStatus {
                path: "/usr/local/bin/cynic".to_string(),
                exists: installed,
                is_dir: false,
                is_file: installed,
                readonly_metadata: Some(false),
            },
            config_root: PathStatus {
                path: "/etc/cynic".to_string(),
                exists: installed,
                is_dir: installed,
                is_file: false,
                readonly_metadata: Some(false),
            },
            state_root: PathStatus {
                path: "/var/lib/cynic".to_string(),
                exists: installed,
                is_dir: installed,
                is_file: false,
                readonly_metadata: Some(false),
            },
            log_root: PathStatus {
                path: "/var/log/cynic".to_string(),
                exists: installed,
                is_dir: installed,
                is_file: false,
                readonly_metadata: Some(false),
            },
            runtime_home: PathStatus {
                path: "/var/lib/cynic".to_string(),
                exists: installed,
                is_dir: installed,
                is_file: false,
                readonly_metadata: Some(false),
            },
            host_exec_helper: PathStatus {
                path: "/usr/local/libexec/cynic/cynic-host-exec".to_string(),
                exists: false,
                is_dir: false,
                is_file: false,
                readonly_metadata: None,
            },
        }
    }

    #[test]
    fn classifies_installed_runtime_when_exe_and_paths_match() {
        let boundary = classify_execution_boundary(
            &process("/usr/local/bin/cynic"),
            &paths(true),
            &EnvironmentEvidence {
                cynic_env: BTreeMap::new(),
                sandbox_env: BTreeMap::new(),
            },
            &RepoAccessEvidence {
                project_root: None,
                profile_path: "/etc/cynic/repo-access-profile.json".to_string(),
                profile_exists: false,
                known_repo_count: 0,
            },
        );

        assert_eq!(boundary.kind, "installed_runtime");
        assert_eq!(boundary.confidence, "high");
    }

    #[test]
    fn classifies_dev_worktree_from_project_root_without_installed_runtime() {
        let boundary = classify_execution_boundary(
            &process("/tmp/cynic/target/debug/cynic"),
            &paths(false),
            &EnvironmentEvidence {
                cynic_env: BTreeMap::new(),
                sandbox_env: BTreeMap::new(),
            },
            &RepoAccessEvidence {
                project_root: Some("/tmp/cynic".to_string()),
                profile_path: "infra/repo-access-profile.json".to_string(),
                profile_exists: false,
                known_repo_count: 1,
            },
        );

        assert_eq!(boundary.kind, "dev_worktree");
    }

    #[test]
    fn sandbox_environment_overrides_other_boundary_signals() {
        let mut sandbox_env = BTreeMap::new();
        sandbox_env.insert("CODEX_SANDBOX".to_string(), "1".to_string());

        let boundary = classify_execution_boundary(
            &process("/usr/local/bin/cynic"),
            &paths(true),
            &EnvironmentEvidence {
                cynic_env: BTreeMap::new(),
                sandbox_env,
            },
            &RepoAccessEvidence {
                project_root: None,
                profile_path: "/etc/cynic/repo-access-profile.json".to_string(),
                profile_exists: false,
                known_repo_count: 0,
            },
        );

        assert_eq!(boundary.kind, "sandbox_or_orchestrated");
    }

    #[test]
    fn sandbox_network_policy_gets_specific_blocker_classification() {
        let mut sandbox_env = BTreeMap::new();
        sandbox_env.insert(
            "CODEX_SANDBOX_NETWORK_DISABLED".to_string(),
            "1".to_string(),
        );
        let environment = EnvironmentEvidence {
            cynic_env: BTreeMap::new(),
            sandbox_env,
        };
        let mut network_preflight =
            crate::runtime_network_preflight::observe_runtime_network_preflight(&[
                "example.invalid".to_string(),
            ]);
        network_preflight.status = "blocked";

        let classification = classify_blockers(
            &ExecutionBoundary {
                kind: "sandbox_or_orchestrated",
                confidence: "high",
                reason: "test",
            },
            &environment,
            &network_preflight,
        );

        assert!(classification.contains(&"blocked_by_sandbox_network_policy"));
        assert!(!classification.contains(&"blocked_by_network_or_sandbox"));
    }
}
