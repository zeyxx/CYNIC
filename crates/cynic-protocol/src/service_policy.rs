use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceManagerKind {
    Systemd,
    OpenRc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceUnitPolicy {
    #[serde(default)]
    pub service_id: String,
    pub unit_name: String,
    pub enable_on_boot: bool,
    pub restart_mode: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub health_port: Option<u16>,
    #[serde(default)]
    pub process_name: Option<String>,
    #[serde(default)]
    pub supervisor_provider: Option<String>,
    #[serde(default)]
    pub health_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServicePolicy {
    pub manager: ServiceManagerKind,
    pub unit_dir: String,
    pub units: Vec<ServiceUnitPolicy>,
    /// Units this host must NOT run.
    ///
    /// A unit lands here when it was installed from a superseded source
    /// and a corrected one replaced it. The policy has no way to express
    /// "stop running this" for a unit it does not declare, so the stale
    /// copy kept starting at every boot. Measured: a soak unit
    /// installed from a duplicated repository path executed a Cargo-cache
    /// binary that did not know its verb, and failed at every boot while
    /// the corrected copy ran in the user scope.
    ///
    /// The retire operation is bounded by this list: a unit that is not
    /// named here is never disabled, stopped or removed.
    #[serde(default)]
    pub retired_units: Vec<String>,
}
