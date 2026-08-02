use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::types::{StoragePrimaryBundleReport, StoragePrimaryPlanReport};

pub fn write_activation_bundle(
    plan: &StoragePrimaryPlanReport,
    output_dir: &Path,
) -> Result<StoragePrimaryBundleReport, String> {
    let Some(desired) = plan.desired.as_ref() else {
        return Ok(blocked(output_dir, plan.blockers.clone()));
    };
    let schema_source = Path::new(&desired.schema_path);
    if !schema_source.is_file() {
        return Ok(blocked(
            output_dir,
            vec!["storage_schema_source_not_observed".to_string()],
        ));
    }
    fs::create_dir_all(output_dir)
        .map_err(|error| format!("storage bundle directory creation failed: {error}"))?;
    let schema =
        fs::read(schema_source).map_err(|error| format!("storage schema read failed: {error}"))?;
    let storage_toml = format!(
        "# Generated from infra/registry.json; contains no secret.\n[storage]\nurl = \"{}\"\nallow_loopback = false\nnamespace = \"cynic\"\ndatabase = \"main\"\nusername = \"{}\"\nsecret_file = \"/etc/cynic/secrets/storage-runtime-password\"\n",
        desired.endpoint, desired.runtime_identity
    );
    let files = [
        ("schema.surql", schema),
        ("storage.toml", storage_toml.into_bytes()),
        (
            "plan.json",
            serde_json::to_vec_pretty(plan)
                .map_err(|error| format!("storage plan serialization failed: {error}"))?,
        ),
    ];
    let mut paths = Vec::new();
    let mut digests = Vec::new();
    for (name, bytes) in files {
        let path = output_dir.join(name);
        fs::write(&path, &bytes)
            .map_err(|error| format!("storage bundle write failed for {name}: {error}"))?;
        paths.push(path.display().to_string());
        digests.push(format!(
            "sha256:{}  {name}",
            hex::encode(Sha256::digest(&bytes))
        ));
    }
    let manifest_path = output_dir.join("manifest.json");
    let manifest = serde_json::json!({
        "schema": "cynic.storage_primary.bundle_manifest.v1",
        "authority": "infra/registry.json",
        "desired": desired,
        "digests": digests,
        "contains_secret": false,
        "mutation_requires": ["explicit_apply", "SchemaMigrator_or_node_bootstrap_authority"]
    });
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest)
            .map_err(|error| format!("storage manifest serialization failed: {error}"))?,
    )
    .map_err(|error| format!("storage manifest write failed: {error}"))?;
    paths.push(manifest_path.display().to_string());
    Ok(StoragePrimaryBundleReport {
        schema: "cynic.storage_primary.bundle.v1",
        status: "prepared",
        mutates_runtime: false,
        output_dir: output_dir.display().to_string(),
        files: paths,
        digests,
        blockers: Vec::new(),
        next_action: "submit the bundle to node bootstrap or SchemaMigrator with explicit apply"
            .to_string(),
    })
}

fn blocked(output_dir: &Path, blockers: Vec<String>) -> StoragePrimaryBundleReport {
    StoragePrimaryBundleReport {
        schema: "cynic.storage_primary.bundle.v1",
        status: "blocked",
        mutates_runtime: false,
        output_dir: output_dir.display().to_string(),
        files: Vec::new(),
        digests: Vec::new(),
        blockers,
        next_action: "repair the storage-primary plan before building a bundle".to_string(),
    }
}
