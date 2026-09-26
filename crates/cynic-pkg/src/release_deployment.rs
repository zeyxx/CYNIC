//! Sovereign release deployment and blue-green activation.
//!
//! Provides pure planning, atomic symlink-based blue-green activation,
//! and read-back proof generation for CYNIC binary releases.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

pub const RELEASE_DEPLOY_REPORT_SCHEMA: &str = "cynic.pkg.release_deploy.report.v1";

/// Specification of a binary to be deployed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployBinarySpec {
    pub name: String,
    pub source_path: PathBuf,
}

/// Specification for a release deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseDeploySpec {
    pub version: String,
    pub state_root: PathBuf,
    pub binaries: Vec<DeployBinarySpec>,
}

/// Planned actions for the release deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseDeployPlan {
    pub release_tag: String,
    pub release_dir: PathBuf,
    pub current_link: PathBuf,
    pub bin_dir: PathBuf,
    pub binary_targets: Vec<(String, PathBuf)>,
}

/// Read-back proof of an executed release deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseDeployReport {
    pub schema: String,
    pub status: String,
    pub release_tag: String,
    pub release_dir: PathBuf,
    pub active_current_target: PathBuf,
    pub deployed_binaries: Vec<String>,
    pub deployed_at_unix_s: u64,
}

/// Purely plan a release deployment layout without filesystem mutations.
pub fn plan_release_deployment(spec: &ReleaseDeploySpec) -> Result<ReleaseDeployPlan, String> {
    if spec.version.trim().is_empty() {
        return Err("release deployment: version cannot be empty".to_string());
    }

    let release_tag = if spec.version.starts_with('v') {
        spec.version.clone()
    } else {
        format!("v{}", spec.version)
    };

    let release_dir = spec.state_root.join("releases").join(&release_tag);
    let current_link = spec.state_root.join("current");
    let bin_dir = spec.state_root.join("bin");

    let mut binary_targets = Vec::new();
    for bin in &spec.binaries {
        if bin.name.trim().is_empty() {
            return Err("release deployment: binary name cannot be empty".to_string());
        }
        let target = release_dir.join(&bin.name);
        binary_targets.push((bin.name.clone(), target));
    }

    Ok(ReleaseDeployPlan {
        release_tag,
        release_dir,
        current_link,
        bin_dir,
        binary_targets,
    })
}

/// Port for release deployment filesystem actuation.
pub trait ReleaseDeployExecutorPort {
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;
    fn copy_file(&self, from: &Path, to: &Path) -> std::io::Result<u64>;
    fn atomic_symlink(&self, target: &Path, link_path: &Path) -> std::io::Result<()>;
    fn read_link(&self, path: &Path) -> std::io::Result<PathBuf>;
}

/// Default local executor implementing blue-green deployment via POSIX rename.
pub struct LocalReleaseDeployExecutor;

impl ReleaseDeployExecutorPort for LocalReleaseDeployExecutor {
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        fs::create_dir_all(path)
    }

    fn copy_file(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        fs::copy(from, to)
    }

    fn atomic_symlink(&self, target: &Path, link_path: &Path) -> std::io::Result<()> {
        let parent = link_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_link = parent.join(format!(
            ".tmp-symlink-{}-{}",
            std::process::id(),
            link_path.file_name().and_then(|n| n.to_str()).unwrap_or("link")
        ));

        let _ = fs::remove_file(&temp_link);
        symlink(target, &temp_link)?;
        fs::rename(&temp_link, link_path)?;
        Ok(())
    }

    fn read_link(&self, path: &Path) -> std::io::Result<PathBuf> {
        fs::read_link(path)
    }
}

