//! Release SBOM inventory — pure `cargo metadata` to CycloneDX transform.
//!
//! The packaging adapter runs `cargo metadata --format-version 1` and feeds
//! the JSON here. This module owns no I/O and no toolchain: it parses the
//! package closure and emits a minimal, deterministic CycloneDX 1.5
//! document. Determinism (sorted components, no timestamps, no serial
//! numbers) keeps release baskets reproducible: the same closure always
//! yields the same bytes, so the SBOM digest is a stable release input.

use serde::Serialize;

pub const RELEASE_SBOM_BOM_FORMAT: &str = "CycloneDX";
pub const RELEASE_SBOM_SPEC_VERSION: &str = "1.5";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SbomComponent {
    #[serde(rename = "type")]
    pub component_type: &'static str,
    pub name: String,
    pub version: String,
    #[serde(rename = "bom-ref")]
    pub bom_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SbomDocument {
    #[serde(rename = "bomFormat")]
    bom_format: &'static str,
    #[serde(rename = "specVersion")]
    spec_version: &'static str,
    version: u32,
    metadata: SbomMetadata,
    components: Vec<SbomComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SbomMetadata {
    component: SbomRootComponent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SbomRootComponent {
    #[serde(rename = "type")]
    component_type: &'static str,
    name: String,
    version: String,
}

/// Build a deterministic CycloneDX inventory from `cargo metadata` JSON.
///
/// `metadata_json` is the stdout of
/// `cargo metadata --format-version 1 --no-deps`. `root_name`/`root_version`
/// identify the workspace being released. Returns the canonical document
/// bytes. Fails closed on unparseable input, wrong format version, or an
/// empty package closure — an SBOM that cannot enumerate its closure proves
/// nothing and must never ship as an empty success.
pub fn build_release_sbom_cyclonedx(
    metadata_json: &str,
    root_name: &str,
    root_version: &str,
) -> Result<Vec<u8>, String> {
    if root_name.trim().is_empty() || root_version.trim().is_empty() {
        return Err("release_sbom_root_identity_missing".to_string());
    }
    let metadata: serde_json::Value = serde_json::from_str(metadata_json)
        .map_err(|_| "release_sbom_metadata_unparseable".to_string())?;
    // NOTE: `cargo metadata --format-version 1` selects the format at
    // invocation but emits it as `version`, not `format_version` (verified
    // against live output: 568 packages, no `format_version` key).
    if metadata.get("version").and_then(|v| v.as_u64()) != Some(1) {
        return Err("release_sbom_metadata_format_unsupported".to_string());
    }
    let packages = metadata
        .get("packages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "release_sbom_packages_missing".to_string())?;
    let mut components = Vec::with_capacity(packages.len());
    for package in packages {
        let name = package
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| "release_sbom_package_identity_invalid".to_string())?;
        let version = package
            .get("version")
            .and_then(|v| v.as_str())
            .filter(|version| !version.trim().is_empty())
            .ok_or_else(|| "release_sbom_package_identity_invalid".to_string())?;
        components.push(SbomComponent {
            component_type: "library",
            name: name.to_string(),
            version: version.to_string(),
            bom_ref: format!("pkg:cargo/{name}@{version}"),
        });
    }
    if components.is_empty() {
        return Err("release_sbom_closure_empty".to_string());
    }
    components.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.version.cmp(&right.version))
    });
    let document = SbomDocument {
        bom_format: RELEASE_SBOM_BOM_FORMAT,
        spec_version: RELEASE_SBOM_SPEC_VERSION,
        version: 1,
        metadata: SbomMetadata {
            component: SbomRootComponent {
                component_type: "application",
                name: root_name.to_string(),
                version: root_version.to_string(),
            },
        },
        components,
    };
    crate::canonical_json_bytes(&document)
        .map_err(|_| "release_sbom_serialization_failed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_METADATA: &str = r#"{
        "packages": [
            {"name": "zerocopy", "version": "0.8.26", "id": "a"},
            {"name": "anyhow", "version": "1.0.95", "id": "b"},
            {"name": "anyhow", "version": "0.9.0", "id": "c"}
        ],
        "workspace_members": ["a"],
        "resolve": null,
        "target_directory": "/tmp/target",
        "version": 1,
        "workspace_root": "/tmp",
        "metadata": null
    }"#;

    #[test]
    fn sbom_lists_sorted_components_with_purls() {
        let bytes = build_release_sbom_cyclonedx(FIXTURE_METADATA, "cynic", "0.1.0").unwrap();
        let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(document["bomFormat"], "CycloneDX");
        assert_eq!(document["specVersion"], "1.5");
        assert_eq!(document["metadata"]["component"]["name"], "cynic");
        let names = document["components"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["anyhow", "anyhow", "zerocopy"]);
        assert_eq!(
            document["components"][0]["bom-ref"],
            "pkg:cargo/anyhow@0.9.0"
        );
    }

    #[test]
    fn sbom_is_deterministic_without_timestamps() {
        let first = build_release_sbom_cyclonedx(FIXTURE_METADATA, "cynic", "0.1.0").unwrap();
        let second = build_release_sbom_cyclonedx(FIXTURE_METADATA, "cynic", "0.1.0").unwrap();
        assert_eq!(first, second);
        let text = String::from_utf8(first).unwrap();
        assert!(!text.contains("timestamp"));
        assert!(!text.contains("serialNumber"));
    }

    #[test]
    fn sbom_rejects_unparseable_and_empty_closures() {
        assert!(build_release_sbom_cyclonedx("not json", "cynic", "0.1.0").is_err());
        assert!(build_release_sbom_cyclonedx(r#"{"packages": []}"#, "cynic", "0.1.0").is_err());
        assert!(build_release_sbom_cyclonedx(FIXTURE_METADATA, "", "0.1.0").is_err());
        assert!(
            build_release_sbom_cyclonedx(
                r#"{"version": 2, "packages": [{"name": "a", "version": "1"}]}"#,
                "cynic",
                "0.1.0"
            )
            .is_err()
        );
    }
}
