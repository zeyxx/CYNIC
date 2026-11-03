//! Dependency evaluation domain logic.
//!
//! This module holds the pure domain logic for evaluating ecosystem
//! dependencies: pin/status evaluation, version parsing & comparison,
//! python-cli policy evaluation, and provider metadata lookups.
//! It is deliberately free of CLI-specific model types.

use crate::pins::PinEntry;
use crate::specs::UpgradeMode;
use crate::types::{InstallPrincipalKind, ObservedBinary, PrincipalInstallLayout, ProviderChannel, PythonInstallProvenance};
use crate::python_cli_policy::python_cli_policy;
use crate::providers::plan_for_dep;
use crate::registry::provider_spec;

/// Evaluate a dependency's pin status against its observed version.
///
/// Returns one of: `ok`, `stale`, `missing`, `unknown`, `desired_upgrade`.
pub fn evaluate_status(id: &str, version: Option<&str>, pin: Option<&PinEntry>) -> String {
    let Some(pin) = pin else {
        return if version.is_some() {
            "observed".into()
        } else {
            "unknown".into()
        };
    };

    if let Some(capability) = &pin.min_capability {
        return match version {
            Some(value) if value.contains(capability) || value.contains("inference-start") => {
                "ok".into()
            }
            Some(_) => "stale".into(),
            None => "unknown".into(),
        };
    }

    if let Some(want) = &pin.commit {
        return match version {
            Some(value) if value.contains(want.as_str()) => "ok".into(),
            Some(_) => "stale".into(),
            None => "unknown".into(),
        };
    }

    if let Some(min_build) = pin.min_build {
        let Some(value) = version else {
            return "unknown".into();
        };
        let Some(observed_build) = extract_build_number(value) else {
            return "unknown".into();
        };
        if observed_build < min_build {
            return "stale".into();
        }
        if let Some(desired_build) = pin.desired_build {
            if observed_build < desired_build {
                return "desired_upgrade".into();
            }
        }
        return "ok".into();
    }

    if let Some(min_version) = &pin.min_version {
        let Some(value) = version else {
            return "unknown".into();
        };
        return match cmp_loose_version(value, min_version) {
            Some(std::cmp::Ordering::Less) => "stale".into(),
            Some(_) => "ok".into(),
            None => "unknown".into(),
        };
    }

    let _ = id;
    if version.is_some() {
        "ok".into()
    } else {
        "observed".into()
    }
}

/// Extract a build number from a version string (e.g. `1.2.3.build456` -> `456`).
pub fn extract_build_number(s: &str) -> Option<u64> {
    for part in s.split(|c: char| !c.is_ascii_digit()) {
        if part.len() >= 3 && part.len() <= 6 {
            if let Ok(value) = part.parse::<u64>() {
                if value >= 100 {
                    return Some(value);
                }
            }
        }
    }
    None
}

/// Compare two loose version strings (`1.2.3` vs `1.2.4` -> `Less`).
pub fn cmp_loose_version(observed: &str, min: &str) -> Option<std::cmp::Ordering> {
    let observed = parse_version_triple(observed)?;
    let min = parse_version_triple(min)?;
    Some(observed.cmp(&min))
}

/// Parse a version triple (`major.minor.patch`) from a string.
pub fn parse_version_triple(s: &str) -> Option<(u64, u64, u64)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let slice = &s[start..i];
            let mut parts = slice.split('.').filter_map(|part| part.parse::<u64>().ok());
            let major = parts.next()?;
            let minor = parts.next().unwrap_or(0);
            let patch = parts.next().unwrap_or(0);
            return Some((major, minor, patch));
        }
        i += 1;
    }
    None
}

/// Whether the Python CLI for this dep can be normalized now (via uv).
pub fn python_cli_normalizable_now(layout: &PrincipalInstallLayout, dep_id: &str) -> bool {
    if !python_cli_requires_uv(dep_id) {
        return false;
    }
    if !python_cli_uv_in_runtime_layout(layout) {
        return false;
    }
    if python_cli_requires_runtime_principal(layout, dep_id) {
        return layout.writable;
    }
    true
}

/// Whether uv is present under the runtime layout (and under the principal).
pub fn python_cli_uv_in_runtime_layout(layout: &PrincipalInstallLayout) -> bool {
    let uv = resolve_binary("uv", layout);
    uv.path.is_some() && uv.under_principal
}

/// Whether this dep's python-cli policy is uv-first (canonical argv starts with `uv`).
pub fn python_cli_requires_uv(dep_id: &str) -> bool {
    python_cli_policy(dep_id)
        .map(|policy| policy.canonical_update_argv.first() == Some(&"uv"))
        .unwrap_or(false)
}

/// Whether this dep requires a runtime principal (writable layout) or is hf-cli.
pub fn python_cli_requires_runtime_principal(layout: &PrincipalInstallLayout, dep_id: &str) -> bool {
    python_cli_policy(dep_id)
        .map(|policy| {
            plan_for_dep(dep_id, "policy-check", layout)
                .map(|plan| {
                    plan.steps.iter().any(|step| step.as_principal)
                        || matches!(plan.principal, InstallPrincipalKind::RuntimeUser)
                })
                .unwrap_or(false)
                || policy.dep_id == "huggingface-cli"
        })
        .unwrap_or(false)
}

/// Whether this dep can auto-apply under its python-cli policy.
pub fn python_cli_can_auto_apply(layout: &PrincipalInstallLayout, dep_id: &str) -> bool {
    python_cli_policy(dep_id)
        .and_then(|_| plan_for_dep(dep_id, "policy-check", layout))
        .map(|plan| plan.auto_applicable)
        .unwrap_or(false)
}

/// Format a Python install provenance to a human-readable string.
pub fn format_python_provenance(provenance: PythonInstallProvenance) -> String {
    match provenance {
        PythonInstallProvenance::UvTool => "uv_tool",
        PythonInstallProvenance::UvVenv => "uv_venv",
        PythonInstallProvenance::VirtualEnv => "virtual_env",
        PythonInstallProvenance::Pipx => "pipx",
        PythonInstallProvenance::PipUserSite => "pip_user_site",
        PythonInstallProvenance::SystemPython => "system_python",
        PythonInstallProvenance::Unknown => "unknown",
    }
    .to_string()
}

/// Map a provider channel to an upgrade mode (domain-level mapping).
pub fn mode_from_channel(channel: ProviderChannel) -> UpgradeMode {
    match channel {
        ProviderChannel::Scripted => UpgradeMode::Scripted,
        ProviderChannel::External => UpgradeMode::External,
        ProviderChannel::DistroManaged => UpgradeMode::DistroManaged,
    }
}

/// Look up the provider channel for a dep id.
pub fn channel_for_dep(id: &str) -> ProviderChannel {
    provider_spec(id)
        .map(|spec| spec.channel)
        .unwrap_or(ProviderChannel::External)
}

/// Look up the install principal for a dep id.
pub fn principal_for_dep(id: &str) -> InstallPrincipalKind {
    provider_spec(id)
        .map(|spec| spec.principal)
        .unwrap_or(InstallPrincipalKind::OperatorLocal)
}

/// Resolve the absolute path of a binary under the principal layout (re-exported
/// from `probe` for convenience at the evaluate boundary).
pub fn resolve_binary(name: &str, layout: &PrincipalInstallLayout) -> ObservedBinary {
    crate::probe::resolve_binary(name, layout)
}
