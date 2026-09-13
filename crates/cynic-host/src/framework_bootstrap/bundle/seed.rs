use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::types::{FrameworkSeedArtifact, FrameworkSeedEnrollment, FrameworkSeedManifest};

pub(super) fn write_seed_checksums(output_dir: &Path, files: &[String]) -> Result<(), String> {
    let mut entries = files
        .iter()
        .map(|relative| {
            let artifact = seed_artifact(output_dir, relative)?;
            Ok(format!(
                "{}  {}",
                artifact.sha256.trim_start_matches("sha256:"),
                artifact.path
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    entries.sort();
    fs::write(
        output_dir.join("seed-artifacts.sha256"),
        format!("{}\n", entries.join("\n")),
    )
    .map_err(|error| format!("failed to write seed checksum projection: {error}"))
}

pub(super) fn write_seed_manifest(
    output_dir: &Path,
    files: &[String],
    role: &str,
) -> Result<FrameworkSeedManifest, String> {
    let mut artifacts = files
        .iter()
        .map(|relative| seed_artifact(output_dir, relative))
        .collect::<Result<Vec<_>, _>>()?;
    artifacts.sort_by(|left, right| left.path.cmp(&right.path));
    let identity = serde_json::to_vec(&artifacts)
        .map_err(|error| format!("failed to serialize seed artifact identity: {error}"))?;
    let seed_id = format!("sha256:{}", hex::encode(Sha256::digest(identity)));
    let manifest = FrameworkSeedManifest {
        schema: "cynic.framework_seed.v2",
        seed_id,
        status: "seed_only",
        artifacts,
        enrollment: FrameworkSeedEnrollment {
            required: true,
            target_role: role.to_string(),
            identity_class: "MachineEnrollment",
            token_material: "injected_at_apply_never_embedded",
            admission_status: "external_authority_required",
        },
        proof_contract: vec![
            "host_identity_observed",
            "host_bootstrap_target_satisfied",
            "runtime_principal_active",
            "bridge_status_observed",
        ],
        rollback_intent: "reject enrollment and remove installed seed projection before activating a runtime role",
    };
    fs::write(
        output_dir.join("seed-manifest.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&manifest)
                .map_err(|error| format!("failed to serialize seed manifest: {error}"))?
        ),
    )
    .map_err(|error| format!("failed to write seed manifest: {error}"))?;
    Ok(manifest)
}

fn seed_artifact(output_dir: &Path, relative: &str) -> Result<FrameworkSeedArtifact, String> {
    let path = output_dir.join(relative);
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("failed to stat seed artifact {}: {error}", path.display()))?;
    let content = fs::read(&path)
        .map_err(|error| format!("failed to read seed artifact {}: {error}", path.display()))?;
    Ok(FrameworkSeedArtifact {
        path: relative.to_string(),
        sha256: format!("sha256:{}", hex::encode(Sha256::digest(content))),
        mode: format!("{:04o}", metadata.permissions().mode() & 0o7777),
    })
}
