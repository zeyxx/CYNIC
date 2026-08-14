use serde::{Deserialize, Serialize};

pub const SURFACE_REGISTRY_SCHEMA: &str = "cynic.surface_registry.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceRegistryDocument {
    pub schema: String,
    pub surfaces: Vec<CanonicalCliSurface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalCliSurface {
    pub id: String,
    pub layer: String,
    pub owner_crate: String,
    pub adapter_module: String,
    pub domain_owner: String,
    pub owner_modules: Vec<String>,
    pub owner_call_markers: Vec<String>,
    pub responsibility: String,
    pub mutation_policy: String,
    pub canonical_entrypoints: Vec<String>,
    pub evidence: Vec<String>,
    pub anti_responsibilities: Vec<String>,
}
