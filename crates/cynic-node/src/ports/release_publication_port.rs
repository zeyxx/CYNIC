use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

use cynic_protocol::{
    RuntimeArtifactSource, RuntimeReleasePublicationManifest, SignedReleaseBasketPlan,
    SignedReleaseBasketReadBack,
};

#[derive(Debug)]
pub struct PackagedReleaseBasket {
    pub path: PathBuf,
    pub digest: String,
    pub size_bytes: u64,
    pub(crate) _retention: tempfile::TempDir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetachedReleaseSignature {
    pub path: PathBuf,
    pub digest: String,
    pub size_bytes: u64,
    pub signer_identity: String,
    pub namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePublicationTargets {
    pub basket: RuntimeArtifactSource,
    pub detached_signature: RuntimeArtifactSource,
    pub manifest: RuntimeArtifactSource,
}

pub type ReleasePublicationFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

pub trait ReleaseBasketPackagingPort: Send + Sync {
    fn package<'a>(
        &'a self,
        plan: &'a SignedReleaseBasketPlan,
        workspace_root: &'a Path,
        workload_id: &'a str,
    ) -> ReleasePublicationFuture<'a, PackagedReleaseBasket>;
}

pub trait ReleaseBasketSigningPort: Send + Sync {
    fn sign<'a>(
        &'a self,
        basket: &'a PackagedReleaseBasket,
    ) -> ReleasePublicationFuture<'a, DetachedReleaseSignature>;
}

pub trait ReleaseArtifactPublicationPort: Send + Sync {
    fn targets(&self, release_id: &str) -> Result<ReleasePublicationTargets, String>;

    fn publish<'a>(
        &'a self,
        node_id: &'a str,
        workload_id: &'a str,
        basket: &'a PackagedReleaseBasket,
        signature: &'a DetachedReleaseSignature,
        manifest: &'a RuntimeReleasePublicationManifest,
        targets: &'a ReleasePublicationTargets,
        observed_at_ms: u64,
    ) -> ReleasePublicationFuture<'a, SignedReleaseBasketReadBack>;
}
