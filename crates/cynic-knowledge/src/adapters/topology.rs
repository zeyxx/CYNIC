use crate::domain::geometry::{CrateGeometryRecord, WorkspaceGeometry};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct RawCrateTopology {
    crates: HashMap<String, RawCrateEntry>,
}

#[derive(Debug, Deserialize)]
struct RawCrateEntry {
    #[serde(default)]
    consumers: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    justified: bool,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    layer: String,
    #[serde(default)]
    loc: usize,
}

/// Parseur pur sans I/O du JSON de topologie des crates.
pub fn parse_crate_topology_json(json_str: &str) -> Result<WorkspaceGeometry, serde_json::Error> {
    let raw: RawCrateTopology = serde_json::from_str(json_str)?;
    let records = raw
        .crates
        .into_iter()
        .map(|(name, entry)| CrateGeometryRecord {
            name,
            layer: entry.layer,
            kind: entry.kind,
            loc: entry.loc,
            depends_on: entry.depends_on,
            consumers: entry.consumers,
            justified: entry.justified,
        })
        .collect();

    Ok(WorkspaceGeometry::from_records(records))
}

/// Charge et analyse la géométrie des crates depuis `infra/crate-topology.json`.
pub fn load_workspace_geometry<P: AsRef<Path>>(
    repo_root: P,
) -> Result<WorkspaceGeometry, std::io::Error> {
    let path = repo_root.as_ref().join("infra/crate-topology.json");
    let content = std::fs::read_to_string(path)?;
    parse_crate_topology_json(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
