use std::{
    fs,
    path::{Path, PathBuf},
};

use sysinfo::Disks;

use crate::domain::disk::build_runner::{
    BUILD_CACHE_OBSERVATION_SCHEMA, BuildCacheDesiredState, BuildCacheMutationPort,
    BuildCacheObservation, BuildCacheObservationPort, BuildCachePathState, EphemeralEntry,
    EphemeralKind, EphemeralScope,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalBuildCacheAdapter;

impl BuildCacheObservationPort for LocalBuildCacheAdapter {
    fn observe(&self, desired: &BuildCacheDesiredState) -> BuildCacheObservation {
        let canonical_target = Path::new(&desired.canonical_target);
        let reclaim_profile_path = canonical_target.join(&desired.reclaim_profile);
        let mut unknowns = Vec::new();
        let available_bytes = match available_bytes(Path::new(&desired.state_root)) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                unknowns.push(error);
                None
            }
        };
        let state_root = Path::new(&desired.state_root);
        let temporary_root = Path::new(&desired.temporary_root);
        let canonical_target_state =
            directory_chain_state(state_root, canonical_target, &mut unknowns);
        let reclaim_profile_state =
            directory_chain_state(state_root, &reclaim_profile_path, &mut unknowns);
        let preserved_profile_states = desired
            .preserved_profiles
            .iter()
            .map(|profile| {
                directory_chain_state(state_root, &canonical_target.join(profile), &mut unknowns)
            })
            .collect();
        let mut observed_noncanonical_cache_paths = Vec::new();
        for path in &desired.noncanonical_cache_paths {
            match fs::symlink_metadata(path) {
                Ok(_) => observed_noncanonical_cache_paths.push(path.clone()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => unknowns.push(format!(
                    "build_cache_noncanonical_path_unobservable:{path}:{error}"
                )),
            }
        }
        let (observed_ephemeral, ephemeral_skipped) =
            observe_ephemeral(desired, state_root, temporary_root);
        BuildCacheObservation {
            schema: BUILD_CACHE_OBSERVATION_SCHEMA,
            node_id: desired.node_id.clone(),
            available_bytes,
            canonical_target: desired.canonical_target.clone(),
            canonical_target_state,
            reclaim_profile_path: reclaim_profile_path.display().to_string(),
            reclaim_profile_state,
            preserved_profile_states,
            observed_noncanonical_cache_paths,
            observed_ephemeral,
            ephemeral_skipped,
            unknowns,
        }
    }
}

