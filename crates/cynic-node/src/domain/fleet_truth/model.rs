use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub role: String,
    pub status: String,
    pub constellation: Option<String>,
    pub aliases: Vec<String>,
    pub host: Option<String>,
    pub proxmox_id: Option<String>,
    #[serde(default)]
    pub energy_profile: BTreeMap<String, Value>,
}

impl Node {
    pub fn identities(&self) -> BTreeSet<String> {
        let mut values = BTreeSet::from([self.name.clone()]);
        values.extend(self.aliases.iter().filter(|v| !v.is_empty()).cloned());
        if let Some(proxmox_id) = &self.proxmox_id
            && !proxmox_id.is_empty()
        {
            values.insert(proxmox_id.clone());
        }
        values
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedResource {
    pub provider: String,
    pub name: String,
    pub status: String,
    pub resource_id: Option<String>,
    pub kind: Option<String>,
    pub constellation: Option<String>,
}

impl ObservedResource {
    pub fn identities(&self) -> BTreeSet<String> {
        let mut values = BTreeSet::from([self.name.clone()]);
        if let Some(resource_id) = &self.resource_id
            && !resource_id.is_empty()
        {
            values.insert(resource_id.clone());
        }
        values
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anomaly {
    pub severity: String,
    pub kind: String,
    pub target: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveNode {
    pub name: String,
    pub role: String,
    pub constellation: Option<String>,
    pub declared_status: String,
    pub observed_status: String,
    pub effective_status: String,
    pub provider: Option<String>,
    pub observed_constellation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetSummary {
    pub status: String,
    pub declared_nodes: usize,
    pub observed_resources: usize,
    pub constellations: ConstellationSummary,
    pub anomaly_count: usize,
    pub critical_anomaly_count: usize,
    pub critical_targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstellationSummary {
    pub declared: BTreeMap<String, usize>,
    pub observed: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnergySummary {
    pub classes: BTreeMap<String, usize>,
    pub missing_profiles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetReport {
    pub summary: FleetSummary,
    pub desired: Vec<Node>,
    pub declared: Vec<Node>,
    pub observed: Vec<ObservedResource>,
    pub effective: Vec<EffectiveNode>,
    pub anomalies: Vec<Anomaly>,
    pub energy: EnergySummary,
}
