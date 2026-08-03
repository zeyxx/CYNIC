use std::{ffi::CString, fs, os::unix::ffi::OsStrExt, path::Path};

use crate::domain::disk::build_runner::{
    BUILD_CACHE_OBSERVATION_SCHEMA, BuildCacheDesiredState, BuildCacheMutationPort,
    BuildCacheObservation, BuildCacheObservationPort, BuildCachePathState,
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
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| "build_cache_state_root_invalid".to_string())?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `path` is NUL-terminated and `stats` points to writable memory.
    if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
        return Err("build_cache_available_bytes_unobservable".to_string());
    }
    // SAFETY: statvfs returned success and initialized `stats`.
    let stats = unsafe { stats.assume_init() };
    Ok(stats.f_bavail.saturating_mul(stats.f_frsize))
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
            canonical_target: target.display().to_string(),
            minimum_available_bytes: 1,
            reclaim_profile: "debug".to_string(),
            preserved_profiles: vec!["release".to_string()],
            noncanonical_cache_paths: Vec::new(),
        };

        LocalBuildCacheAdapter
            .reclaim_profile(&desired)
            .expect("bounded reclaim");

        assert!(!target.join("debug").exists());
        assert!(target.join("release/deps/runtime").is_file());
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
            canonical_target: root.path().join("linked/targets").display().to_string(),
            minimum_available_bytes: 1,
            reclaim_profile: "debug".to_string(),
            preserved_profiles: vec!["release".to_string()],
            noncanonical_cache_paths: Vec::new(),
        };

        let error = LocalBuildCacheAdapter
            .reclaim_profile(&desired)
            .expect_err("symlinked ancestor must block");

        assert_eq!(error, "build_cache_mutation_path_rejected");
        assert!(outside.path().join("targets/debug").is_dir());
    }
}