impl BuildCacheMutationPort for LocalBuildCacheAdapter {
    fn reclaim_profile(&self, desired: &BuildCacheDesiredState) -> Result<String, String> {
        let preserves_release = desired.preserved_profiles.len() == 1
            && desired.preserved_profiles.first().map(String::as_str) == Some("release");
        if desired.reclaim_profile != "debug" || !preserves_release {
            return Err("build_cache_mutation_policy_rejected".to_string());
        }
        let target = Path::new(&desired.canonical_target);
        let state_root = Path::new(&desired.state_root);
        if target.strip_prefix(state_root).is_err() || target == state_root {
            return Err("build_cache_mutation_boundary_rejected".to_string());
        }
        let profile = target.join(&desired.reclaim_profile);
        let mut unknowns = Vec::new();
        for path in [target, profile.as_path()] {
            if directory_chain_state(state_root, path, &mut unknowns)
                != BuildCachePathState::SafeDirectory
            {
                return Err("build_cache_mutation_path_rejected".to_string());
            }
        }
        fs::remove_dir_all(&profile)
            .map_err(|error| format!("build_cache_profile_reclaim_failed:{error}"))?;
        fs::File::open(target)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("build_cache_reclaim_sync_failed:{error}"))?;
        match fs::symlink_metadata(&profile) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(_) => return Err("build_cache_profile_reclaim_read_back_failed".to_string()),
            Err(error) => {
                return Err(format!("build_cache_reclaim_read_back_unknown:{error}"));
            }
        }
        Ok(format!("removed_reproducible_cache:{}", profile.display()))
    }


    /// Remove the non-canonical build caches the registry declares.
    ///
    /// Each path was built by `load_build_cache_desired_state` from the registry
    /// (`base: state_root | temporary_directory` + `relative_path`), so the
    /// boundary check here is the same one, applied to the same strings — not a
    /// second derivation.
    ///
    /// Fail-closed on anything ambiguous: a path outside its declared base, a
    /// symlinked ancestor, or a read-back that cannot prove absence all abort
    /// before the next path is touched. A cache removal that cannot be proven is
    /// a cache removal that did not happen.
    fn reclaim_noncanonical_caches(
        &self,
        desired: &BuildCacheDesiredState,
    ) -> Result<String, String> {
        let state_root = Path::new(&desired.state_root);
        let temporary_root = Path::new(&desired.temporary_root);
        let mut removed = Vec::new();
        let mut skipped = Vec::new();

        for raw in &desired.noncanonical_cache_paths {
            let path = Path::new(raw);
            if !path.is_absolute() {
                return Err(format!(
                    "build_cache_noncanonical_path_not_absolute:{raw}"
                ));
            }
            // The path must live under one of the two roots the loader used.
            // Either root is accepted because the loader accepted either; the
            // check is that it is under A root, never that it is under a
            // particular one.
            let under_state = path.strip_prefix(state_root).is_ok();
            let under_temporary = path.strip_prefix(temporary_root).is_ok();
            if !under_state && !under_temporary {
                return Err(format!(
                    "build_cache_noncanonical_path_outside_declared_base:{raw}"
                ));
            }
            if path == state_root || path == temporary_root {
                return Err(format!(
                    "build_cache_noncanonical_path_is_a_root:{raw}"
                ));
            }

            let mut unknowns = Vec::new();
            match fs::symlink_metadata(path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    skipped.push(raw.clone());
                    continue;
                }
                Err(error) => {
                    return Err(format!(
                        "build_cache_noncanonical_path_unobservable:{raw}:{error}"
                    ));
                }
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() {
                        return Err(format!(
                            "build_cache_noncanonical_path_is_symlink:{raw}"
                        ));
                    }
                    if !metadata.is_dir() {
                        return Err(format!(
                            "build_cache_noncanonical_path_not_a_directory:{raw}"
                        ));
                    }
                }
            }

            // Every ancestor between the root and the path must be a real
            // directory, so a symlinked intermediate cannot redirect the removal.
            let boundary = if under_state { state_root } else { temporary_root };
            if directory_chain_state(boundary, path, &mut unknowns)
                != BuildCachePathState::SafeDirectory
            {
                return Err(format!(
                    "build_cache_noncanonical_path_rejected:{raw}"
                ));
            }
            if !unknowns.is_empty() {
                return Err(format!(
                    "build_cache_noncanonical_path_unknown:{raw}:{}",
                    unknowns.join(",")
                ));
            }

            fs::remove_dir_all(path)
                .map_err(|error| format!("build_cache_noncanonical_remove_failed:{raw}:{error}"))?;

            // Read back: absence is the only accepted proof.
            match fs::symlink_metadata(path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    removed.push(raw.clone());
                }
                Ok(_) => {
                    return Err(format!(
                        "build_cache_noncanonical_read_back_failed:{raw}"
                    ));
                }
                Err(error) => {
                    return Err(format!(
                        "build_cache_noncanonical_read_back_unknown:{raw}:{error}"
                    ));
                }
            }
        }

        Ok(format!(
            "removed_noncanonical_caches:{}:skipped_absent:{}",
            if removed.is_empty() {
                "none".to_string()
            } else {
                removed.join("|")
            },
            if skipped.is_empty() {
                "none".to_string()
            } else {
                skipped.join("|")
            }
        ))
    }

    /// Remove retention-eligible ephemeral paths.
    ///
    /// Every path is re-verified at removal time because the observation may
    /// be stale: it must sit exactly one level under a declared retention
    /// scope parent, match that scope's strict name pattern, pass the
    /// ancestor chain check, and — for workspaces — read back terminal
    /// against the store right now. A workspace whose record is live or
    /// unreadable at this instant aborts the whole operation before anything
    /// else is touched.
    fn reclaim_ephemeral_paths(
        &self,
        desired: &BuildCacheDesiredState,
        paths: &[String],
    ) -> Result<String, String> {
        let state_root = Path::new(&desired.state_root);
        let temporary_root = Path::new(&desired.temporary_root);
        let mut removed = Vec::new();
        let mut skipped = Vec::new();

        for raw in paths {
            let path = Path::new(raw);
            let scope = scope_for_ephemeral(desired, state_root, temporary_root, path)
                .ok_or_else(|| {
                    format!("build_cache_ephemeral_path_outside_declared_scopes:{raw}")
                })?;
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    format!("build_cache_ephemeral_path_name_not_utf8:{raw}")
                })?;
            if !name_matches(scope.kind, name) {
                return Err(format!("build_cache_ephemeral_path_name_rejected:{raw}"));
            }
            if scope.kind == EphemeralKind::WorkloadWorkspace {
                match read_workload_terminal(
                    state_root,
                    scope.workload_store_relative_path.as_deref(),
                    name,
                ) {
                    Some(true) => {}
                    Some(false) => {
                        return Err(format!(
                            "build_cache_ephemeral_workspace_live:{raw}"
                        ));
                    }
                    None => {
                        return Err(format!(
                            "build_cache_ephemeral_workspace_unclassifiable:{raw}"
                        ));
                    }
                }
            }

            let mut unknowns = Vec::new();
            match fs::symlink_metadata(path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    skipped.push(raw.clone());
                    continue;
                }
                Err(error) => {
                    return Err(format!(
                        "build_cache_ephemeral_path_unobservable:{raw}:{error}"
                    ));
                }
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() {
                        return Err(format!(
                            "build_cache_ephemeral_path_is_symlink:{raw}"
                        ));
                    }
                    if !metadata.is_dir() {
                        return Err(format!(
                            "build_cache_ephemeral_path_not_a_directory:{raw}"
                        ));
                    }
                }
            }
            let parent = path.parent().ok_or_else(|| {
                format!("build_cache_ephemeral_path_has_no_parent:{raw}")
            })?;
            if directory_chain_state(parent, path, &mut unknowns)
                != BuildCachePathState::SafeDirectory
            {
                return Err(format!("build_cache_ephemeral_path_rejected:{raw}"));
            }
            if !unknowns.is_empty() {
                return Err(format!(
                    "build_cache_ephemeral_path_unknown:{raw}:{}",
                    unknowns.join(",")
                ));
            }

            fs::remove_dir_all(path)
                .map_err(|error| format!("build_cache_ephemeral_remove_failed:{raw}:{error}"))?;

            match fs::symlink_metadata(path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    removed.push(raw.clone());
                }
                Ok(_) => {
                    return Err(format!(
                        "build_cache_ephemeral_read_back_failed:{raw}"
                    ));
                }
                Err(error) => {
                    return Err(format!(
                        "build_cache_ephemeral_read_back_unknown:{raw}:{error}"
                    ));
                }
            }
        }

        Ok(format!(
            "removed_ephemeral_paths:{}:skipped_absent:{}",
            if removed.is_empty() {
                "none".to_string()
            } else {
                removed.join("|")
            },
            if skipped.is_empty() {
                "none".to_string()
            } else {
                skipped.join("|")
            }
        ))
    }
}

