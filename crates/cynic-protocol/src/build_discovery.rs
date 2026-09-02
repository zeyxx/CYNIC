use serde::{Deserialize, Serialize};

pub const BUILD_DISCOVERY_REPORT_SCHEMA: &str = "cynic.build_discovery_report.v1";
pub const BUILD_DISCOVERY_RECIPE_SCHEMA: &str = "cynic.build_discovery_recipe.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildDiscoveryReport {
    pub schema: String,
    pub toolchain: String,
    pub host_triple: String,
    pub targets: Vec<String>,
    pub crates: Vec<DiscoveredCrate>,
    pub total_targets: usize,
    pub total_tests: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredCrate {
    pub name: String,
    pub edition: String,
    pub bins: Vec<String>,
    pub libs: Vec<String>,
    pub tests: Vec<String>,
    pub custom_build: bool,
}

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

pub fn discovery_report_schema() -> &'static str {
    BUILD_DISCOVERY_REPORT_SCHEMA
}
