//! Zone configuration — maps file paths to ownership zones.
//! Loaded from .claude/zones.json at boot. Immutable after load.
//! Data-centric: no state management, just resolution.

use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct ZoneConfig {
    zones: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Deserialize)]
struct ZoneFile {
    zones: BTreeMap<String, ZoneEntry>,
}

#[derive(Debug, Deserialize)]
struct ZoneEntry {
    paths: Vec<String>,
    #[serde(default)]
    _description: Option<String>,
}

impl ZoneConfig {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let file: ZoneFile =
            serde_json::from_str(json).map_err(|e| format!("invalid zones.json: {e}"))?;

        let zones: Vec<(String, Vec<String>)> = file
            .zones
            .into_iter()
            .map(|(name, entry)| (name, entry.paths))
            .collect();

        Ok(ZoneConfig { zones })
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        Self::from_json(&content)
    }

    /// Resolve a file path to its zone. Longest prefix match wins.
    pub fn resolve(&self, file_path: &str) -> Option<String> {
        let mut best: Option<(&str, usize)> = None;

        for (zone_name, paths) in &self.zones {
            for prefix in paths {
                if file_path.starts_with(prefix) && prefix.len() > best.map_or(0, |b| b.1) {
                    best = Some((zone_name.as_str(), prefix.len()));
                }
            }
        }

        best.map(|(name, _)| name.to_string())
    }

    /// Get all path prefixes for a given zone.
    pub fn paths_for_zone(&self, zone_name: &str) -> Option<Vec<String>> {
        self.zones
            .iter()
            .find(|(name, _)| name == zone_name)
            .map(|(_, paths)| paths.clone())
    }

    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_file_to_zone() {
        let config = ZoneConfig::from_json(
            r#"{
            "zones": {
                "api": { "paths": ["cynic-kernel/src/api/"], "description": "API" },
                "scripts": { "paths": ["scripts/", "infra/"], "description": "Scripts" }
            }
        }"#,
        )
        .unwrap();

        assert_eq!(
            config.resolve("cynic-kernel/src/api/rest/coord.rs"),
            Some("api".to_string())
        );
        assert_eq!(
            config.resolve("scripts/config-sync.sh"),
            Some("scripts".to_string())
        );
        assert_eq!(
            config.resolve("infra/systemd/foo.service"),
            Some("scripts".to_string())
        );
        assert_eq!(config.resolve("README.md"), None);
    }

    #[test]
    fn longest_prefix_wins() {
        let config = ZoneConfig::from_json(
            r#"{
            "zones": {
                "domain-core": { "paths": ["cynic-kernel/src/domain/"] },
                "domain-ccm": { "paths": ["cynic-kernel/src/domain/ccm/"] }
            }
        }"#,
        )
        .unwrap();

        assert_eq!(
            config.resolve("cynic-kernel/src/domain/ccm/crystal.rs"),
            Some("domain-ccm".to_string())
        );
        assert_eq!(
            config.resolve("cynic-kernel/src/domain/dog.rs"),
            Some("domain-core".to_string())
        );
    }

    #[test]
    fn empty_zones_resolves_none() {
        let config = ZoneConfig::default();
        assert_eq!(config.resolve("anything.rs"), None);
    }

    #[test]
    fn paths_for_zone_returns_prefixes() {
        let config =
            ZoneConfig::from_json(r#"{"zones": {"scripts": {"paths": ["scripts/", "infra/"]}}}"#)
                .unwrap();

        assert_eq!(
            config.paths_for_zone("scripts"),
            Some(vec!["scripts/".to_string(), "infra/".to_string()])
        );
        assert_eq!(config.paths_for_zone("nonexistent"), None);
    }
}