/// Observe the declared retention scopes: classify each entry, never follow
/// anything ambiguous.
///
/// An entry becomes an `EphemeralEntry` only when ALL of these hold: it is a
/// real directory (never a symlink, never a file), its name matches the
/// scope kind's strict pattern, and its mtime is readable. Anything else is
/// listed in `ephemeral_skipped` — reported, never eligible, never blocking.
/// Workspaces additionally carry a liveness read-back against the workload
/// store; a missing or unreadable record classifies as unclassifiable
/// (`workload_terminal: None`), which the pure decision treats as ineligible.
fn observe_ephemeral(
    desired: &BuildCacheDesiredState,
    state_root: &Path,
    temporary_root: &Path,
) -> (Vec<EphemeralEntry>, Vec<String>) {
    let mut observed = Vec::new();
    let mut skipped = Vec::new();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    for scope in &desired.ephemeral_scopes {
        let base = match scope.base.as_str() {
            "state_root" => state_root,
            "temporary_directory" => temporary_root,
            _ => {
                skipped.push(format!(
                    "ephemeral_scope_base_unsupported:{}",
                    scope.relative_path
                ));
                continue;
            }
        };
        let parent = base.join(&scope.relative_path);
        let entries = match fs::read_dir(&parent) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                skipped.push(format!(
                    "ephemeral_scope_unobservable:{}:{error}",
                    parent.display()
                ));
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    skipped.push(format!(
                        "ephemeral_entry_unobservable:{}:{error}",
                        parent.display()
                    ));
                    continue;
                }
            };
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                skipped.push(format!(
                    "ephemeral_entry_name_not_utf8:{}",
                    parent.display()
                ));
                continue;
            };
            if !name_matches(scope.kind, name) {
                skipped.push(format!("ephemeral_entry_name_rejected:{name}"));
                continue;
            }
            match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    skipped.push(format!("ephemeral_entry_is_symlink:{name}"));
                    continue;
                }
                Ok(metadata) if !metadata.is_dir() => {
                    skipped.push(format!("ephemeral_entry_not_a_directory:{name}"));
                    continue;
                }
                Ok(metadata) => {
                    let age_ms = match metadata.modified() {
                        Ok(modified) => {
                            let mtime_ms = modified
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|duration| duration.as_millis() as u64)
                                .unwrap_or(now_ms);
                            now_ms.saturating_sub(mtime_ms)
                        }
                        Err(_) => {
                            skipped.push(format!("ephemeral_entry_mtime_unknown:{name}"));
                            continue;
                        }
                    };
                    let workload_terminal = match scope.kind {
                        EphemeralKind::EvidenceCheckout => None,
                        EphemeralKind::WorkloadWorkspace => read_workload_terminal(
                            state_root,
                            scope.workload_store_relative_path.as_deref(),
                            name,
                        ),
                    };
                    observed.push(EphemeralEntry {
                        path: path.display().to_string(),
                        age_ms,
                        kind: scope.kind,
                        workload_terminal,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    skipped.push(format!("ephemeral_entry_unobservable:{name}:{error}"));
                }
            }
        }
    }
    observed.sort_by(|left, right| left.path.cmp(&right.path));
    (observed, skipped)
}

