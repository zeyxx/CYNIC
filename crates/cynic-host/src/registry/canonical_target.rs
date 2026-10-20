//! The builder's canonical build cache path, resolved from the registry.
//!
//! THE DIVERGENCE THIS CLOSES. The runner compiled into
//! `CynicPaths::state_dir().join("shared-target")` — a literal in code.
//! The registry declared `build-targets/bootstrap` as the canonical
//! target. The two were different paths, and the registry never
//! mentioned `shared-target` at all.
//!
//! That divergence is not cosmetic. `cynic build reclaim` reads the
//! registry to know what to reclaim, and refuses to touch the canonical
//! profile when the canonical path does not exist:
//!
//! ```text
//! canonical_target_state: "missing"
//! reclaim_profile_state: "missing"
//! ```
//!
//! So the 30 Go of real build cache sat outside the governance of the
//! policy that exists to reclaim it, while the registry pointed at a
//! directory that nothing had ever created.
//!
//! WHY THE REGISTRY WINS. `infra/directory-tree.json` is the authority
//! for the runtime tree, and it says the build target is
//! `{state-root}/build-targets/<profile>`. A path in the code and a path
//! in the registry that disagree are two claims about the same machine;
//! the registry is the one that is read by the tooling that has to act
//! on it.
//!
//! The runner therefore asks the registry where the cache is, and a
//! registry that cannot answer is an explicit refusal — never a guess.

use serde_json::Value;

/// The relative path of the canonical build cache, as the registry
/// declares it for the build runner.
///
/// The value is RELATIVE to the state root, exactly as the registry
/// states it (`canonical_target_relative_path`). The caller joins it
/// with the state root it already knows.
pub const CANONICAL_TARGET_KEY: &str = "canonical_target_relative_path";

/// The registry path of the build runner's resource policy.
const BUILD_LIFECYCLE_KEY: &str = "build_lifecycle";

/// The canonical build cache path, relative to the state root.
///
/// Reads the registry's `build_lifecycle.canonical_target_relative_path`
/// for the node whose `role` is `build_runner`.
///
/// WHY A NODE BY ROLE. The cache belongs to the machine that builds,
/// not to the machine that reads this registry. A registry can declare
/// several nodes, and only one of them is the build runner; resolving by
/// role is what makes the answer unambiguous.
///
/// The path is validated before it is returned: an absolute path or one
/// escaping the state root is refused, because this value is JOINED to
/// the state root and later used to delete a directory.
pub fn canonical_target_relative_path(registry: &Value) -> Result<String, String> {
    let nodes = registry
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry_nodes_missing".to_string())?;
    let mut found: Vec<String> = Vec::new();
    for node in nodes {
        if node.get("role").and_then(Value::as_str) != Some("build_runner") {
            continue;
        }
        let Some(policy) = node.get(BUILD_LIFECYCLE_KEY) else {
            continue;
        };
        let Some(path) = policy
            .get(CANONICAL_TARGET_KEY)
            .and_then(Value::as_str)
        else {
            continue;
        };
        let path = path.trim();
        if path.is_empty() {
            continue;
        }
        if path.starts_with('/') {
            return Err("canonical_target_not_relative".to_string());
        }
        if path
            .split('/')
            .any(|component| component == ".." || component.is_empty())
        {
            return Err("canonical_target_escapes_state_root".to_string());
        }
        found.push(path.to_string());
    }
    match found.as_slice() {
        [] => Err("build_runner_canonical_target_undeclared".to_string()),
        [path] => Ok(path.clone()),
        _ => Err("build_runner_canonical_target_ambiguous".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn registry_with(path: &str) -> Value {
        json!({
            "nodes": [
                { "id": "local", "role": "observer" },
                {
                    "id": "cynic-builder",
                    "role": "build_runner",
                    "build_lifecycle": {
                        "canonical_target_relative_path": path
                    }
                }
            ]
        })
    }

    #[test]
    fn the_build_runner_declares_the_canonical_target() {
        let registry = registry_with("build-targets/bootstrap");
        assert_eq!(
            canonical_target_relative_path(&registry).expect("resolved"),
            "build-targets/bootstrap"
        );
    }

    #[test]
    fn a_node_that_is_not_the_build_runner_is_ignored() {
        let registry = json!({
            "nodes": [
                {
                    "id": "cortex",
                    "role": "observer",
                    "build_lifecycle": { "canonical_target_relative_path": "wrong" }
                }
            ]
        });
        assert_eq!(
            canonical_target_relative_path(&registry).expect_err("refused"),
            "build_runner_canonical_target_undeclared"
        );
    }

    #[test]
    fn an_absolute_path_is_refused() {
        let registry = registry_with("/var/lib/cynic/shared-target");
        assert_eq!(
            canonical_target_relative_path(&registry).expect_err("refused"),
            "canonical_target_not_relative"
        );
    }

    #[test]
    fn a_path_escaping_the_state_root_is_refused() {
        let registry = registry_with("../outside");
        assert_eq!(
            canonical_target_relative_path(&registry).expect_err("refused"),
            "canonical_target_escapes_state_root"
        );
    }

    #[test]
    fn two_build_runners_are_ambiguous_rather_than_guessed() {
        let registry = json!({
            "nodes": [
                {
                    "id": "a", "role": "build_runner",
                    "build_lifecycle": { "canonical_target_relative_path": "one" }
                },
                {
                    "id": "b", "role": "build_runner",
                    "build_lifecycle": { "canonical_target_relative_path": "two" }
                }
            ]
        });
        assert_eq!(
            canonical_target_relative_path(&registry).expect_err("refused"),
            "build_runner_canonical_target_ambiguous"
        );
    }

    #[test]
    fn a_registry_without_the_policy_is_refused() {
        let registry = json!({ "nodes": [{ "id": "a", "role": "build_runner" }] });
        assert_eq!(
            canonical_target_relative_path(&registry).expect_err("refused"),
            "build_runner_canonical_target_undeclared"
        );
    }
}
