use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::install_contract::{MODE_0440, MODE_0644};

use super::types::PrivilegeMembraneBundleManifest;

fn prepare_bundle_target(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Stage the privilege membrane: render the catalog-derived sudoers file
/// for the single `cynic` binary. No helper binary is staged or installed;
/// the executor is the release binary the basket already deploys.
pub fn write_privilege_membrane_bundle(
    output_dir: impl AsRef<Path>,
) -> Result<PrivilegeMembraneBundleManifest, String> {
    use cynic_protocol::privilege::CapabilityCatalog;

    const SUBJECT: &str = "cynic";
    const BINARY: &str = "/usr/local/bin/cynic";

    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).map_err(|error| format!("membrane bundle dir: {error}"))?;
    let catalog_path = crate::privilege::capability_catalog::catalog_path_from_env();
    let raw = fs::read_to_string(&catalog_path).map_err(|error| {
        format!(
            "membrane catalog unreadable {}: {error}",
            catalog_path.display()
        )
    })?;
    let catalog: CapabilityCatalog =
        serde_json::from_str(&raw).map_err(|error| format!("membrane catalog invalid: {error}"))?;
    if catalog.capabilities.is_empty() {
        return Err(
            "empty_membrane: the catalog declares no capability, refusing to stage".to_string(),
        );
    }
    let content = catalog.sudoers_file(SUBJECT, BINARY);
    let sudoers_target = output_dir.join("privilege.sudoers");
    prepare_bundle_target(&sudoers_target).map_err(|error| format!("membrane stage: {error}"))?;
    fs::write(&sudoers_target, &content).map_err(|error| format!("membrane write: {error}"))?;
    fs::set_permissions(&sudoers_target, fs::Permissions::from_mode(MODE_0440.unix))
        .map_err(|error| format!("membrane mode: {error}"))?;

    let capability_ids: Vec<String> = catalog
        .capabilities
        .iter()
        .filter(|capability| capability.requires_root)
        .map(|capability| capability.id.clone())
        .collect();
    let rule_count = capability_ids.len();
    let bundle_dir = output_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("privilege-bundle")
        .to_string();
    let manifest = PrivilegeMembraneBundleManifest {
        schema: "cynic.privilege_bundle.v1",
        mutates: false,
        binary: BINARY.to_string(),
        subject: SUBJECT.to_string(),
        sudoers_target: "/etc/sudoers.d/cynic".to_string(),
        capability_ids,
        rule_count,
        bundle_dir,
        bundle_files: BTreeMap::from([
            ("sudoers", "privilege.sudoers".to_string()),
            ("manifest", "manifest.json".to_string()),
        ]),
    };
    let manifest_target = output_dir.join("manifest.json");
    let rendered = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("membrane manifest: {error}"))?;
    fs::write(&manifest_target, format!("{rendered}\n"))
        .map_err(|error| format!("membrane manifest write: {error}"))?;
    fs::set_permissions(&manifest_target, fs::Permissions::from_mode(MODE_0644.unix))
        .map_err(|error| format!("membrane manifest mode: {error}"))?;
    Ok(manifest)
}