/// Strict name patterns: checkouts are `cynic-<hex>` evidence clones,
/// workspaces are `build-<hex>` scheduler materializations. Anything else
/// under a scope parent is not ours to judge.
fn name_matches(kind: EphemeralKind, name: &str) -> bool {
    let (prefix, min_hex, max_hex) = match kind {
        EphemeralKind::EvidenceCheckout => ("cynic-", 7, 64),
        EphemeralKind::WorkloadWorkspace => ("build-", 16, 64),
    };
    let Some(hex) = name.strip_prefix(prefix) else {
        return false;
    };
    (min_hex..=max_hex).contains(&hex.len())
        && !hex.is_empty()
        && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Minimal liveness read-back: latest `revision-*` file of the workload
/// record, `record.state` only. No chain validation — this is an observer,
/// not the store; any doubt classifies as unclassifiable (None), which is
/// never eligible.
fn read_workload_terminal(
    state_root: &Path,
    store_relative: Option<&str>,
    workload_id: &str,
) -> Option<bool> {
    let store_relative = store_relative?;
    if workload_id.is_empty()
        || workload_id.contains('/')
        || workload_id.contains('\0')
    {
        return None;
    }
    let record_dir = state_root.join(store_relative).join(workload_id);
    let entries = fs::read_dir(&record_dir).ok()?;
    let mut latest: Option<(u64, PathBuf)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(revision) = name
            .strip_prefix("revision-")
            .and_then(|number| number.parse::<u64>().ok())
        else {
            continue;
        };
        if latest.as_ref().is_some_and(|(best, _)| *best >= revision) {
            continue;
        }
        latest = Some((revision, entry.path()));
    }
    let (_, path) = latest?;
    let bytes = fs::read(&path).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    match value
        .get("record")?
        .get("state")?
        .as_str()?
    {
        "completed" | "blocked" | "failed" | "degraded" | "recovered_unattributed" => Some(true),
        "dispatched" | "claimed" | "cancel_requested" => Some(false),
        _ => None,
    }
}

