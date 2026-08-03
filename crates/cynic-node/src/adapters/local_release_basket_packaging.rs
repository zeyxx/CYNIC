use std::{
    fs,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Component, Path, PathBuf},
    process::Command,
};

use cynic_protocol::{
    RUNTIME_SEED_BUNDLE_MARKER_FILE, SignedReleaseBasketPlan, canonical_json_bytes,
};
use sha2::{Digest, Sha256};

use crate::ports::release_publication_port::{
    PackagedReleaseBasket, ReleaseBasketPackagingPort, ReleasePublicationFuture,
};

#[derive(Debug, Clone)]
pub struct LocalReleaseBasketPackaging {
    work_root: PathBuf,
    max_bytes: u64,
}

impl LocalReleaseBasketPackaging {
    pub fn new(work_root: PathBuf, max_bytes: u64) -> Result<Self, String> {
        if !work_root.is_absolute() || max_bytes == 0 {
            return Err("release_packaging_boundary_invalid".to_string());
        }
        Ok(Self {
            work_root,
            max_bytes,
        })
    }
}

impl ReleaseBasketPackagingPort for LocalReleaseBasketPackaging {
    fn package<'a>(
        &'a self,
        plan: &'a SignedReleaseBasketPlan,
        workspace_root: &'a Path,
        workload_id: &'a str,
    ) -> ReleasePublicationFuture<'a, PackagedReleaseBasket> {
        let work_root = self.work_root.clone();
        let workspace_root = workspace_root.to_path_buf();
        let workload_id = workload_id.to_string();
        let plan = plan.clone();
        let max_bytes = self.max_bytes;
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                package_sync(&work_root, &workspace_root, &workload_id, &plan, max_bytes)
            })
            .await
            .map_err(|error| format!("release_packaging_task_failed:{error}"))?
        })
    }
}

fn package_sync(
    work_root: &Path,
    workspace_root: &Path,
    workload_id: &str,
    plan: &SignedReleaseBasketPlan,
    max_bytes: u64,
) -> Result<PackagedReleaseBasket, String> {
    if plan.status != "planned" || !plan.blockers.is_empty() {
        return Err("release_packaging_plan_not_admitted".to_string());
    }
    if !safe_identifier(workload_id) {
        return Err("release_packaging_workload_id_invalid".to_string());
    }
    ensure_absolute_directory_chain(work_root)?;
    let retention = tempfile::Builder::new()
        .prefix(&format!("{workload_id}-"))
        .tempdir_in(work_root)
        .map_err(|error| format!("release_packaging_workspace_create_failed:{error}"))?;
    let staging = retention.path().join("staging");
    fs::create_dir(&staging)
        .map_err(|error| format!("release_packaging_staging_create_failed:{error}"))?;
    for (name, expected_digest) in &plan.artifacts {
        let source = workspace_root.join("target/release").join(name);
        regular_file(&source, "release_binary")?;
        let (digest, _) = digest_file(&source, max_bytes)?;
        if &digest != expected_digest {
            return Err(format!("release_packaging_artifact_digest_mismatch:{name}"));
        }
        let destination = staging.join(name);
        fs::copy(&source, &destination)
            .map_err(|error| format!("release_packaging_artifact_copy_failed:{name}:{error}"))?;
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o755)).map_err(|error| {
            format!("release_packaging_artifact_permissions_failed:{name}:{error}")
        })?;
    }
    let marker_path = staging.join(RUNTIME_SEED_BUNDLE_MARKER_FILE);
    let marker_bytes = canonical_json_bytes(&plan.marker)
        .map_err(|error| format!("release_packaging_marker_serialization_failed:{error}"))?;
    fs::write(&marker_path, marker_bytes)
        .map_err(|error| format!("release_packaging_marker_write_failed:{error}"))?;
    fs::set_permissions(&marker_path, fs::Permissions::from_mode(0o644))
        .map_err(|error| format!("release_packaging_marker_permissions_failed:{error}"))?;
    let registry_path = workspace_root.join("infra/registry.json");
    let (registry_digest, _) = digest_file(&registry_path, max_bytes)?;
    if registry_digest != plan.marker.registry_digest {
        return Err("release_packaging_registry_digest_mismatch".to_string());
    }
    let registry_target = staging.join("registry.json");
    fs::copy(&registry_path, &registry_target)
        .map_err(|error| format!("release_packaging_registry_copy_failed:{error}"))?;
    fs::set_permissions(&registry_target, fs::Permissions::from_mode(0o644))
        .map_err(|error| format!("release_packaging_registry_permissions_failed:{error}"))?;

    let version = Command::new("tar")
        .arg("--version")
        .output()
        .map_err(|error| format!("release_packaging_tar_probe_failed:{error}"))?;
    if !version.status.success() || !String::from_utf8_lossy(&version.stdout).contains("GNU tar") {
        return Err("release_packaging_tar_capability_unknown".to_string());
    }
    let basket = retention
        .path()
        .join(format!("{}.tar", plan.marker.release_id));
    let output = Command::new("tar")
        .args([
            "--sort=name",
            "--mtime=@0",
            "--owner=0",
            "--group=0",
            "--numeric-owner",
            "--format=ustar",
            "--mode=u+rwX,go+rX,go-w",
            "--create",
            "--file",
        ])
        .arg(&basket)
        .arg("--directory")
        .arg(&staging)
        .args([
            RUNTIME_SEED_BUNDLE_MARKER_FILE,
            "cynic",
            "cynic-kernel",
            "cynic-mcp",
            "cynic-node",
            "registry.json",
        ])
        .output()
        .map_err(|error| format!("release_packaging_tar_spawn_failed:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "release_packaging_tar_failed:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let (digest, size_bytes) = digest_file(&basket, max_bytes)?;
    Ok(PackagedReleaseBasket {
        path: basket,
        digest,
        size_bytes,
        _retention: retention,
    })
}

fn ensure_absolute_directory_chain(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("release_packaging_work_root_not_absolute".to_string());
    }
    let mut current = PathBuf::from("/");
    for component in path.components() {
        match component {
            Component::RootDir => continue,
            Component::Normal(name) => current.push(name),
            _ => return Err("release_packaging_work_root_unsafe".to_string()),
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => return Err("release_packaging_work_root_unsafe".to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&current)
                .map_err(|error| format!("release_packaging_work_root_create_failed:{error}"))?,
            Err(error) => {
                return Err(format!("release_packaging_work_root_unobservable:{error}"));
            }
        }
    }
    Ok(())
}

