use serde::{Deserialize, Serialize};

pub const BUILD_DISCOVERY_REPORT_SCHEMA: &str = "cynic.build_discovery_report.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostTriple(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Toolchain(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildDiscoveryReport {
    pub schema: String,
    pub toolchain: Toolchain,
    pub host_triple: HostTriple,
    pub targets: Vec<String>,
    pub crates: Vec<DiscoveredCrate>,
    pub observed_at_unix_s: u64,
    pub residual_unknowns: Vec<String>,
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

pub fn discovery_report_schema() -> &'static str {
    BUILD_DISCOVERY_REPORT_SCHEMA
}

/// Pure port for discovery (observe without mutation).
pub trait BuildDiscoveryPort {
    fn discover(&self) -> Result<BuildDiscoveryReport, String>;
}