/// Resolve the retention scope governing `path`: the path must sit exactly
/// one level under a declared scope parent. Ambiguous (under two scopes of
/// different kinds) resolves to nothing — fail closed.
fn scope_for_ephemeral<'a>(
    desired: &'a BuildCacheDesiredState,
    state_root: &Path,
    temporary_root: &Path,
    path: &Path,
) -> Option<&'a crate::domain::disk::build_runner::EphemeralScope> {
    let parent = path.parent()?;
    let mut matches = desired.ephemeral_scopes.iter().filter(|scope| {
        let base = match scope.base.as_str() {
            "state_root" => state_root,
            "temporary_directory" => temporary_root,
            _ => return false,
        };
        base.join(&scope.relative_path) == parent
    });
    let first = matches.next()?;
    if matches.any(|scope| scope.kind != first.kind) {
        return None;
    }
    Some(first)
}

fn directory_chain_state(
    state_root: &Path,
    path: &Path,
    unknowns: &mut Vec<String>,
) -> BuildCachePathState {
    let Ok(relative) = path.strip_prefix(state_root) else {
        return BuildCachePathState::Rejected;
    };
    let mut current = state_root.to_path_buf();
    for component in std::iter::once(None).chain(relative.components().map(Some)) {
        if let Some(component) = component {
            current.push(component);
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return BuildCachePathState::Rejected;
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return BuildCachePathState::Missing;
            }
            Err(error) => {
                unknowns.push(format!(
                    "build_cache_path_unobservable:{}:{error}",
                    current.display()
                ));
                return BuildCachePathState::Unknown;
            }
        }
    }
    BuildCachePathState::SafeDirectory
}

