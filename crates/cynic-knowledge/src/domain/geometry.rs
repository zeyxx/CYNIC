use crate::domain::chunk::{AuthorityTier, DocumentChunk};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const WORKSPACE_GEOMETRY_SCHEMA_V1: &str = "cynic.knowledge.workspace_geometry.v1";

/// Géométrie structurelle d'une crate dans le cône d'autorité CYNIC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateGeometryRecord {
    pub name: String,
    pub layer: String,
    pub kind: String,
    pub loc: usize,
    pub depends_on: Vec<String>,
    pub consumers: Vec<String>,
    pub justified: bool,
}

impl CrateGeometryRecord {
    /// Rang d'altitude dans le cône (0 = Fondation L0, 4 = Surface L4).
    pub fn layer_rank(&self) -> u8 {
        match self.layer.as_str() {
            "L0" => 0,
            "L1" => 1,
            "L2" => 2,
            "L3" => 3,
            "L4" => 4,
            _ => 5,
        }
    }

    /// Fan-in : degré entrant (combien de crates s'appuient sur celle-ci).
    pub fn fan_in(&self) -> usize {
        self.consumers.len()
    }

    /// Fan-out : degré sortant (de combien de crates internes celle-ci dépend).
    pub fn fan_out(&self) -> usize {
        self.depends_on.len()
    }

    /// Convertit la fiche géométrique en DocumentChunk indexable par BM25.
    pub fn to_chunk(&self) -> DocumentChunk {
        let hierarchy = vec![
            "Code Geometry".to_string(),
            format!("Layer {} - {}", self.layer, self.kind),
            self.name.clone(),
        ];

        let mut content = format!(
            "Crate: {}\nLayer: {}\nKind: {}\nLOC: {}\nJustified: {}\nFan-In (Consumers): {}\nFan-Out (Depends On): {}\n",
            self.name,
            self.layer,
            self.kind,
            self.loc,
            self.justified,
            self.fan_in(),
            self.fan_out()
        );

        if !self.depends_on.is_empty() {
            content.push_str("Dependencies:\n");
            for dep in &self.depends_on {
                content.push_str(&format!("  - {}\n", dep));
            }
        }

        if !self.consumers.is_empty() {
            content.push_str("Consumers:\n");
            for c in &self.consumers {
                content.push_str(&format!("  - {}\n", c));
            }
        }

        let authority = match self.layer.as_str() {
            "L0" => AuthorityTier::L0Constitution,
            "L1" => AuthorityTier::L1Specification,
            _ => AuthorityTier::L2Design,
        };

        DocumentChunk::new(
            format!("crate://{}", self.name),
            1,
            content.lines().count(),
            hierarchy,
            content,
            authority,
        )
    }
}

/// Vue globale de la géométrie du workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceGeometry {
    pub schema: String,
    pub total_crates: usize,
    pub total_loc: usize,
    pub crates: HashMap<String, CrateGeometryRecord>,
}

impl WorkspaceGeometry {
    pub fn from_records(records: Vec<CrateGeometryRecord>) -> Self {
        let total_crates = records.len();
        let total_loc = records.iter().map(|r| r.loc).sum();
        let mut crates = HashMap::new();

        for r in records {
            crates.insert(r.name.clone(), r);
        }

        Self {
            schema: WORKSPACE_GEOMETRY_SCHEMA_V1.to_string(),
            total_crates,
            total_loc,
            crates,
        }
    }

    /// Liste des crates appartenant à une couche donnée ("L0", "L1", etc.).
    pub fn crates_in_layer(&self, layer: &str) -> Vec<&CrateGeometryRecord> {
        let mut list: Vec<&CrateGeometryRecord> =
            self.crates.values().filter(|c| c.layer == layer).collect();
        list.sort_by(|a, b| b.loc.cmp(&a.loc));
        list
    }
}
