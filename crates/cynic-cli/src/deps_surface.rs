//! Thin CLI surface for ecosystem tech-watch.
//!
//! Business logic lives in `cynic_host::domain::ecosystem_package`:
//! types / layout / port / registry / providers (not a monolith).
//!
//! This module: probe pins, print human/JSON, consent (`--yes` / token).
//! Pins SSOT: `infra/ecosystem-pins.json` (override: `CYNIC_ECOSYSTEM_PINS`).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_host::domain::ecosystem_package::{
    InstallPrincipalKind, PrincipalInstallLayout, ProviderChannel, RunAs, apply_for_dep,
    channel_auto_applicable, has_auto_handler, plan_for_dep, provider_spec, resolve_binary,
    resolve_principal_install_layout,
};
use serde::{Deserialize, Serialize};

// ── Pins file ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct PinsFile {
    pins: Vec<PinEntry>,
}

#[derive(Debug, Deserialize, Clone)]
struct PinEntry {
    id: String,
    #[serde(default)]
    min_version: Option<String>,
    #[serde(default)]
    min_build: Option<u64>,
    #[serde(default)]
    desired_build: Option<u64>,
    #[serde(default)]
    min_capability: Option<String>,
    #[serde(default)]
    required: Option<bool>,
    #[serde(default)]
    notes: Option<String>,
}

// ── Report types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct DepWatch {
    upstream: Option<&'static str>,
    check_hint: &'static str,
    update_hint: &'static str,
    update_commands: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DepEntry {
    id: &'static str,
    role: &'static str,
    category: &'static str,
    tier: &'static str,
    path: Option<String>,
    version: Option<String>,
    /// observed | missing | stale | desired_upgrade | unknown | ok
    status: String,
    /// Mapped from provider channel (scripted | external | distro_managed).
    upgrade_mode: UpgradeMode,
    channel: ProviderChannel,
    principal: InstallPrincipalKind,
    under_principal: bool,
    pin: Option<PinView>,
    watch: DepWatch,
}

#[derive(Debug, Serialize)]
struct PinView {
    min_version: Option<String>,
    min_build: Option<u64>,
    desired_build: Option<u64>,
    min_capability: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Serialize)]
struct DepsReport {
    schema: &'static str,
    surface: &'static str,
    observed_at_unix_ms: u64,
    pins_path: String,
    pins_loaded: bool,
    host_path_note: String,
    principal: PrincipalLayoutView,
    deps: Vec<DepEntry>,
    summary: DepsSummary,
    next: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PrincipalLayoutView {
    runtime_user: String,
    home_dir: String,
    bin_dir: String,
    src_dir: String,
    writable: bool,
    run_as: String,
    notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DepsSummary {
    observed: usize,
    missing: usize,
    stale: usize,
    desired_upgrade: usize,
    ok: usize,
    required_blocked: usize,
}

#[derive(Debug, Serialize)]
struct DepsPlanReport {
    schema: &'static str,
    surface: &'static str,
    observed_at_unix_ms: u64,
    pins_path: String,
    actions: Vec<PlanAction>,
    copy_paste: Vec<String>,
    frictionless: FrictionlessHints,
}

#[derive(Debug, Serialize)]
struct PlanAction {
    priority: u32,
    dep_id: String,
    reason: String,
    commands: Vec<String>,
    mutates: bool,
    upgrade_mode: UpgradeMode,
    channel: ProviderChannel,
    principal: InstallPrincipalKind,
    /// If true, deps-upgrade --apply can run this under human confirm token.
    auto_applicable: bool,
}

#[derive(Debug, Serialize)]
struct FrictionlessHints {
    after_cli_change: &'static str,
    daily_check: &'static str,
    model_start: &'static str,
    pins_edit: &'static str,
}

// ── Specs (what to probe) ───────────────────────────────────────────────────

struct DepSpec {
    id: &'static str,
    role: &'static str,
    category: &'static str,
    tier: &'static str,
    bins: &'static [&'static str],
    version_args: &'static [&'static str],
    version_from: VersionFrom,
    upstream: Option<&'static str>,
    check_hint: &'static str,
    update_hint: &'static str,
    update_commands: &'static [&'static str],
}

/// CLI-facing upgrade mode (stable labels for humans).
/// Maps 1:1 from [`ProviderChannel`] — no permanent "manual" dead-end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum UpgradeMode {
    /// CYNIC first-class apply (principal-local build/install).
    Scripted,
    /// Vendor/self-update CLI CYNIC may invoke.
    External,
    /// Distro/OS managed — plan only.
    DistroManaged,
}

fn mode_from_channel(channel: ProviderChannel) -> UpgradeMode {
    match channel {
        ProviderChannel::Scripted => UpgradeMode::Scripted,
        ProviderChannel::External => UpgradeMode::External,
        ProviderChannel::DistroManaged => UpgradeMode::DistroManaged,
    }
}

