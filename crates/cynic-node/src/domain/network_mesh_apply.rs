use serde::Deserialize;

#[derive(Debug, Clone)]
pub(crate) struct MeshApplyBundle {
    pub(crate) manifest: MeshBundleManifest,
    pub(crate) plan: MeshPlanFile,
    pub(crate) complete: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MeshBundleManifest {
    pub(crate) schema: String,
    pub(crate) status: String,
    pub(crate) provider: String,
    pub(crate) bundle_files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MeshPlanFile {
    pub(crate) provider: String,
    pub(crate) cidr: Option<String>,
    pub(crate) desired_peer_count: usize,
    pub(crate) desired_peers: Vec<MeshPlanPeerFile>,
    pub(crate) routes: Vec<MeshPlanRouteFile>,
    pub(crate) dns: Vec<MeshPlanDnsRecordFile>,
    pub(crate) acl: Vec<MeshPlanAclRuleFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MeshPlanPeerFile {
    pub(crate) id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MeshPlanRouteFile {
    pub(crate) destination: String,
    pub(crate) via_peer: String,
    pub(crate) provider: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MeshPlanDnsRecordFile {
    pub(crate) name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MeshPlanAclRuleFile {}
