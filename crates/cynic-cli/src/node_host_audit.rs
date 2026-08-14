use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use cynic_host::process_observation::capture_command_in_dir;
use serde::Serialize;

const NODE_ROOT: &str = "crates/cynic-node";
const HOST_ROOT: &str = "crates/cynic-host";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NodeHostAudit {
    schema: &'static str,
    scope: Vec<&'static str>,
    git: GitAudit,
    summary: AuditSummary,
    modules: Vec<ModuleAudit>,
    boundary_violations: Vec<BoundaryViolation>,
    inferred_errors: Vec<InferredError>,
}

#[derive(Debug, Clone, Serialize)]
struct GitAudit {
    worktree_summary: GitCommandSummary,
    diff_stat: GitCommandSummary,
    recent_history: Vec<GitHistoryEntry>,
    note: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct GitCommandSummary {
    command: String,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone, Serialize)]
struct GitHistoryEntry {
    commit: String,
    subject: String,
}

#[derive(Debug, Clone, Serialize)]
struct AuditSummary {
    file_count: usize,
    by_owner: BTreeMap<String, usize>,
    by_layer: BTreeMap<String, usize>,
    by_action: BTreeMap<String, usize>,
    boundary_violation_count: usize,
    untracked_cynic_host: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ModuleAudit {
    path: String,
    owner_expected: &'static str,
    owner_actual: &'static str,
    layer: &'static str,
    git_status: String,
    line_count: usize,
    action: &'static str,
    risk: &'static str,
    signals: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct BoundaryViolation {
    path: String,
    kind: &'static str,
    current_owner: &'static str,
    target_owner: &'static str,
    reason: &'static str,
    action: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct InferredError {
    kind: &'static str,
    evidence: Vec<String>,
    consequence: &'static str,
    correction: &'static str,
}

pub(crate) fn build_node_host_audit(max_commits: usize) -> Result<NodeHostAudit, String> {
    let root = std::env::current_dir().map_err(|error| error.to_string())?;
    let mut modules = Vec::new();
    for path in discover_source_files(&root, NODE_ROOT)? {
        modules.push(classify_module(&root, &path)?);
    }
    for path in discover_source_files(&root, HOST_ROOT)? {
        modules.push(classify_module(&root, &path)?);
    }
    modules.sort_by(|left, right| left.path.cmp(&right.path));

    let boundary_violations = boundary_violations(&modules);
    let summary = summary(&modules, boundary_violations.len());
    let git = git_audit(&root, max_commits);
    let inferred_errors = infer_errors(&modules, &boundary_violations, &git);

    Ok(NodeHostAudit {
        schema: "cynic.node_host_audit.v1",
        scope: vec![NODE_ROOT, HOST_ROOT],
        git,
        summary,
        modules,
        boundary_violations,
        inferred_errors,
    })
}

fn discover_source_files(root: &Path, relative_root: &str) -> Result<Vec<PathBuf>, String> {
    let start = root.join(relative_root);
    if !start.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_files(&start, &mut files)?;
    files.retain(|path| is_source_path(root, path));
    Ok(files)
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read dir entry: {error}"))?;
        let entry_path = entry.path();
        let file_name = entry_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if file_name == "target" || file_name.starts_with('.') {
            continue;
        }
        if entry_path.is_dir() {
            collect_files(&entry_path, files)?;
        } else {
            files.push(entry_path);
        }
    }
    Ok(())
}

fn is_source_path(root: &Path, path: &Path) -> bool {
    let relative = relative_path(root, path);
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext, "rs" | "toml"))
        && !relative.contains("/target/")
}

