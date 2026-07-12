use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostSubstrateDomain {
    Principal,
    Privilege,
    Packages,
    Filesystem,
    Services,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceStatus {
    Planned,
    Active,
    Degraded,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceContract {
    pub surface: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstrateEvidence {
    pub id: String,
    pub status: SurfaceStatus,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstrateBundleFile {
    pub logical_name: String,
    pub install_path: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstrateDomainContract {
    pub domain: HostSubstrateDomain,
    pub status: SurfaceStatus,
    pub required_surfaces: Vec<SurfaceContract>,
    pub evidences: Vec<SubstrateEvidence>,
}