fn regular_file(path: &Path, kind: &str) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("{kind}_unobservable:{error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() == 0 {
        return Err(format!("{kind}_unsafe"));
    }
    Ok(())
}

pub(crate) fn digest_file(path: &Path, max_bytes: u64) -> Result<(String, u64), String> {
    regular_file(path, "release_publication_file")?;
    let mut input = fs::File::open(path)
        .map_err(|error| format!("release_publication_file_open_failed:{error}"))?;
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| format!("release_publication_file_read_failed:{error}"))?;
        if read == 0 {
            break;
        }
        size = size
            .checked_add(read as u64)
            .ok_or_else(|| "release_publication_size_overflow".to_string())?;
        if size > max_bytes {
            return Err("release_publication_size_limit_exceeded".to_string());
        }
        hasher.update(&buffer[..read]);
    }
    Ok((format!("sha256:{}", hex::encode(hasher.finalize())), size))
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cynic_protocol::{
        BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA, BuildReleasePublicationIntent,
        SIGNED_RELEASE_BASKET_REQUEST_SCHEMA, SignedReleaseBasketRequest,
        plan_signed_release_basket,
    };

    use super::*;

    #[tokio::test]
    async fn packaging_is_deterministic_and_bound_to_the_proved_binaries() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("workspace");
        let release = workspace.join("target/release");
        fs::create_dir_all(&release).unwrap();
        let mut artifacts = BTreeMap::new();
        for name in cynic_protocol::runtime_release_artifact_names().unwrap() {
            let path = release.join(&name);
            fs::write(&path, format!("binary:{name}")).unwrap();
            artifacts.insert(name, digest_file(&path, 1024).unwrap().0);
        }
        let registry = workspace.join("infra/registry.json");
        fs::create_dir_all(registry.parent().unwrap()).unwrap();
        fs::write(&registry, b"{\"schema\":\"cynic.registry.v1\"}").unwrap();
        let registry_digest = digest_file(&registry, 1024).unwrap().0;
        let plan = plan_signed_release_basket(&SignedReleaseBasketRequest {
            schema: SIGNED_RELEASE_BASKET_REQUEST_SCHEMA.to_string(),
            node_id: "builder-a".to_string(),
            workload_id: "build-a".to_string(),
            source_commit: "a".repeat(40),
            build_proof_digest: format!("sha256:{}", "b".repeat(64)),
            registry_digest,
            publication: BuildReleasePublicationIntent {
                schema: BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA.to_string(),
                release_id: "release-a".to_string(),
                release_epoch: 1,
            },
            artifacts,
        });
        let adapter =
            LocalReleaseBasketPackaging::new(root.path().join("work"), 1024 * 1024).unwrap();

        let first = adapter.package(&plan, &workspace, "build-a").await.unwrap();
        let second = adapter.package(&plan, &workspace, "build-a").await.unwrap();

        assert_eq!(first.digest, second.digest);
        assert_eq!(first.size_bytes, second.size_bytes);
        let listing = Command::new("tar")
            .args(["-tf", first.path.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(listing.status.success());
        assert!(
            String::from_utf8_lossy(&listing.stdout)
                .lines()
                .any(|entry| entry == "registry.json")
        );
    }
}