fn classify_module(root: &Path, path: &Path) -> Result<ModuleAudit, String> {
    let relative = relative_path(root, path);
    let content = fs::read_to_string(path).unwrap_or_default();
    let signals = signals(&relative, &content);
    let layer = layer_for(&relative, &signals);
    let owner_actual = owner_for_path(&relative);
    let owner_expected = expected_owner_for(layer, &relative, &signals);
    let action = action_for(owner_actual, owner_expected, layer, &signals);
    let risk = risk_for(action, &signals);

    Ok(ModuleAudit {
        path: relative.clone(),
        owner_expected,
        owner_actual,
        layer,
        git_status: git_status_for(root, &relative),
        line_count: content.lines().count(),
        action,
        risk,
        signals,
    })
}

fn signals(relative: &str, content: &str) -> Vec<&'static str> {
    let mut signals = BTreeSet::new();
    let lower = relative.to_ascii_lowercase();
    let haystack = format!("{lower}\n{}", content.to_ascii_lowercase());

    add_if(
        &mut signals,
        "host_substrate",
        contains_any(
            &haystack,
            &[
                "sudoers",
                "systemctl",
                "/etc/systemd",
                "host_substrate",
                "host_bootstrap",
                "process_execution",
                "runtime_layout",
                "cynic_host::",
            ],
        ),
    );
    add_if(
        &mut signals,
        "raw_process",
        contains_any(
            &haystack,
            &["std::process::command", "tokio::process::command"],
        ),
    );
    add_if(
        &mut signals,
        "network",
        contains_any(
            &haystack,
            &[
                "wireguard",
                "wg",
                "dns",
                "coredns",
                "resolved",
                "route",
                "mesh",
                "synapse",
            ],
        ),
    );
    add_if(
        &mut signals,
        "provider_fallback",
        contains_any(
            &haystack,
            &[
                "provider_fallback",
                "headscale",
                "tailscale",
                "proxmox",
                "lxc",
            ],
        ),
    );
    add_if(
        &mut signals,
        "artifact",
        contains_any(
            &haystack,
            &[
                "binary_attestation",
                "execstart",
                "artifact",
                "hash",
                "publish_path",
            ],
        ),
    );
    add_if(
        &mut signals,
        "watchdog",
        contains_any(&haystack, &["watchdog", "heartbeat", "sampler"]),
    );
    add_if(
        &mut signals,
        "kernel_bus",
        contains_any(
            &haystack,
            &["/observe", "kernel", "websocket", "mempool", "coord"],
        ),
    );
    add_if(
        &mut signals,
        "registry",
        contains_any(
            &haystack,
            &[
                "infra/registry.json",
                "registry",
                "declared_nodes",
                "runtime_artifacts",
            ],
        ),
    );
    add_if(
        &mut signals,
        "cli_surface",
        contains_any(&haystack, &["clap", "subcommand", "parser"]),
    );
    add_if(&mut signals, "large_file", content.lines().count() > 500);

    signals.into_iter().collect()
}

