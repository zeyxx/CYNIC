use serde::Deserialize;

use crate::domain::chunk::AuthorityTier;

/// Schema tag for `contracts/authority-tiers.json`.
pub const AUTHORITY_TIERS_SCHEMA_V1: &str = "cynic.knowledge.authority_tiers.v1";

/// Workspace-relative registry path consulted at ingestion time.
pub const AUTHORITY_TIERS_REGISTRY_PATH: &str = "contracts/authority-tiers.json";

/// Registry-driven authority downgrades for paths whose directory default
/// overstates their normative weight (e.g. operational JSON policies living
/// under `contracts/` next to real specifications).
#[derive(Debug, Default, Clone, Deserialize)]
pub struct AuthorityOverrides {
    /// Paths demoted to L3 operational notes.
    #[serde(default)]
    pub operational: Vec<String>,
    /// Paths demoted to L2 design documents.
    #[serde(default)]
    pub design: Vec<String>,
}

impl AuthorityOverrides {
    /// Parses registry content; pure, no I/O.
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }

    /// Returns the override tier for a workspace-relative path, if listed.
    pub fn resolve(&self, source_path: &str) -> Option<AuthorityTier> {
        if self.operational.iter().any(|p| p == source_path) {
            Some(AuthorityTier::L3Operation)
        } else if self.design.iter().any(|p| p == source_path) {
            Some(AuthorityTier::L2Design)
        } else {
            None
        }
    }
}

/// Resolves the ingestion authority for a path: registry override wins,
/// otherwise the historical directory default applies.
pub fn authority_for(source_path: &str, overrides: &AuthorityOverrides) -> AuthorityTier {
    overrides
        .resolve(source_path)
        .unwrap_or_else(|| AuthorityTier::from_path(source_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY: &str = r#"{
        "schema": "cynic.knowledge.authority_tiers.v1",
        "operational": ["contracts/builder-retention.json"],
        "design": ["docs/design/example.md"]
    }"#;

    #[test]
    fn operational_override_demotes_contract_to_l3() {
        let overrides = AuthorityOverrides::from_json(REGISTRY).expect("parse registry");
        assert_eq!(
            authority_for("contracts/builder-retention.json", &overrides),
            AuthorityTier::L3Operation
        );
    }

    #[test]
    fn design_override_demotes_doc_to_l2() {
        let overrides = AuthorityOverrides::from_json(REGISTRY).expect("parse registry");
        assert_eq!(
            authority_for("docs/design/example.md", &overrides),
            AuthorityTier::L2Design
        );
    }

    #[test]
    fn unlisted_paths_keep_directory_default() {
        let overrides = AuthorityOverrides::from_json(REGISTRY).expect("parse registry");
        assert_eq!(
            authority_for("contracts/foundation.json", &overrides),
            AuthorityTier::L0Constitution
        );
        assert_eq!(
            authority_for("docs/specs/example.md", &overrides),
            AuthorityTier::L1Specification
        );
    }

    #[test]
    fn malformed_registry_is_an_error_not_a_silent_default() {
        assert!(AuthorityOverrides::from_json("{not json").is_err());
    }
}
