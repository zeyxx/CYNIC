use serde::{Deserialize, Serialize};

pub const BUILD_RECIPE_SCHEMA: &str = "cynic.build.recipe.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRecipe {
    pub schema: String,
    pub action: String,
    pub target_profile: String,
    pub allowed_actions: Vec<String>,
    pub rust_min_stack: usize,
    pub cargo_incremental: bool,
    pub memory_limit_bytes: Option<u64>,
    pub target_crates: Vec<String>,
    pub excluded_crates: Vec<String>,
    pub toolchain_override: Option<String>,
    pub env_overrides: Vec<String>,
    pub max_parallel_jobs: usize,
    pub timeout_secs: u64,
    pub evidence_refs: Vec<String>,
}

/// Canonical recipe constructor: recipes exist only for registry-composed
/// actions. The action set, the target profile and the admission scope all
/// derive from `infra/command-registry.json` (via
/// `cynic_protocol::command_registry`); the execution environment
/// (stack, memory, parallelism) stays a code default per profile because it
/// describes the runner, not the command.
pub fn build_recipe(action: &str) -> Result<BuildRecipe, String> {
    // A recipe exists exactly when the registry composes the action: a recipe
    // is an execution commitment over registry data, not a vocabulary entry.
    // (Unwired actions such as cargo:run compose no argv and get no recipe.)
    if cynic_protocol::command_registry::registry_argv(action).is_none() {
        return Err(format!("unsupported_canonical_build_action:{action}"));
    }
    let target_profile = cynic_protocol::command_registry::registry_profile(action)
        .ok_or_else(|| format!("unsupported_canonical_build_action:{action}"))?;
    let (rust_min_stack, memory_limit_bytes) = match target_profile.as_str() {
        "release" => (536_870_912usize, Some(8_589_934_592u64)),
        _ => (536_870_912usize, Some(4_294_967_296u64)),
    };
    Ok(BuildRecipe {
        schema: BUILD_RECIPE_SCHEMA.to_string(),
        action: action.to_string(),
        target_profile,
        allowed_actions: vec![action.to_string()],
        rust_min_stack,
        cargo_incremental: false,
        memory_limit_bytes,
        target_crates: vec![],
        excluded_crates: vec![],
        toolchain_override: None,
        env_overrides: vec![],
        max_parallel_jobs: 1,
        timeout_secs: 600,
        evidence_refs: vec![],
    })
}

pub fn validate_build_recipe(recipe: &BuildRecipe) -> Result<(), String> {
    if recipe.action.trim().is_empty() {
        return Err("build_recipe_action_empty".to_string());
    }
    if recipe.allowed_actions.is_empty() {
        return Err("build_recipe_allowed_actions_empty".to_string());
    }
    if !recipe.allowed_actions.iter().any(|a| a == &recipe.action) {
        return Err("build_recipe_action_not_in_allowed_actions".to_string());
    }
    if recipe.rust_min_stack < 67_108_864 {
        return Err("build_recipe_rust_min_stack_too_small".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_recipes_come_from_the_registry() {
        // Recipes compose only registry actions: the argv a recipe runs is
        // declared in infra/command-registry.json, never restated here.
        for action in ["audit:crate", "cargo:check", "cargo:build", "cargo:test"] {
            let recipe = build_recipe(action).expect("registry recipe");
            assert_eq!(recipe.action, action);
            assert_eq!(recipe.target_profile, "debug");
            assert!(recipe.allowed_actions.contains(&action.to_string()));
            assert!(validate_build_recipe(&recipe).is_ok());
        }
        let release = build_recipe("cargo:build-release").expect("release recipe");
        assert_eq!(release.target_profile, "release");
        assert!(!release.cargo_incremental);
        assert!(validate_build_recipe(&release).is_ok());
    }

    #[test]
    fn retired_make_recipes_are_refused() {
        for action in ["make:check", "make:build", "make:build-release"] {
            assert!(build_recipe(action).is_err(), "{action} must be refused");
        }
    }

    #[test]
    fn rejects_unsupported_action() {
        assert!(build_recipe("custom:hack").is_err());
    }
}
