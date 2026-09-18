//! Typed bindings for machine-readable release contracts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDefinition {
    pub name: String,
    pub description: String,
    pub command: String,
    pub pass_condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatesCatalog {
    pub mandatory: Vec<GateDefinition>,
    pub optional: Vec<GateDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseGatesContract {
    pub schema: String,
    pub status: String,
    pub gates: GatesCatalog,
}

impl ReleaseGatesContract {
    pub fn parse(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    pub fn mandatory_gate_names(&self) -> Vec<String> {
        self.gates
            .mandatory
            .iter()
            .map(|g| g.name.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayTopologySpec {
    pub sovereign_source: String,
    pub public_projection: String,
    pub relationship: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayActionSpec {
    pub target: String,
    pub credential: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayPolicyContract {
    pub schema: String,
    pub status: String,
    pub topology: RelayTopologySpec,
    pub action: RelayActionSpec,
}

impl RelayPolicyContract {
    pub fn parse(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersioningSchemeSpec {
    pub scheme: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseVersioningContract {
    pub schema: String,
    pub status: String,
    pub versioning: VersioningSchemeSpec,
}

impl ReleaseVersioningContract {
    pub fn parse(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}
