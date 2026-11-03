//! Dependency plan domain logic.
//!
//! Pure domain helpers for building dependency upgrade plans (needs,
//! priority, and reason). The CLI-level report assembly (`deps_surface_plan`)
//! delegates the domain decisions here and keeps `cynic-pkg` free of CLI model
//! types.

use crate::types::ToolingInstallProvenance;

/// Whether a dependency needs an upgrade action.
///
/// A dependency needs action when it is `missing`, `stale`, or `desired_upgrade`,
/// or when its tooling/install authority is `noncanonical`.
pub fn needs_upgrade(dep_id: &str, status: &str, install_authority_status: &str) -> bool {
    matches!(status, "missing" | "stale" | "desired_upgrade")
        || install_authority_status == "noncanonical"
}

/// Compute the action priority for a dependency tier.
///
/// `required` keeps the base priority, `recommended` gets +100, and everything
/// else gets +200, so required fixes land first.
pub fn action_priority(tier: &str, priority: u32) -> u32 {
    match tier {
        "required" => priority,
        "recommended" => priority + 100,
        _ => priority + 200,
    }
}

/// Build the human-readable reason for a dependency action.
///
/// `min_version` is the resolved min pin from the pin view: either the declared
/// `min_version` or the `min_build` rendered as a string (the original
/// `build_plan` combines these with `or_else`).
pub fn reason_for_dep(
    dep_id: &str,
    status: &str,
    install_authority_status: &str,
    min_version: Option<String>,
    tooling_install_provenance: Option<&ToolingInstallProvenance>,
    expected_tooling_install_provenance: Option<&ToolingInstallProvenance>,
) -> String {
    match status {
        "missing" => format!("{} missing from PATH / principal bin", dep_id),
        "stale" => {
            format!("{} below pin ({:?})", dep_id, min_version)
        }
        "desired_upgrade" => format!(
            "{} at minimum but below desired (e.g. newer GGUF support)",
            dep_id
        ),
        _other if install_authority_status == "noncanonical" => format!(
            "{} tooling authority is noncanonical ({:?} observed, {:?} expected)",
            dep_id, tooling_install_provenance, expected_tooling_install_provenance
        ),
        other => other.to_string(),
    }
}
