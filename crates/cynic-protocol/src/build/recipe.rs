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

pub fn build_recipe(action: &str) -> Result<BuildRecipe, String> {
    match action {
        "make:check" => Ok(BuildRecipe {
            schema: BUILD_RECIPE_SCHEMA.to_string(),
            action: "make:check".to_string(),
            target_profile: "debug".to_string(),
            allowed_actions: vec![
                "make:check".to_string(),
                "cargo:check".to_string(),
                "cargo:run".to_string(),
                "cargo:test".to_string(),
            ],
            rust_min_stack: 536_870_912,
            cargo_incremental: false,
            memory_limit_bytes: Some(4_294_967_296),
            target_crates: vec![],
            excluded_crates: vec![],
            toolchain_override: None,
            env_overrides: vec![],
            max_parallel_jobs: 1,
            timeout_secs: 600,
            evidence_refs: vec![],
        }),
        "make:build" => Ok(BuildRecipe {
            schema: BUILD_RECIPE_SCHEMA.to_string(),
            action: "make:build".to_string(),
            target_profile: "debug".to_string(),
            allowed_actions: vec!["make:build".to_string(), "cargo:build".to_string()],
            rust_min_stack: 536_870_912,
            cargo_incremental: false,
            memory_limit_bytes: Some(4_294_967_296),
            target_crates: vec![],
            excluded_crates: vec![],
            toolchain_override: None,
            env_overrides: vec![],
            max_parallel_jobs: 1,
            timeout_secs: 600,
            evidence_refs: vec![],
        }),
        "make:build-release" => Ok(BuildRecipe {
            schema: BUILD_RECIPE_SCHEMA.to_string(),
            action: "make:build-release".to_string(),
            target_profile: "release".to_string(),
            allowed_actions: vec!["make:build-release".to_string(), "cargo:build".to_string()],
            rust_min_stack: 536_870_912,
            cargo_incremental: false,
            memory_limit_bytes: Some(8_589_934_592),
            target_crates: vec![],
            excluded_crates: vec![],
            toolchain_override: None,
            env_overrides: vec![],
            max_parallel_jobs: 1,
            timeout_secs: 600,
            evidence_refs: vec![],
        }),
        unsupported => Err(format!("unsupported_canonical_build_action:{unsupported}")),
    }
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
    fn check_recipe_sets_deterministic_environment_and_stack() {
        let recipe = build_recipe("make:check").expect("recipe");
        assert_eq!(recipe.action, "make:check");
        assert_eq!(recipe.rust_min_stack, 536_870_912);
        assert!(!recipe.cargo_incremental);
        assert!(recipe.allowed_actions.contains(&"cargo:check".to_string()));
        assert!(validate_build_recipe(&recipe).is_ok());
    }

    #[test]
    fn release_recipe_sets_release_profile() {
        let recipe = build_recipe("make:build-release").expect("release recipe");
        assert_eq!(recipe.target_profile, "release");
        assert_eq!(recipe.rust_min_stack, 536_870_912);
        assert!(!recipe.cargo_incremental);
        assert!(validate_build_recipe(&recipe).is_ok());
    }

    #[test]
    fn rejects_unsupported_action() {
        assert!(build_recipe("custom:hack").is_err());
    }
}
