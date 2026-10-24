//! Host adapter providing concrete gate proof records for release evaluation.

use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

use crate::types::GateProofRecord;

/// Collect raw host gate proofs (git cleanliness, secret scan).
///
/// Compilation is NOT checked locally: the builder is the declared SSOT
/// for build validation (see `build_proof` gate). Running `cargo check`
/// on the operator host (faulty RAM, no network) was redundant with the
/// builder proof and produced false negatives in offline mode.
/// `git_clean` runs FIRST so the gate judges the tree as committed.
pub fn collect_host_gate_proofs(root: &Path) -> Vec<GateProofRecord> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut records = Vec::new();

    // 1. git clean — the tree must be pristine BEFORE any tool runs.
    let git_passed = check_git_status_clean(root);
    records.push(GateProofRecord {
        name: "git_clean".to_string(),
        passed: git_passed,
        detail: if git_passed {
            "worktree clean".to_string()
        } else {
            "worktree has uncommitted or dirty files".to_string()
        },
        observed_at_unix_s: now,
    });

    // 3. secret lint
    let lint_passed = run_secret_lint(root);
    records.push(GateProofRecord {
        name: "secret_lint".to_string(),
        passed: lint_passed,
        detail: if lint_passed {
            "no secrets found in source tree".to_string()
        } else {
            "secret lint detected suspicious patterns".to_string()
        },
        observed_at_unix_s: now,
    });

    // 3. git clean
    let git_passed = check_git_status_clean(root);
    records.push(GateProofRecord {
        name: "git_clean".to_string(),
        passed: git_passed,
        detail: if git_passed {
            "worktree clean".to_string()
        } else {
            "worktree has uncommitted or dirty files".to_string()
        },
        observed_at_unix_s: now,
    });

    // 4. version monotonic — the release version must be strictly greater
    // than the newest tag already published.
    //
    // `contracts/release-versioning.json`:
    //   "git_tag_authority: Git tag is the source of truth for a release
    //    version. No release exists without a tag."
    //   rule `version_scheme_semver`: the version follows SemVer.
    //
    // The gate was missing, so `Cargo.toml` could carry 0.8.0 forever and
    // every release gate would still pass — a release that overwrites a
    // published tag, which the contract forbids.
    let (version_passed, version_detail) = check_version_is_monotonic(root);
    records.push(GateProofRecord {
        name: "version_monotonic".to_string(),
        passed: version_passed,
        detail: version_detail,
        observed_at_unix_s: now,
    });

    records
}

/// Reads the workspace version and compares it to the newest published tag.
///
/// Returns `(passed, detail)`. The detail always names the version and
/// the tag it was compared against, so a refusal says what to change.
fn check_version_is_monotonic(root: &Path) -> (bool, String) {
    let declared = match read_workspace_version(root) {
        Some(version) => version,
        None => {
            return (
                false,
                "no workspace version found in Cargo.toml [workspace.package]".to_string(),
            )
        }
    };

    let tags = list_release_tags(root);
    let newest = tags.iter().find_map(|tag| {
        crate::types::ReleaseVersion::parse_tag(tag).map(|version| (tag.clone(), version))
    });

    let Some((tag, prior)) = newest else {
        // No published tag yet: the first release is monotonic by
        // definition.
        return (
            true,
            format!("version {declared} — no release tag published yet"),
        );
    };

    let current = match crate::types::ReleaseVersion::parse_tag(&declared) {
        Some(version) => version,
        None => {
            return (
                false,
                format!("version {declared} does not parse as SemVer"),
            )
        }
    };

    if current.is_strictly_monotonic_after(&prior) {
        (
            true,
            format!("version {declared} is strictly after tag {tag}"),
        )
    } else {
        (
            false,
            format!(
                "version {declared} is not strictly after the published tag {tag} — \
                 a release must not reuse or precede a published version"
            ),
        )
    }
}

/// Reads the `[workspace.package] version` from the workspace manifest.
fn read_workspace_version(root: &Path) -> Option<String> {
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let section = manifest.split("[workspace.package]").nth(1)?;
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            // The next section starts; the version was not in it.
            break;
        }
        if let Some(rest) = line.strip_prefix("version") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let value = rest.trim().trim_matches('"').trim_matches('\'');
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// Lists the published release tags, newest first.
///
/// Tags are compared as SemVer rather than as text: `v0.9.0` is newer than
/// `v0.10.0` only under SemVer, and a text sort would get that backwards.
fn list_release_tags(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["tag", "--list", "v*"])
        .current_dir(root)
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut tags: Vec<(u64, u64, u64, String)> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|tag| {
            crate::types::ReleaseVersion::parse_tag(tag).map(|version| {
                (
                    version.major,
                    version.minor,
                    version.patch,
                    tag.to_string(),
                )
            })
        })
        .collect();
    tags.sort_by(|a, b| (b.0, b.1, b.2).cmp(&(a.0, a.1, a.2)));
    tags.into_iter().map(|(_, _, _, tag)| tag).collect()
}