fn available_bytes(path: &Path) -> Result<u64, String> {
    if !path.is_absolute() {
        return Err("build_cache_state_root_invalid".to_string());
    }
    Disks::new_with_refreshed_list()
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().components().count())
        .map(sysinfo::Disk::available_space)
        .ok_or_else(|| "build_cache_available_bytes_unobservable".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn mutation_preserves_release_and_removes_only_debug() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("targets");
        fs::create_dir_all(target.join("debug/deps")).unwrap();
        fs::create_dir_all(target.join("release/deps")).unwrap();
        fs::write(target.join("debug/deps/cache"), b"cache").unwrap();
        fs::write(target.join("release/deps/runtime"), b"runtime").unwrap();
        let desired = BuildCacheDesiredState {
            node_id: "builder-a".to_string(),
            state_root: root.path().display().to_string(),
            temporary_root: root.path().display().to_string(),
            canonical_target: target.display().to_string(),
            minimum_available_bytes: 1,
            reclaim_profile: "debug".to_string(),
            preserved_profiles: vec!["release".to_string()],
            noncanonical_cache_paths: Vec::new(),
            ephemeral_scopes: Vec::new(),
            retention_max_age_ms: None,
            retention_pressure_grace_ms: None,
        };

        LocalBuildCacheAdapter
            .reclaim_profile(&desired)
            .expect("bounded reclaim");

        assert!(!target.join("debug").exists());
        assert!(target.join("release/deps/runtime").is_file());
    }

    #[test]
    fn noncanonical_reclaim_removes_declared_caches_and_reads_back_absence() {
        let root = tempfile::tempdir().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let state_cache = root.path().join("source/repositories/CYNIC/target");
        let temp_cache = tmp.path().join("cynic-cargo-targets");
        fs::create_dir_all(state_cache.join("debug/deps")).unwrap();
        fs::create_dir_all(temp_cache.join("debug/deps")).unwrap();
        fs::write(state_cache.join("debug/deps/cache"), b"cache").unwrap();
        fs::write(temp_cache.join("debug/deps/cache"), b"cache").unwrap();
        let desired = BuildCacheDesiredState {
            node_id: "builder-a".to_string(),
            state_root: root.path().display().to_string(),
            temporary_root: tmp.path().display().to_string(),
            canonical_target: root.path().join("targets").display().to_string(),
            minimum_available_bytes: 1,
            reclaim_profile: "debug".to_string(),
            preserved_profiles: vec!["release".to_string()],
            noncanonical_cache_paths: vec![
                state_cache.display().to_string(),
                temp_cache.display().to_string(),
            ],
            ephemeral_scopes: Vec::new(),
            retention_max_age_ms: None,
            retention_pressure_grace_ms: None,
        };

        let evidence = LocalBuildCacheAdapter
            .reclaim_noncanonical_caches(&desired)
            .expect("both declared caches are within their bases");

        assert!(!state_cache.exists(), "state-root cache must be gone");
        assert!(!temp_cache.exists(), "temporary-root cache must be gone");
        assert!(evidence.starts_with("removed_noncanonical_caches:"));
        assert!(evidence.contains("cynic-cargo-targets"));
    }

    #[test]
    fn noncanonical_reclaim_skips_an_absent_cache_without_failing() {
        let root = tempfile::tempdir().unwrap();
        let absent = root.path().join("never-created");
        let desired = BuildCacheDesiredState {
            node_id: "builder-a".to_string(),
            state_root: root.path().display().to_string(),
            temporary_root: root.path().display().to_string(),
            canonical_target: root.path().join("targets").display().to_string(),
            minimum_available_bytes: 1,
            reclaim_profile: "debug".to_string(),
            preserved_profiles: vec!["release".to_string()],
            noncanonical_cache_paths: vec![absent.display().to_string()],
            ephemeral_scopes: Vec::new(),
            retention_max_age_ms: None,
            retention_pressure_grace_ms: None,
        };

        let evidence = LocalBuildCacheAdapter
            .reclaim_noncanonical_caches(&desired)
            .expect("an absent cache is the desired state, not a failure");

        assert!(evidence.contains("removed_noncanonical_caches:none"));
        assert!(evidence.contains("skipped_absent:"));
    }

    #[test]
    fn noncanonical_reclaim_rejects_a_path_outside_its_declared_base() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(outside.path().join("not-mine")).unwrap();
        let desired = BuildCacheDesiredState {
            node_id: "builder-a".to_string(),
            state_root: root.path().display().to_string(),
            temporary_root: root.path().display().to_string(),
            canonical_target: root.path().join("targets").display().to_string(),
            minimum_available_bytes: 1,
            reclaim_profile: "debug".to_string(),
            preserved_profiles: vec!["release".to_string()],
            noncanonical_cache_paths: vec![outside
                .path()
                .join("not-mine")
                .display()
                .to_string()],
            ephemeral_scopes: Vec::new(),
            retention_max_age_ms: None,
            retention_pressure_grace_ms: None,
        };

        let error = LocalBuildCacheAdapter
            .reclaim_noncanonical_caches(&desired)
            .expect_err("a path under neither root must be refused");

        assert!(error.contains("outside_declared_base"), "got: {error}");
        assert!(
            outside.path().join("not-mine").is_dir(),
            "the refused path must be untouched"
        );
    }

    #[test]
    fn noncanonical_reclaim_rejects_a_symlinked_cache() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(outside.path().join("real")).unwrap();
        let link = root.path().join("linked-cache");
        symlink(outside.path().join("real"), &link).unwrap();
        let desired = BuildCacheDesiredState {
            node_id: "builder-a".to_string(),
            state_root: root.path().display().to_string(),
            temporary_root: root.path().display().to_string(),
            canonical_target: root.path().join("targets").display().to_string(),
            minimum_available_bytes: 1,
            reclaim_profile: "debug".to_string(),
            preserved_profiles: vec!["release".to_string()],
            noncanonical_cache_paths: vec![link.display().to_string()],
            ephemeral_scopes: Vec::new(),
            retention_max_age_ms: None,
            retention_pressure_grace_ms: None,
        };

        let error = LocalBuildCacheAdapter
            .reclaim_noncanonical_caches(&desired)
            .expect_err("a symlinked cache must be refused, not followed");

        assert!(error.contains("is_symlink"), "got: {error}");
        assert!(
            outside.path().join("real").is_dir(),
            "the symlink target must be untouched"
        );
    }

    #[test]
    fn mutation_rejects_a_symlinked_target_ancestor() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(outside.path().join("targets/debug")).unwrap();
        symlink(outside.path(), root.path().join("linked")).unwrap();
        let desired = BuildCacheDesiredState {
            node_id: "builder-a".to_string(),
            state_root: root.path().display().to_string(),
            temporary_root: root.path().display().to_string(),
            canonical_target: root.path().join("linked/targets").display().to_string(),
            minimum_available_bytes: 1,
            reclaim_profile: "debug".to_string(),
            preserved_profiles: vec!["release".to_string()],
            noncanonical_cache_paths: Vec::new(),
            ephemeral_scopes: Vec::new(),
            retention_max_age_ms: None,
            retention_pressure_grace_ms: None,
        };

        let error = LocalBuildCacheAdapter
            .reclaim_profile(&desired)
            .expect_err("symlinked ancestor must block");

        assert_eq!(error, "build_cache_mutation_path_rejected");
        assert!(outside.path().join("targets/debug").is_dir());
    }

    fn retention_desired(
        root: &std::path::Path,
        scopes: Vec<EphemeralScope>,
    ) -> BuildCacheDesiredState {
        BuildCacheDesiredState {
            node_id: "builder-a".to_string(),
            state_root: root.display().to_string(),
            temporary_root: root.display().to_string(),
            canonical_target: root.join("targets").display().to_string(),
            minimum_available_bytes: 1,
            reclaim_profile: "debug".to_string(),
            preserved_profiles: vec!["release".to_string()],
            noncanonical_cache_paths: Vec::new(),
            ephemeral_scopes: scopes,
            retention_max_age_ms: Some(u64::MAX),
            retention_pressure_grace_ms: Some(0),
        }
    }

    fn checkout_scope() -> EphemeralScope {
        EphemeralScope {
            base: "state_root".to_string(),
            relative_path: "checkouts".to_string(),
            kind: EphemeralKind::EvidenceCheckout,
            workload_store_relative_path: None,
        }
    }

    fn workspace_scope() -> EphemeralScope {
        EphemeralScope {
            base: "state_root".to_string(),
            relative_path: "ws".to_string(),
            kind: EphemeralKind::WorkloadWorkspace,
            workload_store_relative_path: Some("wstore".to_string()),
        }
    }

    fn write_record(root: &std::path::Path, workload_id: &str, state: &str) {
        let dir = root.join("wstore").join(workload_id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("revision-1"),
            format!(r#"{{"record":{{"state":"{state}"}}}}"#),
        )
        .unwrap();
    }

    #[test]
    fn ephemeral_observe_classifies_checkouts_and_rejects_foreign_names() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("checkouts");
        fs::create_dir_all(parent.join("cynic-abcdef12")).unwrap();
        fs::create_dir_all(parent.join("random-stuff")).unwrap();
        fs::write(parent.join("notes.txt"), b"not a cache").unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(outside.path().join("real")).unwrap();
        symlink(outside.path().join("real"), parent.join("cynic-eeeeeeee")).unwrap();
        let desired = retention_desired(root.path(), vec![checkout_scope()]);

        let observation = LocalBuildCacheAdapter.observe(&desired);

        assert_eq!(observation.observed_ephemeral.len(), 1);
        let entry = &observation.observed_ephemeral[0];
        assert!(entry.path.ends_with("cynic-abcdef12"));
        assert_eq!(entry.kind, EphemeralKind::EvidenceCheckout);
        assert_eq!(entry.workload_terminal, None);
        assert!(
            observation
                .ephemeral_skipped
                .iter()
                .any(|line| line.contains("random-stuff")),
            "foreign dir must be reported skipped: {:?}",
            observation.ephemeral_skipped
        );
        assert!(
            observation
                .ephemeral_skipped
                .iter()
                .any(|line| line.contains("notes.txt")),
            "foreign file must be reported skipped: {:?}",
            observation.ephemeral_skipped
        );
        assert!(
            observation
                .ephemeral_skipped
                .iter()
                .any(|line| line.contains("cynic-eeeeeeee")),
            "symlink must be reported skipped: {:?}",
            observation.ephemeral_skipped
        );
        assert!(outside.path().join("real").is_dir());
    }

    #[test]
    fn ephemeral_observe_reads_workspace_liveness_from_the_store() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("ws");
        fs::create_dir_all(parent.join("build-0123456789abcdef")).unwrap();
        fs::create_dir_all(parent.join("build-1111111111111111")).unwrap();
        fs::create_dir_all(parent.join("build-2222222222222222")).unwrap();
        write_record(root.path(), "build-0123456789abcdef", "completed");
        write_record(root.path(), "build-1111111111111111", "claimed");
        let desired = retention_desired(root.path(), vec![workspace_scope()]);

        let observation = LocalBuildCacheAdapter.observe(&desired);

        let terminal = |name: &str| {
            observation
                .observed_ephemeral
                .iter()
                .find(|entry| entry.path.ends_with(name))
                .map(|entry| entry.workload_terminal)
        };
        assert_eq!(
            terminal("build-0123456789abcdef"),
            Some(Some(true)),
            "completed workspace must read back terminal"
        );
        assert_eq!(
            terminal("build-1111111111111111"),
            Some(Some(false)),
            "claimed workspace must read back live"
        );
        assert_eq!(
            terminal("build-2222222222222222"),
            Some(None),
            "record-less workspace must read back unclassifiable"
        );
    }

    #[test]
    fn ephemeral_mutation_removes_dead_and_refuses_live() {
        let root = tempfile::tempdir().unwrap();
        let checkouts = root.path().join("checkouts");
        let dead = checkouts.join("cynic-deadbeef");
        fs::create_dir_all(dead.join("target")).unwrap();
        let ws = root.path().join("ws");
        let live = ws.join("build-aaaaaaaaaaaaaaaa");
        fs::create_dir_all(&live).unwrap();
        write_record(root.path(), "build-aaaaaaaaaaaaaaaa", "claimed");
        let desired =
            retention_desired(root.path(), vec![checkout_scope(), workspace_scope()]);

        let evidence = LocalBuildCacheAdapter
            .reclaim_ephemeral_paths(&desired, &[dead.display().to_string()])
            .expect("dead checkout removes");
        assert!(!dead.exists());
        assert!(evidence.contains("cynic-deadbeef"), "got: {evidence}");

        let error = LocalBuildCacheAdapter
            .reclaim_ephemeral_paths(&desired, &[live.display().to_string()])
            .expect_err("live workspace must abort");
        assert!(error.contains("workspace_live"), "got: {error}");
        assert!(live.is_dir(), "the refused workspace must be untouched");
    }

    #[test]
    fn ephemeral_mutation_refuses_a_path_outside_declared_scopes() {
        let root = tempfile::tempdir().unwrap();
        let rogue = root.path().join("cynic-rogue1234");
        fs::create_dir_all(&rogue).unwrap();
        let desired = retention_desired(root.path(), vec![checkout_scope()]);

        let error = LocalBuildCacheAdapter
            .reclaim_ephemeral_paths(&desired, &[rogue.display().to_string()])
            .expect_err("outside any scope parent must be refused");
        assert!(error.contains("outside_declared_scopes"), "got: {error}");
        assert!(rogue.is_dir(), "the refused path must be untouched");
    }
}