fn add_if(values: &mut BTreeSet<&'static str>, value: &'static str, condition: bool) {
    if condition {
        values.insert(value);
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn layer_for(relative: &str, signals: &[&'static str]) -> &'static str {
    if has(signals, "host_substrate") || relative.starts_with(HOST_ROOT) {
        "L0_host_substrate"
    } else if has(signals, "artifact") {
        "L1_artifact_process"
    } else if relative.contains("system_probe") || relative.contains("runtime_observation") {
        "L2_service_observation"
    } else if has(signals, "network") {
        "L3_network_dns_mesh"
    } else if has(signals, "kernel_bus") {
        "L4_kernel_bus"
    } else if has(signals, "watchdog") {
        "L5_node_runtime"
    } else {
        "supporting_runtime_code"
    }
}

fn owner_for_path(relative: &str) -> &'static str {
    if relative.starts_with(HOST_ROOT) {
        "cynic-host"
    } else if relative.starts_with(NODE_ROOT) {
        "cynic-node"
    } else {
        "unknown"
    }
}

fn expected_owner_for(
    layer: &'static str,
    relative: &str,
    signals: &[&'static str],
) -> &'static str {
    if layer == "L0_host_substrate" {
        "cynic-host"
    } else if has(signals, "provider_fallback") && relative.contains("proxmox") {
        "cynic-fleet_or_transport_adapter"
    } else if has(signals, "cli_surface") && relative.ends_with("cli.rs") {
        "cynic-cli_or_thin_node_cli"
    } else {
        "cynic-node"
    }
}

fn action_for(
    owner_actual: &'static str,
    owner_expected: &'static str,
    layer: &'static str,
    signals: &[&'static str],
) -> &'static str {
    if owner_actual != owner_expected && owner_expected == "cynic-host" {
        "move_host_substrate_to_cynic_host"
    } else if owner_expected == "cynic-fleet_or_transport_adapter" {
        "quarantine_provider_transport_truth"
    } else if has(signals, "large_file") {
        "split_large_runtime_surface"
    } else if layer == "L0_host_substrate" && owner_actual == "cynic-host" {
        "keep_as_host_substrate"
    } else {
        "keep_and_harden"
    }
}

fn risk_for(action: &str, signals: &[&'static str]) -> &'static str {
    if action == "move_host_substrate_to_cynic_host" {
        "node_owns_os_substrate"
    } else if action == "quarantine_provider_transport_truth" {
        "provider_adapter_can_become_canonical_truth"
    } else if action == "split_large_runtime_surface" {
        "large_surface_hides_mixed_responsibilities"
    } else if has(signals, "raw_process") {
        "raw_process_execution_boundary"
    } else {
        "low"
    }
}

fn boundary_violations(modules: &[ModuleAudit]) -> Vec<BoundaryViolation> {
    modules
        .iter()
        .filter_map(|module| {
            if module.action == "move_host_substrate_to_cynic_host" {
                Some(BoundaryViolation {
                    path: module.path.clone(),
                    kind: "host_substrate_inside_node",
                    current_owner: module.owner_actual,
                    target_owner: "cynic-host",
                    reason: "OS/process/systemd/sudoers primitives must be owned by host substrate, not node runtime.",
                    action: "move primitive or keep only a thin typed adapter consuming cynic-host",
                })
            } else if module.action == "quarantine_provider_transport_truth" {
                Some(BoundaryViolation {
                    path: module.path.clone(),
                    kind: "provider_transport_inside_node_truth",
                    current_owner: module.owner_actual,
                    target_owner: "cynic-fleet_or_transport_adapter",
                    reason: "Provider and hypervisor transport may observe/reach machines but must not define canonical node truth.",
                    action: "keep as explicit adapter or move to fleet boundary after runtime proof",
                })
            } else {
                None
            }
        })
        .collect()
}

fn summary(modules: &[ModuleAudit], boundary_violation_count: usize) -> AuditSummary {
    let mut by_owner = BTreeMap::new();
    let mut by_layer = BTreeMap::new();
    let mut by_action = BTreeMap::new();
    let mut untracked_cynic_host = false;

    for module in modules {
        increment(&mut by_owner, module.owner_actual);
        increment(&mut by_layer, module.layer);
        increment(&mut by_action, module.action);
        if module.path.starts_with(HOST_ROOT) && module.git_status.starts_with("??") {
            untracked_cynic_host = true;
        }
    }

    AuditSummary {
        file_count: modules.len(),
        by_owner,
        by_layer,
        by_action,
        boundary_violation_count,
        untracked_cynic_host,
    }
}

fn increment(values: &mut BTreeMap<String, usize>, key: &str) {
    *values.entry(key.to_string()).or_insert(0) += 1;
}

fn git_audit(root: &Path, max_commits: usize) -> GitAudit {
    let worktree_summary = git_command(root, &["status", "--short", "--", NODE_ROOT, HOST_ROOT]);
    let diff_stat = git_command(root, &["diff", "--stat", "--", NODE_ROOT, HOST_ROOT]);
    let max_count = format!("--max-count={max_commits}");
    let history = git_command(
        root,
        &[
            "log",
            "--oneline",
            "--decorate",
            max_count.as_str(),
            "--",
            NODE_ROOT,
            HOST_ROOT,
        ],
    );
    GitAudit {
        recent_history: parse_history(&history.stdout),
        worktree_summary,
        diff_stat,
        note: "cynic-host is untracked in this worktree if paths appear with ??; Git cannot provide file-level history until it is committed.",
    }
}

fn git_command(root: &Path, args: &[&str]) -> GitCommandSummary {
    match capture_command_in_dir("git", args, Some(root)) {
        Ok(output) => GitCommandSummary {
            command: format!("git {}", args.join(" ")),
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
        },
        Err(error) => GitCommandSummary {
            command: format!("git {}", args.join(" ")),
            exit_code: None,
            stdout: String::new(),
            stderr: error,
        },
    }
}

fn parse_history(stdout: &str) -> Vec<GitHistoryEntry> {
    stdout
        .lines()
        .filter_map(|line| {
            let (commit, subject) = line.split_once(' ')?;
            Some(GitHistoryEntry {
                commit: commit.to_string(),
                subject: subject.to_string(),
            })
        })
        .collect()
}

fn infer_errors(
    modules: &[ModuleAudit],
    violations: &[BoundaryViolation],
    git: &GitAudit,
) -> Vec<InferredError> {
    let mut errors = Vec::new();
    if modules
        .iter()
        .any(|module| module.path.starts_with(HOST_ROOT) && module.git_status.starts_with("??"))
    {
        errors.push(InferredError {
            kind: "host_substrate_not_committed",
            evidence: vec![HOST_ROOT.to_string()],
            consequence: "The host substrate cannot be audited historically and cannot serve as stable authority yet.",
            correction: "Commit or explicitly quarantine cynic-host before moving more OS authority out of cynic-node.",
        });
    }
    if violations
        .iter()
        .any(|violation| violation.kind == "host_substrate_inside_node")
    {
        errors.push(InferredError {
            kind: "node_absorbed_host_substrate_too_early",
            evidence: violations
                .iter()
                .filter(|violation| violation.kind == "host_substrate_inside_node")
                .map(|violation| violation.path.clone())
                .take(12)
                .collect(),
            consequence: "cynic-node becomes an OS substrate crate instead of a node-local observer/runtime.",
            correction: "Move OS primitives into cynic-host and leave node with typed observation/command ports.",
        });
    }
    if violations
        .iter()
        .any(|violation| violation.kind == "provider_transport_inside_node_truth")
    {
        errors.push(InferredError {
            kind: "provider_transport_confused_with_truth",
            evidence: violations
                .iter()
                .filter(|violation| violation.kind == "provider_transport_inside_node_truth")
                .map(|violation| violation.path.clone())
                .take(12)
                .collect(),
            consequence: "Proxmox/Headscale/Tailscale fallback can masquerade as canonical fleet or network truth.",
            correction: "Keep provider code as adapters and require registry plus runtime proof before it affects status.",
        });
    }
    if git.diff_stat.stdout.contains("51 files changed")
        || git.diff_stat.stdout.contains("files changed")
    {
        errors.push(InferredError {
            kind: "large_unsequenced_absorption",
            evidence: vec![git.diff_stat.stdout.clone()],
            consequence: "Large node changes obscure which runtime proof each moved responsibility actually satisfies.",
            correction: "Split work into host substrate, node observation, network semantics, provider adapters, and artifact proof batches.",
        });
    }
    errors
}

fn git_status_for(root: &Path, relative: &str) -> String {
    let status = git_command(root, &["status", "--short", "--", relative]);
    status
        .stdout
        .lines()
        .next()
        .map(|line| line.chars().take(2).collect::<String>())
        .unwrap_or_else(|| "clean".to_string())
}

fn has(values: &[&'static str], expected: &'static str) -> bool {
    values.contains(&expected)
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
