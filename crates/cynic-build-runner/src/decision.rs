//! Pure reconciliation: desired state + observed → BuildPlan.
//! No I/O, no mutation — pure function per CYNIC coherence pattern.
use crate::types::{BuildRequest, BuilderTarget};

#[derive(Debug, Clone)]
pub struct BuildPlan {
    pub src: String,
    pub manifest: String,
    pub packages: Vec<String>,
    pub builder: BuilderTarget,
}
impl BuildRequest {
    pub fn reconcile(&self, discovered: BuilderTarget) -> BuildPlan {
        BuildPlan {
            src: self.manifest.clone(),
            manifest: self.manifest.clone(),
            packages: self.packages.clone(),
            builder: discovered,
        }
    }
}
