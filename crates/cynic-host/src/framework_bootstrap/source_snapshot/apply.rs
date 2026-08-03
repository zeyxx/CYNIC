use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::update_lifecycle::ArgvRunner;

use super::plan::plan_source_snapshot;
use super::types::{SourceSnapshotPlan, SourceSnapshotReport};

pub fn create_source_snapshot(
    repository_root: &str,
    source_commit: &str,
    output_dir: &str,
    yes: bool,
    runner: &dyn ArgvRunner,
) -> SourceSnapshotReport {
    let plan = plan_source_snapshot(repository_root, source_commit, output_dir, runner);
    if plan.status != "planned" || !yes {
        let mut blockers = plan.blockers.clone();
        if !yes {
            blockers.push("explicit_yes_required".to_string());
        }
        return report(
            &plan,
            false,
            yes,
            None,
            None,
            Vec::new(),
            blockers,
            "blocked",
        );
    }
    let output = Path::new(output_dir);
    let created_output = match prepare_output_directory(output) {
        Ok(created) => created,
        Err(error) => {
            return report(
                &plan,
                false,
                yes,
                None,
                None,
                Vec::new(),
                vec![error],
                "blocked",
            );
        }
    };
    let staging = output.join(format!(".cynic-source-snapshot-{}", std::process::id()));
    if staging.exists() {
        rollback_staging(&staging, output, created_output);
        return report(
            &plan,
            false,
            yes,
            None,
            None,
            Vec::new(),
            vec!["source_snapshot_staging_exists".to_string()],
            "blocked",
        );
    }
    let mut executed = Vec::new();
    let temporary_ref = temporary_ref(source_commit);
    let create_ref_argv = vec![
        "git".to_string(),
        "-C".to_string(),
        repository_root.to_string(),
        "update-ref".to_string(),
        temporary_ref.clone(),
        source_commit.to_string(),
        "0".repeat(source_commit.len()),
    ];
    let create_ref = runner.run(
        &create_ref_argv
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    executed.push(create_ref_argv);
    if !create_ref.success() {
        rollback_staging(&staging, output, created_output);
        return report(
            &plan,
            true,
            yes,
            None,
            None,
            executed,
            vec!["source_snapshot_temporary_ref_create_failed".to_string()],
            "blocked",
        );
    }
    let create_argv = vec![
        "git".to_string(),
        "-C".to_string(),
        repository_root.to_string(),
        "bundle".to_string(),
        "create".to_string(),
        staging.display().to_string(),
        temporary_ref.clone(),
    ];
    let create = runner.run(&create_argv.iter().map(String::as_str).collect::<Vec<_>>());
    executed.push(create_argv);
    let cleanup_ref = delete_temporary_ref(
        repository_root,
        source_commit,
        &temporary_ref,
        runner,
        &mut executed,
    );
    if !create.success() || cleanup_ref.is_err() {
        rollback_staging(&staging, output, created_output);
        let mut blockers = Vec::new();
        if !create.success() {
            blockers.push("source_snapshot_bundle_create_failed".to_string());
        }
        if let Err(error) = cleanup_ref {
            blockers.push(error);
        }
        return report(&plan, true, yes, None, None, executed, blockers, "unknown");
    }
    let digest = match sha256_file(&staging) {
        Ok(digest) => digest,
        Err(error) => {
            rollback_staging(&staging, output, created_output);
            return report(
                &plan,
                true,
                yes,
                None,
                None,
                executed,
                vec![error],
                "unknown",
            );
        }
    };
    if let Err(error) = fs::set_permissions(&staging, fs::Permissions::from_mode(0o440)) {
        rollback_staging(&staging, output, created_output);
        return report(
            &plan,
            true,
            yes,
            Some(digest),
            None,
            executed,
            vec![format!("source_snapshot_mode_failed:{error}")],
            "unknown",
        );
    }
    let final_path = output.join(digest.trim_start_matches("sha256:"));
    let created_final = match promote_snapshot(&staging, &final_path, &digest) {
        Ok(created) => created,
        Err(error) => {
            rollback_staging(&staging, output, created_output);
            return report(
                &plan,
                true,
                yes,
                Some(digest),
                None,
                executed,
                vec![error],
                "unknown",
            );
        }
    };
    let verify_argv = vec![
        "git".to_string(),
        "bundle".to_string(),
        "verify".to_string(),
        final_path.display().to_string(),
    ];
    let verify = runner.run(&verify_argv.iter().map(String::as_str).collect::<Vec<_>>());
    executed.push(verify_argv);
    let heads_argv = vec![
        "git".to_string(),
        "bundle".to_string(),
        "list-heads".to_string(),
        final_path.display().to_string(),
    ];
    let heads = runner.run(&heads_argv.iter().map(String::as_str).collect::<Vec<_>>());
    executed.push(heads_argv);
    let advertised_commit = heads
        .stdout
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .find(|commit| *commit == source_commit)
        .map(str::to_string);
    let import_read_back =
        prove_snapshot_import(&final_path, source_commit, output, runner, &mut executed);
    let commit_read_back = import_read_back.as_ref().ok().cloned();
    let digest_read_back = sha256_file(&final_path).ok();
    let proved = verify.success()
        && heads.success()
        && advertised_commit.as_deref() == Some(source_commit)
        && commit_read_back.as_deref() == Some(source_commit)
        && digest_read_back.as_deref() == Some(digest.as_str());
    if !proved && created_final {
        rollback_published(&final_path, output, created_output);
    }
    report(
        &plan,
        true,
        yes,
        Some(digest),
        commit_read_back,
        executed,
        if proved {
            Vec::new()
        } else if let Err(error) = import_read_back {
            vec![error]
        } else {
            vec!["source_snapshot_read_back_failed".to_string()]
        },
        if proved { "proved" } else { "unknown" },
    )
}

pub(super) fn prove_snapshot_import(
    snapshot_path: &Path,
    source_commit: &str,
    output_dir: &Path,
    runner: &dyn ArgvRunner,
    executed: &mut Vec<Vec<String>>,
) -> Result<String, String> {
    let probe = output_dir.join(format!(
        ".cynic-source-snapshot-import-{}",
        std::process::id()
    ));
    fs::create_dir(&probe)
        .map_err(|error| format!("source_snapshot_consumer_probe_create_failed:{error}"))?;

    let proof_ref = "refs/cynic/source-snapshot-import-proof";
    let init_argv = vec![
        "git".to_string(),
        "-C".to_string(),
        probe.display().to_string(),
        "init".to_string(),
        "--bare".to_string(),
    ];
    let init = runner.run(&init_argv.iter().map(String::as_str).collect::<Vec<_>>());
    executed.push(init_argv);

    let mut result = if !init.success() {
        Err("source_snapshot_consumer_probe_init_failed".to_string())
    } else {
        let fetch_argv = vec![
            "git".to_string(),
            "-C".to_string(),
            probe.display().to_string(),
            "fetch".to_string(),
            "--no-tags".to_string(),
            snapshot_path.display().to_string(),
            format!("{source_commit}:{proof_ref}"),
        ];
        let fetch = runner.run(&fetch_argv.iter().map(String::as_str).collect::<Vec<_>>());
        executed.push(fetch_argv);
        if !fetch.success() {
            Err("source_snapshot_consumer_import_failed".to_string())
        } else {
            let read_back_argv = vec![
                "git".to_string(),
                "-C".to_string(),
                probe.display().to_string(),
                "rev-parse".to_string(),
                format!("{proof_ref}^{{commit}}"),
            ];
            let read_back = runner.run(
                &read_back_argv
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            );
            executed.push(read_back_argv);
            let observed = read_back.stdout.trim();
            if read_back.success() && observed == source_commit {
                Ok(observed.to_string())
            } else {
                Err("source_snapshot_consumer_commit_read_back_failed".to_string())
            }
        }
    };

    if let Err(error) = fs::remove_dir_all(&probe) {
        result = Err(format!(
            "source_snapshot_consumer_probe_cleanup_failed:{error}"
        ));
    }
    result
}

fn temporary_ref(source_commit: &str) -> String {
    format!(
        "refs/cynic/source-snapshots/{}-{source_commit}",
        std::process::id()
    )
}

fn delete_temporary_ref(
    repository_root: &str,
    source_commit: &str,
    temporary_ref: &str,
    runner: &dyn ArgvRunner,
    executed: &mut Vec<Vec<String>>,
) -> Result<(), String> {
    let delete_argv = vec![
        "git".to_string(),
        "-C".to_string(),
        repository_root.to_string(),
        "update-ref".to_string(),
        "-d".to_string(),
        temporary_ref.to_string(),
        source_commit.to_string(),
    ];
    let delete = runner.run(&delete_argv.iter().map(String::as_str).collect::<Vec<_>>());
    executed.push(delete_argv);
    if !delete.success() {
        return Err("source_snapshot_temporary_ref_delete_failed".to_string());
    }
    let read_back_argv = vec![
        "git".to_string(),
        "-C".to_string(),
        repository_root.to_string(),
        "show-ref".to_string(),
        "--verify".to_string(),
        "--quiet".to_string(),
        temporary_ref.to_string(),
    ];
    let read_back = runner.run(
        &read_back_argv
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    executed.push(read_back_argv);
    if read_back.exit_code == Some(1) && read_back.spawn_error.is_none() {
        Ok(())
    } else {
        Err("source_snapshot_temporary_ref_delete_read_back_failed".to_string())
    }
}

fn prepare_output_directory(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(false),
        Ok(_) => Err("source_snapshot_output_directory_unsafe".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .ok_or_else(|| "source_snapshot_output_parent_missing".to_string())?;
            let parent_metadata = fs::symlink_metadata(parent).map_err(|failure| {
                format!("source_snapshot_output_parent_unavailable:{failure}")
            })?;
            if !parent_metadata.is_dir() || parent_metadata.file_type().is_symlink() {
                return Err("source_snapshot_output_parent_unsafe".to_string());
            }
            fs::create_dir(path)
                .map_err(|failure| format!("source_snapshot_output_create_failed:{failure}"))?;
            Ok(true)
        }
        Err(error) => Err(format!("source_snapshot_output_unobservable:{error}")),
    }
}

fn promote_snapshot(staging: &Path, final_path: &Path, expected: &str) -> Result<bool, String> {
    if final_path.exists() {
        if sha256_file(final_path)? == expected {
            fs::remove_file(staging)
                .map_err(|error| format!("source_snapshot_staging_cleanup_failed:{error}"))?;
            return Ok(false);
        }
        return Err("source_snapshot_content_address_collision".to_string());
    }
    fs::rename(staging, final_path)
        .map_err(|error| format!("source_snapshot_promote_failed:{error}"))?;
    if let Err(error) = File::open(final_path.parent().unwrap_or(Path::new("/")))
        .and_then(|directory| directory.sync_all())
    {
        let rollback = fs::remove_file(final_path)
            .err()
            .map(|failure| format!("; rollback_failed:{failure}"))
            .unwrap_or_default();
        return Err(format!(
            "source_snapshot_output_sync_failed:{error}{rollback}"
        ));
    }
    Ok(true)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("source_snapshot_unobservable:{error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("source_snapshot_file_unsafe".to_string());
    }
    let mut file =
        File::open(path).map_err(|error| format!("source_snapshot_read_failed:{error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("source_snapshot_read_failed:{error}"))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
}

fn rollback_staging(staging: &Path, output: &Path, created_output: bool) {
    let _ = fs::remove_file(staging);
    if created_output {
        let _ = fs::remove_dir(output);
    }
}

fn rollback_published(path: &Path, output: &Path, created_output: bool) {
    let _ = fs::remove_file(path);
    if created_output {
        let _ = fs::remove_dir(output);
    }
}

#[allow(clippy::too_many_arguments)]
fn report(
    plan: &SourceSnapshotPlan,
    mutates: bool,
    requested_apply: bool,
    snapshot_digest: Option<String>,
    commit_read_back: Option<String>,
    executed_argv: Vec<Vec<String>>,
    blockers: Vec<String>,
    status: &'static str,
) -> SourceSnapshotReport {
    SourceSnapshotReport {
        schema: "cynic.source_snapshot.report.v1",
        status,
        mutates,
        requested_apply,
        source_commit: plan.source_commit.clone(),
        snapshot_path: (status == "proved")
            .then(|| {
                snapshot_digest.as_ref().map(|digest| {
                    PathBuf::from(&plan.output_dir)
                        .join(digest.trim_start_matches("sha256:"))
                        .display()
                        .to_string()
                })
            })
            .flatten(),
        snapshot_digest,
        snapshot_format: "git_bundle_v1",
        commit_read_back,
        executed_argv,
        blockers,
        rollback_intent: plan.rollback_intent,
    }
}
