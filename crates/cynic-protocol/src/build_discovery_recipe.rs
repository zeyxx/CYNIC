use serde::{Deserialize, Serialize};

pub const BUILD_DISCOVERY_RECIPE_SCHEMA: &str = "cynic.build_discovery_recipe.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildDiscoveryRecipe {
    pub schema: String,
    pub id: String,
    pub action: String,
    pub crate_name: String,
    pub target_kind: String,
    pub target_name: String,
    pub argv: Vec<String>,
}
