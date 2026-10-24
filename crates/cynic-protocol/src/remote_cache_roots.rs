//! Pure resolution of the remote build cache roots.
//!
//! The build trigger runs on one host and builds on another. The cache
//! directories belong to the *runner*, not to the client: the runner's state
//! root is what holds the checkout, the Cargo target dir and the sccache
//! cache. Resolving them on the client produces a path that only works when
//! both hosts happen to share a layout — which is why the two conventions
//! (`shared-target` and `build-targets/<profile>`) existed side by side and
//! why 33 GiB of cache was invisible to the disk observer.
//!
//! The resolution is pure: the adapter observes the runner's paths, this
//! module turns the observation into the paths the build must use.

use serde::{Deserialize, Serialize};

/// Schema identifier of the remote cache root resolution.
pub const REMOTE_CACHE_ROOTS_SCHEMA: &str = "cynic.build_runner.remote_cache_roots.v1";

/// The paths a remote build must use, as resolved on the runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RemoteCacheRoots {
    pub schema: &'static str,
    /// The runner's own state root.
    pub state_root: String,
    /// Where the per-SHA checkout is materialized on the runner.
    pub checkout_base: String,
    /// Where Cargo must write its build artifacts on the runner.
    pub target_dir: String,
    /// Where sccache must keep its cache on the runner.
    pub sccache_dir: String,
    /// The sccache binary on the runner.
    pub sccache_bin: String,
}

/// The profile segment used under the target root.
///
/// The runner's layout is `<state_root>/build-targets/<profile>`, matching the
/// CI workflow and the scheduler configuration on the builder. One layout,
/// one cache, one observer.
pub const BUILD_TARGET_PROFILE: &str = "fgate";

/// Derive the cache roots from the runner's state root.
///
/// The state root is the only thing the client needs to know, and it is read
/// from the runner itself (`cynic runtime paths --json`), never assumed from
/// the client's own layout.
pub fn remote_cache_roots(state_root: &str) -> RemoteCacheRoots {
    let root = state_root.trim_end_matches('/');
    RemoteCacheRoots {
        schema: REMOTE_CACHE_ROOTS_SCHEMA,
        state_root: root.to_string(),
        checkout_base: format!("{root}/checkouts"),
        target_dir: format!("{root}/build-targets/{BUILD_TARGET_PROFILE}"),
        sccache_dir: format!("{root}/sccache-cache"),
        sccache_bin: format!("{root}/bin/sccache"),
    }
}

/// Scope a profile target root to one checkout.
///
/// WHY: `<root>/build-targets/<profile>` shared across per-SHA checkouts
/// poisons cargo's unit fingerprints — a module added in SHA B resolves
/// against SHA A's stale artifacts and the test profile dies with E0432
/// while check passes. One directory per checkout keeps the warm cache
/// (same SHA rebuilds reuse it, sccache absorbs the rest) without letting
/// two source sets share one fingerprint space.
///
/// The name is the checkout's final path component (`cynic-<12hex>`), never
/// a caller invention. A hostile or empty name (`..`, `/`, blank) yields
/// `None` and the caller keeps the unscoped root rather than writing
/// outside the cache.
pub fn scoped_target_dir(profile_target_root: &str, checkout_name: &str) -> Option<String> {
    let name = checkout_name.trim();
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        return None;
    }
    Some(format!(
        "{}/{}",
        profile_target_root.trim_end_matches('/'),
        name
    ))
}

/// Extract the `state_root` from a `cynic runtime paths --json` read-back.
///
/// The payload is a JSON object with a `state_root` string field. The parse
/// is deliberately minimal and strict: a missing or empty field yields
/// `None`, and the caller refuses the build rather than guessing a path on a
/// host that is not the one the build runs on.
pub fn parse_state_root(paths_json: &str) -> Option<String> {
    let document: serde_json::Value = serde_json::from_str(paths_json).ok()?;
    let root = document.get("state_root")?.as_str()?.trim();
    if root.is_empty() {
        return None;
    }
    Some(root.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cache_roots_live_under_the_runner_state_root() {
        let roots = remote_cache_roots("/var/lib/cynic");
        assert_eq!(roots.state_root, "/var/lib/cynic");
        assert_eq!(roots.checkout_base, "/var/lib/cynic/checkouts");
        assert_eq!(roots.target_dir, "/var/lib/cynic/build-targets/fgate");
        assert_eq!(roots.sccache_dir, "/var/lib/cynic/sccache-cache");
        assert_eq!(roots.sccache_bin, "/var/lib/cynic/bin/sccache");
    }

    #[test]
    fn a_trailing_slash_does_not_double_up() {
        let roots = remote_cache_roots("/var/lib/cynic/");
        assert_eq!(roots.target_dir, "/var/lib/cynic/build-targets/fgate");
    }

    #[test]
    fn a_second_runner_gets_its_own_paths_not_the_clients() {
        // The failure this prevents: the client resolved /var/lib/cynic and
        // sent it to a runner whose state root is elsewhere.
        let roots = remote_cache_roots("/home/titouan/.local/share/cynic");
        assert_eq!(
            roots.target_dir,
            "/home/titouan/.local/share/cynic/build-targets/fgate"
        );
        assert_ne!(roots.target_dir, "/var/lib/cynic/shared-target");
    }

    #[test]
    fn the_state_root_is_read_from_the_runner_payload() {
        let payload = r#"{"schema":"cynic.runtime_paths.v1","state_root":"/var/lib/cynic"}"#;
        assert_eq!(parse_state_root(payload).as_deref(), Some("/var/lib/cynic"));
    }

    #[test]
    fn the_target_dir_scopes_to_the_checkout_name() {
        assert_eq!(
            scoped_target_dir("/var/lib/cynic/build-targets/fgate", "cynic-655786214746"),
            Some("/var/lib/cynic/build-targets/fgate/cynic-655786214746".to_string())
        );
    }

    #[test]
    fn a_hostile_checkout_name_falls_back_to_none() {
        for hostile in ["", "  ", ".", "..", "a/b", "a\\b", "cynic-\0x"] {
            assert_eq!(
                scoped_target_dir("/var/lib/cynic/build-targets/fgate", hostile),
                None,
                "checkout name must never escape the cache: {hostile:?}"
            );
        }
    }

    #[test]
    fn an_unreadable_payload_yields_no_state_root() {
        assert_eq!(parse_state_root("not json"), None);
        assert_eq!(parse_state_root("{}"), None);
        assert_eq!(parse_state_root(r#"{"state_root":""}"#), None);
        assert_eq!(parse_state_root(r#"{"state_root":42}"#), None);
    }
}
