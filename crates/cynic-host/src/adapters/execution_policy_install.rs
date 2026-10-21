//! Install the deny-by-default execution policy for this process.
//!
//! WHY THIS EXISTS.
//!
//! The policy in `cynic_protocol::execution_policy` is pure: it decides.
//! This adapter is what makes the decision effective. Without an
//! installed policy the runner observes, which is the historical
//! behaviour: every argv is refused and still runs. An agent holding a
//! shell could therefore reach the builder with an ad-hoc ssh line and
//! compile on a host that is not allowed to compile.
//!
//! Installing the policy is the act that closes that gap, so it happens
//! once, at the process entry point, from the declared catalogue. The
//! ruleset is therefore the catalogue the operator reviews, not a
//! hardcoded list in the code.
//!
//! A catalogue that cannot be read or that fails validation does NOT
//! install a permissive policy. The failure is reported, and the caller
//! decides: a surface that cannot read its policy must not pretend it
//! is protected.

use cynic_protocol::argv_allow::AllowRule;
use cynic_protocol::execution_policy::{ExecutionPolicy, rules_from_catalog};
use cynic_protocol::privilege::CapabilityCatalog;

/// The outcome of the installation attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyInstallation {
    /// The policy is installed; the ruleset carries this many rules.
    Installed { rules: usize },
    /// No policy was installed: the catalogue could not be read or
    /// parsed. The reason is named.
    Unavailable { reason: String },
}

/// Read the declared catalogue and build the policy from it.
///
/// The catalogue is the only source of rules. A catalogue that is
/// structurally invalid is an error, not an empty ruleset: an empty
/// ruleset would refuse everything, which is safe but wrong, and a
/// permissive one would be unsafe.
pub fn policy_from_catalog(catalog: &CapabilityCatalog) -> ExecutionPolicy {
    let rules: Vec<AllowRule> = rules_from_catalog(catalog);
    ExecutionPolicy::new(rules)
}

/// Read the catalogue from a path and build the policy.
pub fn policy_from_catalog_path(path: &std::path::Path) -> Result<ExecutionPolicy, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    let catalog: CapabilityCatalog =
        serde_json::from_str(&raw).map_err(|error| format!("parse {}: {error}", path.display()))?;
    let structural = catalog.validate();
    if !structural.is_empty() {
        return Err(format!(
            "catalog {} is invalid: {}",
            path.display(),
            structural.join(", ")
        ));
    }
    Ok(policy_from_catalog(&catalog))
}

/// Install the policy built from the catalogue at `path`.
///
/// The first installation wins: the process-wide slot is write-once, so
/// a later caller cannot replace a narrow policy with a wide one.
pub fn install_policy_from_catalog_path(path: &std::path::Path) -> PolicyInstallation {
    match policy_from_catalog_path(path) {
        Ok(policy) => {
            let rules = policy.rules.len();
            crate::adapters::argv_runner::install_policy(policy);
            PolicyInstallation::Installed { rules }
        }
        Err(reason) => PolicyInstallation::Unavailable { reason },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog_json() -> &'static str {
        r#"{
          "schema": "cynic.capability_catalog.v1",
          "capabilities": [
            {
              "id": "pct-list",
              "domain": "hypervisor",
              "argv_template": ["/usr/sbin/pct", "list"],
              "params": [],
              "requires_root": true,
              "mandatory": false,
              "purpose": "CT inventory"
            }
          ]
        }"#
    }

    #[test]
    fn a_catalog_becomes_a_policy() {
        let catalog: CapabilityCatalog = serde_json::from_str(catalog_json()).expect("parses");
        let policy = policy_from_catalog(&catalog);
        assert_eq!(policy.rules.len(), 1);
        assert_eq!(policy.rules[0].program, "/usr/sbin/pct");
    }

    #[test]
    fn an_unreadable_catalog_is_an_error_not_a_permissive_policy() {
        let error = policy_from_catalog_path(std::path::Path::new("/nonexistent/catalog.json"))
            .expect_err("must fail");
        assert!(error.contains("read"));
    }

    #[test]
    fn an_invalid_catalog_is_an_error() {
        let dir = std::env::temp_dir().join(format!("cynic-pol-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("catalog.json");
        std::fs::write(
            &path,
            r#"{"schema":"cynic.capability_catalog.v1","capabilities":[]}"#,
        )
        .expect("write");
        let error = policy_from_catalog_path(&path).expect_err("must fail");
        assert!(error.contains("invalid"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
