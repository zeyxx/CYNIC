use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA, BuildReleasePublicationIntent,
    RUNTIME_SEED_BUNDLE_MARKER_SCHEMA, RuntimeArtifactSource, RuntimeArtifactTransportKind,
    RuntimeSeedBundleMarker, valid_release_publication_id,
};

pub const SIGNED_RELEASE_BASKET_REQUEST_SCHEMA: &str = "cynic.release.signed_basket_request.v2";
pub const SIGNED_RELEASE_BASKET_PLAN_SCHEMA: &str = "cynic.release.signed_basket_plan.v2";
pub const SIGNED_RELEASE_BASKET_READ_BACK_SCHEMA: &str = "cynic.release.signed_basket_read_back.v2";
pub const RUNTIME_RELEASE_PUBLICATION_MANIFEST_SCHEMA: &str =
    "cynic.release.publication_manifest.v2";
pub const RUNTIME_RELEASE_SIGNATURE_NAMESPACE: &str = "cynic-runtime-seed";
pub const RELEASE_PUBLICATION_DESIRED_STATE_SCHEMA: &str =
    "cynic.release_publication.desired_state.v1";

const RELEASE_BINARIES: [&str; 4] = ["cynic", "cynic-kernel", "cynic-mcp", "cynic-node"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleasePublicationDesiredState {
    pub schema: String,
    pub storage_role: String,
    pub service_name: String,
    pub protocol: String,
    pub authentication_kind: String,
    pub credential_path: String,
    pub signing_key_path: String,
    pub signer_identity: String,
    pub signature_namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedReleaseBasketRequest {
    pub schema: String,
    pub node_id: String,
    pub workload_id: String,
    pub source_commit: String,
    pub build_proof_digest: String,
    pub registry_digest: String,
    pub publication: BuildReleasePublicationIntent,
    pub artifacts: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedReleaseBasketPlan {
    pub schema: String,
    pub status: String,
    pub marker: RuntimeSeedBundleMarker,
    pub artifacts: BTreeMap<String, String>,
    #[serde(default)]
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeReleasePublicationManifest {
    pub schema: String,
    pub release_id: String,
    pub release_epoch: u64,
    pub source_commit: String,
    pub build_proof_digest: String,
    pub registry_digest: String,
    pub basket_digest: String,
    pub basket_source: RuntimeArtifactSource,
    pub detached_signature_digest: String,
    pub detached_signature_source: RuntimeArtifactSource,
    pub signer_identity: String,
    pub signature_namespace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedReleaseBasketReadBack {
    pub schema: String,
    pub node_id: String,
    pub workload_id: String,
    pub manifest: RuntimeReleasePublicationManifest,
    pub manifest_digest: String,
    pub manifest_source: RuntimeArtifactSource,
    pub basket_size_bytes: u64,
    pub detached_signature_size_bytes: u64,
    pub observed_at_ms: u64,
}

pub fn plan_signed_release_basket(request: &SignedReleaseBasketRequest) -> SignedReleaseBasketPlan {
    let mut blockers = validate_request(request);
    blockers.sort();
    blockers.dedup();
    SignedReleaseBasketPlan {
        schema: SIGNED_RELEASE_BASKET_PLAN_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            "planned"
        } else {
            "blocked"
        }
        .to_string(),
        marker: RuntimeSeedBundleMarker {
            schema: RUNTIME_SEED_BUNDLE_MARKER_SCHEMA.to_string(),
            release_id: request.publication.release_id.clone(),
            release_epoch: request.publication.release_epoch,
            source_commit: request.source_commit.clone(),
            build_proof_digest: request.build_proof_digest.clone(),
            registry_digest: request.registry_digest.clone(),
        },
        artifacts: request.artifacts.clone(),
        blockers,
    }
}

pub fn validate_release_publication_desired_state(
    desired: &ReleasePublicationDesiredState,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if desired.schema != RELEASE_PUBLICATION_DESIRED_STATE_SCHEMA {
        blockers.push("release_publication_desired_state_schema_mismatch".to_string());
    }
    if desired.storage_role.trim().is_empty()
        || desired.service_name.trim().is_empty()
        || desired.signer_identity.trim().is_empty()
    {
        blockers.push("release_publication_desired_identity_missing".to_string());
    }
    if desired.protocol != "generic_https_put_get" || desired.authentication_kind != "bearer_file" {
        blockers.push("release_publication_desired_adapter_unsupported".to_string());
    }
    if desired.signature_namespace != RUNTIME_RELEASE_SIGNATURE_NAMESPACE {
        blockers.push("release_publication_desired_signature_namespace_mismatch".to_string());
    }
    if [&desired.credential_path, &desired.signing_key_path]
        .iter()
        .any(|path| !std::path::Path::new(path).is_absolute())
    {
        blockers.push("release_publication_desired_machine_path_not_absolute".to_string());
    }
    blockers
}

pub fn validate_signed_release_basket_read_back(
    read_back: &SignedReleaseBasketReadBack,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if read_back.schema != SIGNED_RELEASE_BASKET_READ_BACK_SCHEMA {
        blockers.push("signed_release_read_back_schema_mismatch".to_string());
    }
    if read_back.node_id.is_empty() || read_back.workload_id.is_empty() {
        blockers.push("signed_release_read_back_identity_missing".to_string());
    }
    let manifest = &read_back.manifest;
    if manifest.schema != RUNTIME_RELEASE_PUBLICATION_MANIFEST_SCHEMA {
        blockers.push("signed_release_manifest_schema_mismatch".to_string());
    }
    if !valid_release_publication_id(&manifest.release_id) || manifest.release_epoch == 0 {
        blockers.push("signed_release_manifest_release_identity_invalid".to_string());
    }
    if !valid_source_commit(&manifest.source_commit) {
        blockers.push("signed_release_manifest_source_commit_invalid".to_string());
    }
    for (kind, digest) in [
        ("build_proof", manifest.build_proof_digest.as_str()),
        ("registry", manifest.registry_digest.as_str()),
        ("basket", manifest.basket_digest.as_str()),
        (
            "detached_signature",
            manifest.detached_signature_digest.as_str(),
        ),
        ("manifest", read_back.manifest_digest.as_str()),
    ] {
        if !valid_sha256(digest) {
            blockers.push(format!("signed_release_{kind}_digest_invalid"));
        }
    }
    if crate::digest_json(manifest).ok().as_deref() != Some(read_back.manifest_digest.as_str()) {
        blockers.push("signed_release_manifest_digest_mismatch".to_string());
    }
    for (kind, source) in [
        ("basket", &manifest.basket_source),
        ("detached_signature", &manifest.detached_signature_source),
        ("manifest", &read_back.manifest_source),
    ] {
        if !valid_published_source(source) {
            blockers.push(format!("signed_release_{kind}_source_invalid"));
        }
    }
    if manifest.signer_identity.is_empty()
        || manifest.signature_namespace != RUNTIME_RELEASE_SIGNATURE_NAMESPACE
    {
        blockers.push("signed_release_signer_contract_invalid".to_string());
    }
    if read_back.basket_size_bytes == 0
        || read_back.detached_signature_size_bytes == 0
        || read_back.observed_at_ms == 0
    {
        blockers.push("signed_release_read_back_observation_invalid".to_string());
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn validate_request(request: &SignedReleaseBasketRequest) -> Vec<String> {
    let mut blockers = Vec::new();
    if request.schema != SIGNED_RELEASE_BASKET_REQUEST_SCHEMA {
        blockers.push("signed_release_request_schema_mismatch".to_string());
    }
    if request.node_id.is_empty() || request.workload_id.is_empty() {
        blockers.push("signed_release_request_identity_missing".to_string());
    }
    if request.publication.schema != BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA
        || !valid_release_publication_id(&request.publication.release_id)
        || request.publication.release_epoch == 0
    {
        blockers.push("signed_release_publication_intent_invalid".to_string());
    }
    if !valid_source_commit(&request.source_commit) {
        blockers.push("signed_release_source_commit_invalid".to_string());
    }
    if !valid_sha256(&request.build_proof_digest) {
        blockers.push("signed_release_build_proof_digest_invalid".to_string());
    }
    if !valid_sha256(&request.registry_digest) {
        blockers.push("signed_release_registry_digest_invalid".to_string());
    }
    let names = request
        .artifacts
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if names != RELEASE_BINARIES {
        blockers.push("signed_release_artifact_set_mismatch".to_string());
    }
    if request
        .artifacts
        .values()
        .any(|digest| !valid_sha256(digest))
    {
        blockers.push("signed_release_artifact_digest_invalid".to_string());
    }
    blockers
}

fn valid_published_source(source: &RuntimeArtifactSource) -> bool {
    source.transport == RuntimeArtifactTransportKind::Https
        && source.locator.starts_with("https://")
        && !source.locator.contains('@')
        && !source.locator.contains('?')
        && !source.locator.contains('#')
}

fn valid_source_commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> SignedReleaseBasketRequest {
        SignedReleaseBasketRequest {
            schema: SIGNED_RELEASE_BASKET_REQUEST_SCHEMA.to_string(),
            node_id: "builder-a".to_string(),
            workload_id: "build-a".to_string(),
            source_commit: "a".repeat(40),
            build_proof_digest: format!("sha256:{}", "b".repeat(64)),
            registry_digest: format!("sha256:{}", "e".repeat(64)),
            publication: BuildReleasePublicationIntent {
                schema: BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA.to_string(),
                release_id: "release-a".to_string(),
                release_epoch: 1,
            },
            artifacts: RELEASE_BINARIES
                .into_iter()
                .map(|name| (name.to_string(), format!("sha256:{}", "c".repeat(64))))
                .collect(),
        }
    }

    #[test]
    fn complete_release_input_produces_a_bound_plan() {
        let request = request();
        let plan = plan_signed_release_basket(&request);

        assert_eq!(plan.status, "planned");
        assert_eq!(plan.marker.source_commit, request.source_commit);
        assert_eq!(plan.artifacts, request.artifacts);
    }

    #[test]
    fn incomplete_binary_set_is_blocked() {
        let mut request = request();
        request.artifacts.remove("cynic-mcp");

        assert!(
            plan_signed_release_basket(&request)
                .blockers
                .contains(&"signed_release_artifact_set_mismatch".to_string())
        );
    }

    #[test]
    fn read_back_requires_the_exact_published_manifest_digest() {
        let manifest = RuntimeReleasePublicationManifest {
            schema: RUNTIME_RELEASE_PUBLICATION_MANIFEST_SCHEMA.to_string(),
            release_id: "release-a".to_string(),
            release_epoch: 1,
            source_commit: "a".repeat(40),
            build_proof_digest: format!("sha256:{}", "b".repeat(64)),
            registry_digest: format!("sha256:{}", "e".repeat(64)),
            basket_digest: format!("sha256:{}", "c".repeat(64)),
            basket_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::Https,
                locator: "https://artifacts.cynic/release-a/release-a.tar".to_string(),
            },
            detached_signature_digest: format!("sha256:{}", "d".repeat(64)),
            detached_signature_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::Https,
                locator: "https://artifacts.cynic/release-a/release-a.tar.sig".to_string(),
            },
            signer_identity: "cynic-release".to_string(),
            signature_namespace: RUNTIME_RELEASE_SIGNATURE_NAMESPACE.to_string(),
        };
        let mut read_back = SignedReleaseBasketReadBack {
            schema: SIGNED_RELEASE_BASKET_READ_BACK_SCHEMA.to_string(),
            node_id: "builder-a".to_string(),
            workload_id: "build-a".to_string(),
            manifest_digest: crate::digest_json(&manifest).unwrap(),
            manifest,
            manifest_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::Https,
                locator: "https://artifacts.cynic/release-a/release-a.manifest.json".to_string(),
            },
            basket_size_bytes: 1024,
            detached_signature_size_bytes: 256,
            observed_at_ms: 1,
        };
        assert!(validate_signed_release_basket_read_back(&read_back).is_empty());

        read_back.manifest.release_epoch = 2;
        assert!(
            validate_signed_release_basket_read_back(&read_back)
                .contains(&"signed_release_manifest_digest_mismatch".to_string())
        );
    }
}