fn run_secret_lint(root: &Path) -> bool {
    let observation = super::secret_lint::scan_for_secrets(root);
    !observation.has_findings()
}

fn check_git_status_clean(root: &Path) -> bool {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output();
    match output {
        Ok(o) => o.status.success() && o.stdout.is_empty(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .expect("git runs")
                .status;
            assert!(status.success(), "git {args:?} failed");
        };
        run(&["init"]);
        run(&["config", "user.email", "gate@test.local"]);
        run(&["config", "user.name", "gate-test"]);
        std::fs::write(dir.path().join("file.txt"), "committed").expect("write");
        run(&["add", "."]);
        run(&["commit", "-qm", "init"]);
        dir
    }

    #[test]
    fn clean_tree_passes_git_clean() {
        let dir = init_repo();
        assert!(check_git_status_clean(dir.path()));
    }

    #[test]
    fn dirty_tree_fails_git_clean() {
        let dir = init_repo();
        std::fs::write(dir.path().join("file.txt"), "dirty").expect("dirty write");
        assert!(!check_git_status_clean(dir.path()));
    }

    #[test]
    fn untracked_file_fails_git_clean() {
        let dir = init_repo();
        std::fs::write(dir.path().join("untracked.txt"), "new").expect("write");
        assert!(!check_git_status_clean(dir.path()));
    }

    /// A repository whose manifest declares `version` and which carries
    /// the given release tags.
    fn init_versioned_repo(version: &str, tags: &[&str]) -> tempfile::TempDir {
        let dir = init_repo();
        let manifest = format!(
            "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"{version}\"\n"
        );
        std::fs::write(dir.path().join("Cargo.toml"), manifest).expect("write manifest");
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .expect("git runs")
                .status;
            assert!(status.success(), "git {args:?} failed");
        };
        run(&["add", "."]);
        run(&["commit", "-qm", "manifest"]);
        for tag in tags {
            run(&["tag", tag]);
        }
        dir
    }

    #[test]
    fn a_version_after_the_published_tag_passes() {
        let dir = init_versioned_repo("0.9.0", &["v0.8.0"]);
        let (passed, detail) = check_version_is_monotonic(dir.path());
        assert!(passed, "expected pass, got: {detail}");
        assert!(detail.contains("0.9.0") && detail.contains("v0.8.0"), "{detail}");
    }

    #[test]
    fn the_published_version_itself_is_refused() {
        // The real case: Cargo.toml still carried 0.8.0 while v0.8.0 was
        // already published, so a release would have overwritten it.
        let dir = init_versioned_repo("0.8.0", &["v0.8.0"]);
        let (passed, detail) = check_version_is_monotonic(dir.path());
        assert!(!passed, "expected refusal, got: {detail}");
        assert!(detail.contains("v0.8.0"), "{detail}");
    }

    #[test]
    fn a_version_before_the_published_tag_is_refused() {
        let dir = init_versioned_repo("0.7.9", &["v0.8.0"]);
        let (passed, detail) = check_version_is_monotonic(dir.path());
        assert!(!passed, "expected refusal, got: {detail}");
    }

    #[test]
    fn the_newest_tag_is_the_one_compared_against() {
        // SemVer order, not text order: v0.9.0 is older than v0.10.0.
        let dir = init_versioned_repo("0.10.1", &["v0.9.0", "v0.10.0"]);
        let (passed, detail) = check_version_is_monotonic(dir.path());
        assert!(passed, "expected pass, got: {detail}");
        assert!(detail.contains("v0.10.0"), "{detail}");
    }

    #[test]
    fn a_first_release_with_no_tag_passes() {
        let dir = init_versioned_repo("0.1.0", &[]);
        let (passed, detail) = check_version_is_monotonic(dir.path());
        assert!(passed, "expected pass, got: {detail}");
    }

    #[test]
    fn a_manifest_without_a_version_is_refused() {
        let dir = init_repo();
        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = []\n")
            .expect("write manifest");
        let (passed, detail) = check_version_is_monotonic(dir.path());
        assert!(!passed, "expected refusal, got: {detail}");
    }

    #[test]
    fn a_version_that_is_not_semver_is_refused() {
        let dir = init_versioned_repo("0.8", &["v0.7.0"]);
        let (passed, detail) = check_version_is_monotonic(dir.path());
        assert!(!passed, "expected refusal, got: {detail}");
    }

    #[test]
    fn the_gate_is_recorded_in_the_collected_proofs() {
        let dir = init_versioned_repo("0.8.0", &["v0.8.0"]);
        let records = collect_host_gate_proofs(dir.path());
        let version_gate = records
            .iter()
            .find(|record| record.name == "version_monotonic")
            .expect("the version gate is recorded");
        assert!(!version_gate.passed, "the gate must refuse a reused version");
    }
}