/// Apply a planned release deployment using the injected executor port.
pub fn apply_release_deployment<E: ReleaseDeployExecutorPort>(
    spec: &ReleaseDeploySpec,
    plan: &ReleaseDeployPlan,
    executor: &E,
    now_unix_s: u64,
) -> Result<ReleaseDeployReport, String> {
    // 1. Create target release staging directory
    executor
        .create_dir_all(&plan.release_dir)
        .map_err(|e| format!("create release dir {}: {e}", plan.release_dir.display()))?;

    // 2. Stage binaries into the release directory
    let mut deployed_binaries = Vec::new();
    for (bin_spec, (_, target_path)) in spec.binaries.iter().zip(plan.binary_targets.iter()) {
        if !bin_spec.source_path.exists() {
            return Err(format!(
                "source binary not found: {}",
                bin_spec.source_path.display()
            ));
        }
        executor
            .copy_file(&bin_spec.source_path, target_path)
            .map_err(|e| {
                format!(
                    "copy binary from {} to {}: {e}",
                    bin_spec.source_path.display(),
                    target_path.display()
                )
            })?;
        deployed_binaries.push(bin_spec.name.clone());
    }

    // 3. Atomically switch /current symlink to the new release
    executor
        .atomic_symlink(&plan.release_dir, &plan.current_link)
        .map_err(|e| {
            format!(
                "atomic symlink {} -> {}: {e}",
                plan.current_link.display(),
                plan.release_dir.display()
            )
        })?;

    // 4. Update /bin symlinks pointing through /current/<binary>
    executor
        .create_dir_all(&plan.bin_dir)
        .map_err(|e| format!("create bin dir {}: {e}", plan.bin_dir.display()))?;

    for bin_name in &deployed_binaries {
        let current_bin_target = plan.current_link.join(bin_name);
        let bin_link = plan.bin_dir.join(bin_name);
        executor
            .atomic_symlink(&current_bin_target, &bin_link)
            .map_err(|e| {
                format!(
                    "atomic symlink {} -> {}: {e}",
                    bin_link.display(),
                    current_bin_target.display()
                )
            })?;
    }

    // 5. Read back proof verification
    let active_current_target = executor
        .read_link(&plan.current_link)
        .map_err(|e| format!("read back symlink {}: {e}", plan.current_link.display()))?;

    if active_current_target != plan.release_dir {
        return Err(format!(
            "read back mismatch: expected current -> {} but got {}",
            plan.release_dir.display(),
            active_current_target.display()
        ));
    }

    Ok(ReleaseDeployReport {
        schema: RELEASE_DEPLOY_REPORT_SCHEMA.to_string(),
        status: "converged".to_string(),
        release_tag: plan.release_tag.clone(),
        release_dir: plan.release_dir.clone(),
        active_current_target,
        deployed_binaries,
        deployed_at_unix_s: now_unix_s,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn plans_release_deployment_correctly() {
        let spec = ReleaseDeploySpec {
            version: "0.8.0".to_string(),
            state_root: PathBuf::from("/var/lib/cynic"),
            binaries: vec![
                DeployBinarySpec {
                    name: "cynic".to_string(),
                    source_path: PathBuf::from("target/release/cynic"),
                },
                DeployBinarySpec {
                    name: "cynic-node".to_string(),
                    source_path: PathBuf::from("target/release/cynic-node"),
                },
            ],
        };

        let plan = plan_release_deployment(&spec).expect("plan");
        assert_eq!(plan.release_tag, "v0.8.0");
        assert_eq!(
            plan.release_dir,
            PathBuf::from("/var/lib/cynic/releases/v0.8.0")
        );
        assert_eq!(plan.current_link, PathBuf::from("/var/lib/cynic/current"));
        assert_eq!(plan.bin_dir, PathBuf::from("/var/lib/cynic/bin"));
        assert_eq!(plan.binary_targets.len(), 2);
    }

    #[test]
    fn applies_deployment_with_atomic_symlinks_and_read_back() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();

        let source_bin = root.join("src-cynic");
        fs::write(&source_bin, b"binary-content-v0.8.0").expect("write fake bin");

        let spec = ReleaseDeploySpec {
            version: "0.8.0".to_string(),
            state_root: root.clone(),
            binaries: vec![DeployBinarySpec {
                name: "cynic".to_string(),
                source_path: source_bin,
            }],
        };

        let plan = plan_release_deployment(&spec).expect("plan");
        let executor = LocalReleaseDeployExecutor;
        let report = apply_release_deployment(&spec, &plan, &executor, 1790000000).expect("apply");

        assert_eq!(report.status, "converged");
        assert_eq!(report.release_tag, "v0.8.0");
        assert_eq!(report.active_current_target, plan.release_dir);

        let bin_on_disk = plan.release_dir.join("cynic");
        assert!(bin_on_disk.exists());
        assert_eq!(
            fs::read(&bin_on_disk).unwrap(),
            b"binary-content-v0.8.0"
        );

        let bin_link = plan.bin_dir.join("cynic");
        assert!(bin_link.exists());
    }
}