fn channel_for_dep(id: &str) -> ProviderChannel {
    provider_spec(id)
        .map(|s| s.channel)
        .unwrap_or(ProviderChannel::External)
}

fn principal_for_dep(id: &str) -> InstallPrincipalKind {
    provider_spec(id)
        .map(|s| s.principal)
        .unwrap_or(InstallPrincipalKind::OperatorLocal)
}

enum VersionFrom {
    StdoutFirstLine,
    JsonField(&'static str),
    HelpPresence,
}

fn specs() -> &'static [DepSpec] {
    &[
        DepSpec {
            id: "cynic",
            role: "organism_cli",
            category: "runtime",
            tier: "required",
            bins: &["cynic"],
            version_args: &["runtime", "--help"],
            version_from: VersionFrom::HelpPresence,
            upstream: Some("repo monorepo"),
            check_hint: "cynic runtime --help | grep inference-start",
            update_hint: "cynic deps upgrade --yes --only cynic  (reinstalls this binary to ~/.local/bin)",
            update_commands: &[
                "cynic deps upgrade --yes --only cynic",
                "# or from repo after rebuild: cynic bootstrap framework install-cli --apply --target ~/.local/bin/cynic",
            ],
        },
        DepSpec {
            id: "llama.cpp",
            role: "native_inference_engine",
            category: "runtime",
            tier: "required",
            bins: &["llama-server"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: Some("https://github.com/ggml-org/llama.cpp"),
            check_hint: "llama-server --version",
            update_hint: "cynic deps upgrade --yes --only llama.cpp  (principal-local cmake install)",
            update_commands: &["cynic deps upgrade --yes --only llama.cpp"],
        },
        DepSpec {
            id: "huggingface-cli",
            role: "model_hub_download",
            category: "hub",
            tier: "required",
            bins: &["hf", "huggingface-cli"],
            version_args: &["version", "--format", "json"],
            version_from: VersionFrom::JsonField("version"),
            upstream: Some("https://hf.co/cli"),
            check_hint: "hf version --format json",
            update_hint: "hf update",
            update_commands: &["hf update", "cynic runtime inference-hf-status --json"],
        },
        DepSpec {
            id: "ods",
            role: "ai_environment_bootstrap",
            category: "bootstrap",
            tier: "recommended",
            bins: &["ods"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: Some("https://github.com/Osmantic/ODS"),
            check_hint: "ods --help",
            update_hint: "cynic deps upgrade --yes --only ods  (principal-local pin + install.sh)",
            update_commands: &[
                "cynic deps upgrade --yes --only ods",
                "cynic runtime inference-discover --json",
            ],
        },
        DepSpec {
            id: "ollama",
            role: "compat_inference_peer",
            category: "runtime",
            tier: "optional",
            bins: &["ollama"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: Some("https://ollama.com"),
            check_hint: "ollama --version",
            update_hint: "vendor channel (external provider)",
            update_commands: &["# external: update ollama via vendor install"],
        },
        DepSpec {
            id: "codex-cli",
            role: "agent_coding",
            category: "agent_cli",
            tier: "recommended",
            bins: &["codex"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: Some("https://github.com/openai/codex"),
            check_hint: "codex --version",
            update_hint: "npm i -g @openai/codex (external channel)",
            update_commands: &["npm i -g @openai/codex@latest", "codex --version"],
        },
        DepSpec {
            id: "claude-code",
            role: "agent_coding",
            category: "agent_cli",
            tier: "optional",
            bins: &["claude"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: Some("https://docs.anthropic.com/en/docs/claude-code"),
            check_hint: "claude --version",
            update_hint: "claude update",
            update_commands: &["claude update", "claude --version"],
        },
        DepSpec {
            id: "grok-build",
            role: "agent_coding",
            category: "agent_cli",
            tier: "recommended",
            bins: &["grok"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: Some("https://x.ai"),
            check_hint: "grok --version",
            update_hint: "xAI / grok install channel (external)",
            update_commands: &[
                "# external: update grok via install channel",
                "grok --version",
            ],
        },
        DepSpec {
            id: "hermes-agent",
            role: "agent_runtime",
            category: "agent_cli",
            tier: "recommended",
            bins: &["hermes", "hermes-agent"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: None,
            check_hint: "hermes --version",
            update_hint: "external packaging channel",
            update_commands: &[
                "# external: update hermes per packaging",
                "hermes --version",
            ],
        },
        DepSpec {
            id: "antigravity-cli",
            role: "agent_cli_legacy",
            category: "agent_cli",
            tier: "optional",
            bins: &["agy"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: None,
            check_hint: "agy --version",
            update_hint: "legacy — prefer cynic/hermes",
            update_commands: &["# optional external: refresh agy if still required"],
        },
        DepSpec {
            id: "docker",
            role: "container_runtime",
            category: "container",
            tier: "recommended",
            bins: &["docker"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: Some("https://docs.docker.com/engine/install/"),
            check_hint: "docker --version",
            update_hint: "distro_managed (host package domain)",
            update_commands: &["# distro: apt/dnf install docker  # via host package domain"],
        },
        DepSpec {
            id: "rustc",
            role: "build_toolchain",
            category: "toolchain",
            tier: "required",
            bins: &["rustc"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: Some("https://rustup.rs"),
            check_hint: "rustc --version",
            update_hint: "rustup update (respect workspace pin)",
            update_commands: &[
                "rustup show",
                "# rustup update  # only if foundation build pin allows",
            ],
        },
        DepSpec {
            id: "cargo",
            role: "build_toolchain",
            category: "toolchain",
            tier: "required",
            bins: &["cargo"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: Some("https://rustup.rs"),
            check_hint: "cargo --version",
            update_hint: "rustup update",
            update_commands: &["cargo --version"],
        },
        DepSpec {
            id: "node",
            role: "js_toolchain",
            category: "toolchain",
            tier: "optional",
            bins: &["node"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: Some("https://nodejs.org"),
            check_hint: "node --version",
            update_hint: "external / nvm / distro",
            update_commands: &["node --version"],
        },
        DepSpec {
            id: "systemd",
            role: "service_manager",
            category: "runtime",
            tier: "required",
            bins: &["systemctl"],
            version_args: &["--version"],
            version_from: VersionFrom::StdoutFirstLine,
            upstream: Some("https://systemd.io"),
            check_hint: "systemctl --version",
            update_hint: "distro_managed",
            update_commands: &["systemctl --version"],
        },
    ]
}

// ── Public entrypoints ──────────────────────────────────────────────────────

/// `cynic deps` — human by default, `--json` for agents.
pub(crate) fn execute_deps(json: bool) -> ExitCode {
    let report = build_deps_report();
    if json {
        return emit(&report);
    }
    print_deps_human(&report);
    ExitCode::SUCCESS
}

/// `cynic deps plan` — human by default.
pub(crate) fn execute_deps_plan(json: bool) -> ExitCode {
    let deps = build_deps_report();
    let plan = build_plan(&deps);
    if json {
        return emit(&plan);
    }
    print_plan_human(&plan);
    ExitCode::SUCCESS
}

fn build_deps_report() -> DepsReport {
    let (pins_path, pins) = load_pins();
    let pin_map: std::collections::BTreeMap<String, PinEntry> =
        pins.into_iter().map(|p| (p.id.clone(), p)).collect();
    let layout = resolve_principal_install_layout();

    let mut deps = Vec::new();
    for spec in specs() {
        deps.push(probe_and_evaluate(spec, pin_map.get(spec.id), &layout));
    }

    let mut summary = DepsSummary {
        observed: 0,
        missing: 0,
        stale: 0,
        desired_upgrade: 0,
        ok: 0,
        required_blocked: 0,
    };
    for d in &deps {
        match d.status.as_str() {
            "missing" => {
                summary.missing += 1;
                if d.tier == "required" {
                    summary.required_blocked += 1;
                }
            }
            "stale" => {
                summary.stale += 1;
                if d.tier == "required" {
                    summary.required_blocked += 1;
                }
            }
            "desired_upgrade" => summary.desired_upgrade += 1,
            "ok" => summary.ok += 1,
            "observed" | "unknown" => summary.observed += 1,
            _ => summary.observed += 1,
        }
    }

    let mut next = Vec::new();
    if summary.required_blocked > 0 {
        next.push("cynic deps plan  # ordered fix actions".into());
    }
    next.push("cynic deps upgrade --yes  # apply scripted + external providers".into());
    next.push(
        "cynic inference-start <alias>  # or: cynic runtime inference-start <alias> --json".into(),
    );
    if deps.iter().any(|d| d.id == "ods" && d.status == "missing") {
        next.push(
            "Optional ODS: cynic deps upgrade --yes --only ods  then inference-discover".into(),
        );
    }
    if deps
        .iter()
        .any(|d| d.id == "llama.cpp" && (d.status == "stale" || d.status == "desired_upgrade"))
    {
        next.push(
            "llama.cpp: cynic deps upgrade --yes --only llama.cpp  (principal-local; newer GGUF arches)".into(),
        );
    }
    if !layout.writable {
        next.push(format!(
            "principal `{}` not writable — enable sudo -u {} or run as that user",
            layout.runtime_user, layout.runtime_user
        ));
    }

    DepsReport {
        schema: "cynic.ecosystem_deps.v1",
        surface: "tech_watch",
        observed_at_unix_ms: now_ms(),
        pins_path: pins_path.display().to_string(),
        pins_loaded: !pin_map.is_empty() || pins_path.exists(),
        host_path_note: format!(
            "principal bin {} should be on PATH before /usr/local/bin",
            layout.bin_dir.display()
        ),
        principal: principal_view(&layout),
        deps,
        summary,
        next,
    }
}

fn principal_view(layout: &PrincipalInstallLayout) -> PrincipalLayoutView {
    PrincipalLayoutView {
        runtime_user: layout.runtime_user.clone(),
        home_dir: layout.home_dir.display().to_string(),
        bin_dir: layout.bin_dir.display().to_string(),
        src_dir: layout.src_dir.display().to_string(),
        writable: layout.writable,
        run_as: match &layout.run_as {
            RunAs::Current => "current".into(),
            RunAs::SudoUser(u) => format!("sudo -u {u}"),
        },
        notes: layout.notes.clone(),
    }
}

fn build_plan(deps: &DepsReport) -> DepsPlanReport {
    let layout = resolve_principal_install_layout();
    let mut actions = Vec::new();
    let mut priority = 10u32;

    for d in &deps.deps {
        let needs = matches!(d.status.as_str(), "missing" | "stale" | "desired_upgrade");
        if !needs {
            continue;
        }
        // required first
        let prio = if d.tier == "required" {
            priority
        } else if d.tier == "recommended" {
            priority + 100
        } else {
            priority + 200
        };
        priority += 1;
        let reason = match d.status.as_str() {
            "missing" => format!("{} missing from PATH / principal bin", d.id),
            "stale" => format!(
                "{} below pin ({:?})",
                d.id,
                d.pin.as_ref().and_then(|p| p
                    .min_version
                    .clone()
                    .or_else(|| p.min_build.map(|b| b.to_string())))
            ),
            "desired_upgrade" => format!(
                "{} at minimum but below desired (e.g. newer GGUF support)",
                d.id
            ),
            other => other.to_string(),
        };

        // Prefer first-class provider plan when available.
        let (commands, auto_applicable) = if let Some(plan) = plan_for_dep(d.id, &reason, &layout) {
            (plan.summary_commands, plan.auto_applicable)
        } else {
            let auto =
                channel_auto_applicable(d.channel) && d.id != "systemd" && has_auto_handler(d.id);
            (d.watch.update_commands.clone(), auto)
        };

        actions.push(PlanAction {
            priority: prio,
            dep_id: d.id.to_string(),
            reason,
            commands,
            mutates: true,
            upgrade_mode: d.upgrade_mode,
            channel: d.channel,
            principal: d.principal,
            auto_applicable,
        });
    }
    actions.sort_by_key(|a| a.priority);

    let mut copy_paste = Vec::new();
    copy_paste.push("set -euo pipefail".into());
    copy_paste.push("# Prefer: cynic deps upgrade --yes".into());
    for a in &actions {
        copy_paste.push(format!(
            "# --- {} [{} / {:?}] : {} ---",
            a.dep_id,
            match a.channel {
                ProviderChannel::Scripted => "scripted",
                ProviderChannel::External => "external",
                ProviderChannel::DistroManaged => "distro_managed",
            },
            a.principal,
            a.reason
        ));
        for c in &a.commands {
            copy_paste.push(c.clone());
        }
    }
    copy_paste.push("cynic deps".into());

    DepsPlanReport {
        schema: "cynic.ecosystem_deps_plan.v1",
        surface: "tech_watch",
        observed_at_unix_ms: now_ms(),
        pins_path: deps.pins_path.clone(),
        actions,
        copy_paste,
        frictionless: FrictionlessHints {
            after_cli_change: "cynic deps upgrade --yes --only cynic",
            daily_check: "cynic deps",
            model_start: "cynic runtime inference-start <alias> --json",
            pins_edit: "edit infra/ecosystem-pins.json then: cynic deps",
        },
    }
}

// ── Probe + pin evaluate ────────────────────────────────────────────────────

fn probe_and_evaluate(
    spec: &DepSpec,
    pin: Option<&PinEntry>,
    layout: &PrincipalInstallLayout,
) -> DepEntry {
    let channel = channel_for_dep(spec.id);
    let principal = principal_for_dep(spec.id);
    let upgrade_mode = mode_from_channel(channel);

    // Prefer principal layout binaries for principal-owned deps.
    let mut path = None;
    let mut under_principal = false;
    for b in spec.bins {
        let obs = resolve_binary(b, layout);
        if let Some(p) = obs.path {
            under_principal = obs.under_principal;
            path = Some(p);
            break;
        }
    }
    // Ambient PATH fallback (already covered by resolve_binary which(), but
    // multi-bin aliases may need second bin name).
    if path.is_none() {
        for b in spec.bins {
            if let Some(p) = which(b) {
                path = Some(p);
                break;
            }
        }
    }

    let watch = DepWatch {
        upstream: spec.upstream,
        check_hint: spec.check_hint,
        update_hint: spec.update_hint,
        update_commands: if let Some(plan) = plan_for_dep(spec.id, "refresh", layout) {
            plan.summary_commands
        } else {
            spec.update_commands
                .iter()
                .map(|s| (*s).to_string())
                .collect()
        },
    };

    let pin_view = pin.map(|p| PinView {
        min_version: p.min_version.clone(),
        min_build: p.min_build,
        desired_build: p.desired_build,
        min_capability: p.min_capability.clone(),
        notes: p.notes.clone(),
    });

    let Some(path) = path else {
        let _optional = pin.map(|p| p.required == Some(false)).unwrap_or(false);
        return DepEntry {
            id: spec.id,
            role: spec.role,
            category: spec.category,
            tier: if _optional { "optional" } else { spec.tier },
            path: None,
            version: None,
            status: "missing".into(),
            upgrade_mode,
            channel,
            principal,
            under_principal: false,
            pin: pin_view,
            watch,
        };
    };

    let version = read_version(spec, &path);
    let status = evaluate_status(spec.id, version.as_deref(), pin);

    DepEntry {
        id: spec.id,
        role: spec.role,
        category: spec.category,
        tier: spec.tier,
        path: Some(path),
        version,
        status,
        upgrade_mode,
        channel,
        principal,
        under_principal,
        pin: pin_view,
        watch,
    }
}

fn evaluate_status(id: &str, version: Option<&str>, pin: Option<&PinEntry>) -> String {
    let Some(pin) = pin else {
        return if version.is_some() {
            "observed".into()
        } else {
            "unknown".into()
        };
    };

    // capability pin (cynic)
    if let Some(cap) = &pin.min_capability {
        return match version {
            Some(v) if v.contains(cap) || v.contains("inference-start") => "ok".into(),
            Some(_) => "stale".into(),
            None => "unknown".into(),
        };
    }

    // build number (llama.cpp "version: 9444")
    if let Some(min_b) = pin.min_build {
        let Some(v) = version else {
            return "unknown".into();
        };
        let Some(obs) = extract_build_number(v) else {
            return "unknown".into();
        };
        if obs < min_b {
            return "stale".into();
        }
        if let Some(des) = pin.desired_build {
            if obs < des {
                return "desired_upgrade".into();
            }
        }
        return "ok".into();
    }

    // semver-ish
    if let Some(min_v) = &pin.min_version {
        let Some(v) = version else {
            return "unknown".into();
        };
        return match cmp_loose_version(v, min_v) {
            Some(std::cmp::Ordering::Less) => "stale".into(),
            Some(_) => "ok".into(),
            None => "unknown".into(),
        };
    }

    // no numeric pin — present is ok
    let _ = id;
    if version.is_some() {
        "ok".into()
    } else {
        "observed".into()
    }
}

fn extract_build_number(s: &str) -> Option<u64> {
    // "version: 9444 (6f165c1c6)" or "b9444"
    for part in s.split(|c: char| !c.is_ascii_digit()) {
        if part.len() >= 3 && part.len() <= 6 {
            if let Ok(n) = part.parse::<u64>() {
                if n >= 100 {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn cmp_loose_version(observed: &str, min: &str) -> Option<std::cmp::Ordering> {
    let o = parse_version_triple(observed)?;
    let m = parse_version_triple(min)?;
    Some(o.cmp(&m))
}

fn parse_version_triple(s: &str) -> Option<(u64, u64, u64)> {
    // find first X.Y.Z or X.Y in string
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let slice = &s[start..i];
            let mut parts = slice.split('.').filter_map(|p| p.parse::<u64>().ok());
            let a = parts.next()?;
            let b = parts.next().unwrap_or(0);
            let c = parts.next().unwrap_or(0);
            return Some((a, b, c));
        }
        i += 1;
    }
    None
}

fn read_version(spec: &DepSpec, path: &str) -> Option<String> {
    match spec.version_from {
        VersionFrom::HelpPresence => {
            let out = Command::new(path).args(spec.version_args).output().ok()?;
            let s = String::from_utf8_lossy(&out.stdout);
            if s.contains("inference-start") {
                Some("cynic-cli+inference-start".into())
            } else if out.status.success() {
                Some("present-old-surface".into())
            } else {
                Some("present".into())
            }
        }
        VersionFrom::StdoutFirstLine => {
            let out = Command::new(path).args(spec.version_args).output().ok()?;
            let s = String::from_utf8_lossy(&out.stdout);
            let e = String::from_utf8_lossy(&out.stderr);
            let line = s
                .lines()
                .next()
                .or_else(|| e.lines().next())
                .unwrap_or("")
                .trim();
            if line.is_empty() {
                None
            } else {
                Some(line.chars().take(120).collect())
            }
        }
        VersionFrom::JsonField(field) => {
            let out = Command::new(path).args(spec.version_args).output().ok()?;
            let s = String::from_utf8_lossy(&out.stdout);
            serde_json::from_str::<serde_json::Value>(&s)
                .ok()
                .and_then(|v| v.get(field)?.as_str().map(|x| x.to_string()))
                .or_else(|| s.lines().next().map(|l| l.trim().to_string()))
        }
    }
}

// ── Pins load ───────────────────────────────────────────────────────────────

fn load_pins() -> (PathBuf, Vec<PinEntry>) {
    let path = resolve_pins_path();
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return (path, Vec::new());
    };
    match serde_json::from_str::<PinsFile>(&raw) {
        Ok(f) => (path, f.pins),
        Err(_) => (path, Vec::new()),
    }
}

fn resolve_pins_path() -> PathBuf {
    if let Ok(p) = std::env::var("CYNIC_ECOSYSTEM_PINS") {
        return PathBuf::from(p);
    }
    // Walk up from cwd for repo root
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for _ in 0..8 {
        let candidate = dir.join("infra/ecosystem-pins.json");
        if candidate.is_file() {
            return candidate;
        }
        if !dir.pop() {
            break;
        }
    }
    PathBuf::from("infra/ecosystem-pins.json")
}

fn which(bin: &str) -> Option<String> {
    let out = Command::new("which").arg(bin).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if p.is_empty() { None } else { Some(p) }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn emit(value: &impl Serialize) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(payload) => {
            println!("{payload}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("json error: {e}");
            ExitCode::FAILURE
        }
    }
}

// ── Human-validated upgrade apply ───────────────────────────────────────────

#[derive(Debug, Serialize)]
struct UpgradeReport {
    schema: &'static str,
    surface: &'static str,
    mutates: bool,
    requested_apply: bool,
    confirm_token: String,
    confirm_required: bool,
    human_validation: HumanValidation,
    candidates: Vec<UpgradeCandidate>,
    results: Vec<UpgradeResult>,
    after: Vec<String>,
}

#[derive(Debug, Serialize)]
struct HumanValidation {
    steps: Vec<&'static str>,
    apply_command_template: String,
}

#[derive(Debug, Serialize, Clone)]
struct UpgradeCandidate {
    dep_id: String,
    status: String,
    upgrade_mode: UpgradeMode,
    auto_applicable: bool,
    reason: String,
    commands: Vec<String>,
}

#[derive(Debug, Serialize)]
struct UpgradeResult {
    dep_id: String,
    attempted: bool,
    ok: bool,
    argv: Vec<String>,
    exit_code: Option<i32>,
    stdout_tail: String,
    stderr_tail: String,
    message: String,
}

/// One-click / human-validated upgrade.
///
/// - default: show plan (human)
/// - `--yes`: apply all scripted candidates in one step (explicit human consent)
/// - `--apply --confirm TOKEN`: non-interactive (agents/CI)
/// - `--json`: machine report
pub(crate) fn execute_deps_upgrade(
    json: bool,
    apply: bool,
    yes: bool,
    confirm: Option<&str>,
    only: &[String],
) -> ExitCode {
    let deps = build_deps_report();
    let plan = build_plan(&deps);
    let mut candidates: Vec<UpgradeCandidate> = plan
        .actions
        .iter()
        .map(|a| UpgradeCandidate {
            dep_id: a.dep_id.clone(),
            status: deps
                .deps
                .iter()
                .find(|d| d.id == a.dep_id)
                .map(|d| d.status.clone())
                .unwrap_or_else(|| "unknown".into()),
            upgrade_mode: a.upgrade_mode,
            auto_applicable: a.auto_applicable,
            reason: a.reason.clone(),
            commands: a.commands.clone(),
        })
        .collect();

    if !only.is_empty() {
        let mut selected = Vec::new();
        for id in only {
            if let Some(c) = candidates.iter().find(|c| c.dep_id == *id) {
                selected.push(c.clone());
                continue;
            }
            if let Some(d) = deps.deps.iter().find(|d| d.id == id.as_str()) {
                if has_auto_handler(d.id) || d.id == "cynic" {
                    selected.push(UpgradeCandidate {
                        dep_id: d.id.to_string(),
                        status: d.status.clone(),
                        upgrade_mode: d.upgrade_mode,
                        auto_applicable: true,
                        reason: format!("operator --only refresh ({})", d.status),
                        commands: d.watch.update_commands.clone(),
                    });
                }
            }
        }
        candidates = selected;
    }

    let token = confirm_token_for(&candidates);
    let human = HumanValidation {
        steps: vec![
            "One-click (recommended):  cynic deps upgrade --yes",
            "Or two-step:              cynic deps upgrade   # note token",
            "                          cynic deps upgrade --apply --confirm <token>",
            "Then verify:              cynic deps",
        ],
        apply_command_template: format!("cynic deps upgrade --yes"),
    };

    let do_apply = yes || apply;
    if !do_apply {
        let report = UpgradeReport {
            schema: "cynic.ecosystem_deps_upgrade.v1",
            surface: "tech_watch",
            mutates: false,
            requested_apply: false,
            confirm_token: token.clone(),
            confirm_required: true,
            human_validation: human,
            candidates: candidates.clone(),
            results: vec![],
            after: vec![
                "No changes applied.".into(),
                format!("One-click: cynic deps upgrade --yes"),
                format!("Or:        cynic deps upgrade --apply --confirm {token}"),
            ],
        };
        if json {
            return emit(&report);
        }
        print_upgrade_human(&report, false);
        return ExitCode::SUCCESS;
    }

    // APPLY: --yes is explicit human consent; --confirm token is the non-interactive path.
    if !yes {
        let Some(provided) = confirm else {
            eprintln!("Need human validation: use --yes  OR  --apply --confirm <token>");
            return ExitCode::FAILURE;
        };
        if provided != token {
            eprintln!("confirm token mismatch — run: cynic deps upgrade");
            return ExitCode::FAILURE;
        }
    }

    if !json {
        println!(
            "Applying scripted upgrades (human consent: {})…\n",
            if yes { "--yes" } else { "--confirm" }
        );
    }

    let mut results = Vec::new();
    let mut any_fail = false;
    let mut any_scripted = false;
    for c in &candidates {
        if !c.auto_applicable {
            if !json {
                println!(
                    "  skip  {}  ({:?} — not auto-applicable; see cynic deps plan)",
                    c.dep_id, c.upgrade_mode
                );
            }
            results.push(UpgradeResult {
                dep_id: c.dep_id.clone(),
                attempted: false,
                ok: false,
                argv: vec![],
                exit_code: None,
                stdout_tail: String::new(),
                stderr_tail: String::new(),
                message: format!("channel_not_auto:{:?}", c.upgrade_mode),
            });
            continue;
        }
        any_scripted = true;
        if !json {
            println!("  run   {}  …", c.dep_id);
        }
        match run_scripted_upgrade(&c.dep_id) {
            Ok(r) => {
                if !json {
                    println!("        → {}", if r.ok { "ok" } else { "FAILED" });
                }
                if !r.ok {
                    any_fail = true;
                }
                results.push(r);
            }
            Err(e) => {
                any_fail = true;
                if !json {
                    println!("        → error: {e}");
                }
                results.push(UpgradeResult {
                    dep_id: c.dep_id.clone(),
                    attempted: true,
                    ok: false,
                    argv: vec![],
                    exit_code: None,
                    stdout_tail: String::new(),
                    stderr_tail: e,
                    message: "execution error".into(),
                });
            }
        }
    }

    if !any_scripted && !json {
        println!(
            "Nothing auto-applicable (distro_managed only, blocked principal, or already ok)."
        );
    }

    let report = UpgradeReport {
        schema: "cynic.ecosystem_deps_upgrade.v1",
        surface: "tech_watch",
        mutates: true,
        requested_apply: true,
        confirm_token: token,
        confirm_required: true,
        human_validation: human,
        candidates,
        results,
        after: vec!["cynic deps".into(), "cynic inference status".into()],
    };
    if json {
        let code = emit(&report);
        if any_fail { ExitCode::FAILURE } else { code }
    } else {
        print_upgrade_human(&report, true);
        if any_fail {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}

fn print_deps_human(report: &DepsReport) {
    println!("CYNIC ecosystem deps");
    println!(
        "  pins: {} ({})",
        report.pins_path,
        if report.pins_loaded {
            "loaded"
        } else {
            "missing"
        }
    );
    let p = &report.principal;
    println!(
        "  principal: user={}  bin={}  writable={}  run_as={}",
        p.runtime_user, p.bin_dir, p.writable, p.run_as
    );
    let s = &report.summary;
    println!(
        "  summary: ok={}  stale={}  desired_upgrade={}  missing={}  blocked_required={}\n",
        s.ok, s.stale, s.desired_upgrade, s.missing, s.required_blocked
    );
    println!(
        "  {:16} {:12} {:14} {:18} {}",
        "STATUS", "TIER", "CHANNEL", "ID", "VERSION"
    );
    println!("  {}", "-".repeat(80));
    for d in &report.deps {
        let mark = match d.status.as_str() {
            "ok" => " ",
            "desired_upgrade" => "~",
            "stale" | "missing" => "!",
            _ => "?",
        };
        let ch = match d.channel {
            ProviderChannel::Scripted => "scripted",
            ProviderChannel::External => "external",
            ProviderChannel::DistroManaged => "distro",
        };
        println!(
            "{mark} {:16} {:12} {:14} {:18} {}",
            d.status,
            d.tier,
            ch,
            d.id,
            d.version.as_deref().unwrap_or("-")
        );
    }
    if !report.next.is_empty() {
        println!("\nNext:");
        for n in &report.next {
            println!("  • {n}");
        }
    }
    println!("\nOne-click upgrades:  cynic deps upgrade --yes");
    println!("Full plan:           cynic deps plan");
    println!("JSON:                cynic deps --json");
}

fn print_plan_human(plan: &DepsPlanReport) {
    println!("CYNIC deps upgrade plan\n");
    if plan.actions.is_empty() {
        println!("  (nothing to do — all pins satisfied or optional missing)");
    }
    for a in &plan.actions {
        let mode = if a.auto_applicable {
            "ONE-CLICK"
        } else {
            match a.upgrade_mode {
                UpgradeMode::DistroManaged => "DISTRO  ",
                UpgradeMode::External => "EXTERNAL",
                UpgradeMode::Scripted => "BLOCKED ",
            }
        };
        println!("  [{mode}] {} ({:?}) — {}", a.dep_id, a.principal, a.reason);
        for c in &a.commands {
            println!("           {c}");
        }
        println!();
    }
    println!("Frictionless:");
    println!("  {}", plan.frictionless.daily_check);
    println!("  {}", plan.frictionless.after_cli_change);
    println!("  cynic deps upgrade --yes   # apply all ONE-CLICK providers");
}

fn print_upgrade_human(report: &UpgradeReport, applied: bool) {
    if !applied {
        println!("CYNIC deps upgrade (preview)\n");
        println!("  confirm_token: {}", report.confirm_token);
        println!("  one-click:     cynic deps upgrade --yes\n");
        for c in &report.candidates {
            let tag = if c.auto_applicable {
                "ONE-CLICK"
            } else {
                "PLAN    "
            };
            println!("  [{tag}] {} ({}) — {}", c.dep_id, c.status, c.reason);
        }
        if report.candidates.is_empty() {
            println!("  (no pending upgrades)");
        }
        println!();
        for s in &report.human_validation.steps {
            println!("  {s}");
        }
        return;
    }
    println!("\nCYNIC deps upgrade results\n");
    for r in &report.results {
        if !r.attempted {
            continue;
        }
        println!(
            "  {}  {}  {}",
            if r.ok { "ok  " } else { "FAIL" },
            r.dep_id,
            r.message
        );
    }
    println!("\nVerify:  cynic deps");
}

fn confirm_token_for(candidates: &[UpgradeCandidate]) -> String {
    let mut material = String::new();
    for c in candidates {
        material.push_str(&c.dep_id);
        material.push(':');
        material.push_str(&c.status);
        material.push(':');
        material.push_str(match c.upgrade_mode {
            UpgradeMode::Scripted => "scripted",
            UpgradeMode::External => "external",
            UpgradeMode::DistroManaged => "distro_managed",
        });
        material.push('|');
    }
    // Include calendar day so tokens expire naturally next day
    let day = now_ms() / (86_400_000);
    material.push_str(&format!("day:{day}"));
    format!("cynic-up-{:016x}", fnv1a64(material.as_bytes()))
}

fn fnv1a64(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for b in data {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn run_scripted_upgrade(dep_id: &str) -> Result<UpgradeResult, String> {
    // Organism self-install stays in CLI (needs current_exe + framework_bootstrap).
    if dep_id == "cynic" {
        return upgrade_cynic_self();
    }

    // All other first-class providers: host port + adapters.
    let layout = resolve_principal_install_layout();
    let Some(result) = apply_for_dep(dep_id, "deps upgrade --yes", &layout) else {
        return Ok(UpgradeResult {
            dep_id: dep_id.into(),
            attempted: false,
            ok: false,
            argv: vec![],
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: String::new(),
            message: "no provider handler".into(),
        });
    };
    let last_argv = result
        .steps_run
        .last()
        .map(|s| s.argv.clone())
        .unwrap_or_default();
    let stdout = result
        .steps_run
        .iter()
        .map(|s| s.stdout_tail.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let stderr = result
        .steps_run
        .iter()
        .map(|s| format!("{}: {}", s.role, s.stderr_tail))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(UpgradeResult {
        dep_id: dep_id.into(),
        attempted: result.attempted,
        ok: result.ok,
        argv: last_argv,
        exit_code: result.steps_run.last().and_then(|s| s.exit_code),
        stdout_tail: tail(&stdout, 1500),
        stderr_tail: tail(&stderr, 1500),
        message: result.message,
    })
}

/// Native self-install: reinstall current cynic binary to ~/.local/bin (no make).
fn upgrade_cynic_self() -> Result<UpgradeResult, String> {
    let source = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let target = default_user_cli_target();
    let report = cynic_host::framework_bootstrap::install_standalone_cli(
        Some(source.to_str().unwrap_or("")),
        Some(target.to_str().unwrap_or("")),
        true,
    )?;
    let ok = matches!(report.status, "installed" | "already_installed");
    Ok(UpgradeResult {
        dep_id: "cynic".into(),
        attempted: true,
        ok,
        argv: report.command,
        exit_code: report.exit_code,
        stdout_tail: report.stdout,
        stderr_tail: report.stderr,
        message: report.status.to_string(),
    })
}

fn default_user_cli_target() -> PathBuf {
    if let Ok(p) = std::env::var("CYNIC_STANDALONE_CLI") {
        return PathBuf::from(p);
    }
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".local/bin/cynic"))
        .unwrap_or_else(|| PathBuf::from("/usr/local/bin/cynic"))
}

fn tail(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.len() <= max {
        t.to_string()
    } else {
        t[t.len().saturating_sub(max)..].to_string()
    }
}

#[allow(dead_code)]
fn _pins_path_exists(p: &Path) -> bool {
    p.exists()
}
