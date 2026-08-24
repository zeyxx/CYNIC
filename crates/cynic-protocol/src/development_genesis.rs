use serde::{Deserialize, Serialize};

use crate::{
    SourceMaterializationObservation, SourceMaterializationPlan, SourceMaterializationRequest,
    plan_source_materialization,
};

pub const DEVELOPMENT_GENESIS_BUILD_REQUEST_SCHEMA: &str =
    "cynic.development_genesis.build_request.v1";
pub const DEVELOPMENT_GENESIS_BUILD_ADMISSION_SCHEMA: &str =
    "cynic.development_genesis.build_admission.v1";
pub const DEVELOPMENT_GENESIS_ALLOWED_CAPABILITIES: &[&str] = &["build_artifact"];

/// The only commands a DevelopmentGenesis builder may execute. This is a
/// protocol allowlist, not an argv transport: adapters map these variants to
/// their canonical argv without accepting caller-supplied command strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevelopmentGenesisAction {
    MakeCheck,
    BuildCli,
    BuildMcp,
    Arbitrary { argv: Vec<String> },
}

/// A one-shot, non-production request for an isolated builder. It contains no
/// install, publish, runtime-state, or signature authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevelopmentGenesisBuildRequest {
    pub schema: String,
    pub request_id: String,
    pub source_commit: String,
    pub source_tree: String,
    pub source_authority_ref: String,
    pub builder_image: String,
    pub expires_at_unix_s: u64,
    pub allowed_actions: Vec<DevelopmentGenesisAction>,
    pub requested_capabilities: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevelopmentGenesisBuildAdmission {
    pub schema: String,
    pub status: String,
    pub request_id: String,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevelopmentGenesisBuildPlan {
    pub schema: String,
    pub status: String,
    pub request_id: String,
    pub request: DevelopmentGenesisBuildRequest,
    pub admission: DevelopmentGenesisBuildAdmission,
    pub source_materialization: Option<SourceMaterializationPlan>,
    pub argv: Vec<Vec<String>>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
}

pub fn admit_development_genesis_build(
    request: &DevelopmentGenesisBuildRequest,
    now_unix_s: u64,
) -> DevelopmentGenesisBuildAdmission {
    let mut blockers = Vec::new();
    if request.schema != DEVELOPMENT_GENESIS_BUILD_REQUEST_SCHEMA {
        blockers.push("development_genesis_request_schema_mismatch".to_string());
    }
    if request.request_id.trim().is_empty() {
        blockers.push("development_genesis_request_id_missing".to_string());
    }
    if !is_git_object_id(&request.source_commit) {
        blockers.push("development_genesis_source_commit_invalid".to_string());
    }
    if !is_git_object_id(&request.source_tree) {
        blockers.push("development_genesis_source_tree_invalid".to_string());
    }
    if request.source_authority_ref.trim().is_empty() {
        blockers.push("development_genesis_source_authority_missing".to_string());
    }
    if !is_pinned_oci_image(&request.builder_image) {
        blockers.push("development_genesis_builder_image_unpinned".to_string());
    }
    if request.expires_at_unix_s <= now_unix_s {
        blockers.push("development_genesis_request_expired".to_string());
    }
    if request.allowed_actions.is_empty()
        || request
            .allowed_actions
            .iter()
            .any(|action| !is_allowed_action(action))
    {
        blockers.push("development_genesis_action_not_allowed".to_string());
    }
    if !request
        .requested_capabilities
        .iter()
        .any(|capability| capability == "build_artifact")
    {
        blockers.push("development_genesis_build_artifact_capability_missing".to_string());
    }
    for capability in &request.requested_capabilities {
        if !DEVELOPMENT_GENESIS_ALLOWED_CAPABILITIES.contains(&capability.as_str()) {
            blockers.push(format!(
                "development_genesis_capability_not_allowed:{capability}"
            ));
        }
    }
    if !request
        .evidence_refs
        .iter()
        .any(|reference| valid_source_capsule_reference(reference))
    {
        blockers.push("development_genesis_source_capsule_evidence_missing".to_string());
    }
    blockers.sort();
    blockers.dedup();

    DevelopmentGenesisBuildAdmission {
        schema: DEVELOPMENT_GENESIS_BUILD_ADMISSION_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            "admitted"
        } else {
            "blocked"
        }
        .to_string(),
        request_id: request.request_id.clone(),
        blockers,
    }
}

pub fn plan_development_genesis_build(
    request: &DevelopmentGenesisBuildRequest,
    source_request: &SourceMaterializationRequest,
    source_observation: &SourceMaterializationObservation,
    now_unix_s: u64,
) -> DevelopmentGenesisBuildPlan {
    let admission = admit_development_genesis_build(request, now_unix_s);
    if admission.status != "admitted" {
        return DevelopmentGenesisBuildPlan {
            schema: "cynic.development_genesis.build_plan.v1".to_string(),
            status: "blocked".to_string(),
            request_id: request.request_id.clone(),
            request: request.clone(),
            blockers: admission.blockers.clone(),
            unknowns: Vec::new(),
            admission,
            source_materialization: None,
            argv: Vec::new(),
        };
    }

    let source_plan = plan_source_materialization(
        source_request,
        source_observation,
        now_unix_s.saturating_mul(1_000),
    );
    let mut blockers = source_plan.blockers.clone();
    let mut unknowns = source_plan.unknowns.clone();
    if source_request.source_commit != request.source_commit {
        blockers.push("development_genesis_source_commit_mismatch".to_string());
    }
    if source_request.source_tree.as_deref() != Some(request.source_tree.as_str()) {
        blockers.push("development_genesis_source_tree_mismatch".to_string());
    }
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference == &format!("source-capsule:{}", source_request.snapshot_digest))
    {
        blockers.push("development_genesis_source_capsule_mismatch".to_string());
    }
    match source_plan.status.as_str() {
        "converged" => {}
        "unknown" => {
            unknowns.push("development_genesis_source_materialization_unknown".to_string())
        }
        status => blockers.push(format!(
            "development_genesis_source_materialization_not_converged:{status}"
        )),
    }
    let argv = development_genesis_canonical_argv(&request.allowed_actions).unwrap_or_else(|| {
        blockers.push("development_genesis_canonical_argv_missing".to_string());
        Vec::new()
    });
    blockers.sort();
    blockers.dedup();
    unknowns.sort();
    unknowns.dedup();
    let status = if !blockers.is_empty() {
        "blocked"
    } else if !unknowns.is_empty() {
        "unknown"
    } else {
        "ready"
    };

    DevelopmentGenesisBuildPlan {
        schema: "cynic.development_genesis.build_plan.v1".to_string(),
        status: status.to_string(),
        request_id: request.request_id.clone(),
        request: request.clone(),
        admission,
        source_materialization: Some(source_plan),
        argv,
        blockers,
        unknowns,
    }
}

fn is_allowed_action(action: &DevelopmentGenesisAction) -> bool {
    matches!(
        action,
        DevelopmentGenesisAction::MakeCheck
            | DevelopmentGenesisAction::BuildCli
            | DevelopmentGenesisAction::BuildMcp
    )
}

/// Projects an admitted action set into the only argv sequences a
/// DevelopmentGenesis executor may receive. Callers never supply argv directly.
pub fn development_genesis_canonical_argv(
    actions: &[DevelopmentGenesisAction],
) -> Option<Vec<Vec<String>>> {
    (!actions.is_empty())
        .then(|| {
            actions
                .iter()
                .map(|action| match action {
                    DevelopmentGenesisAction::MakeCheck => {
                        Some(vec!["make".to_string(), "check".to_string()])
                    }
                    DevelopmentGenesisAction::BuildCli => Some(vec![
                        "cargo".to_string(),
                        "build".to_string(),
                        "--release".to_string(),
                        "-p".to_string(),
                        "cynic-cli".to_string(),
                    ]),
                    DevelopmentGenesisAction::BuildMcp => Some(vec![
                        "cargo".to_string(),
                        "build".to_string(),
                        "--release".to_string(),
                        "-p".to_string(),
                        "cynic-mcp".to_string(),
                    ]),
                    DevelopmentGenesisAction::Arbitrary { .. } => None,
                })
                .collect()
        })
        .flatten()
}

fn is_git_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_pinned_oci_image(value: &str) -> bool {
    value
        .rsplit_once("@sha256:")
        .is_some_and(|(image, digest)| {
            !image.trim().is_empty()
                && digest.len() == 64
                && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

fn valid_source_capsule_reference(value: &str) -> bool {
    value
        .strip_prefix("source-capsule:sha256:")
        .is_some_and(|digest| {
            digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        SOURCE_MATERIALIZATION_FORMAT, SOURCE_MATERIALIZATION_OBSERVATION_SCHEMA,
        SOURCE_MATERIALIZATION_REQUEST_SCHEMA, SecurityEvidenceState,
        SourceMaterializationObservation, SourceMaterializationRequest,
    };

    fn request() -> DevelopmentGenesisBuildRequest {
        DevelopmentGenesisBuildRequest {
            schema: DEVELOPMENT_GENESIS_BUILD_REQUEST_SCHEMA.to_string(),
            request_id: "development-genesis-builder-1".to_string(),
            source_commit: "a".repeat(40),
            source_tree: "b".repeat(40),
            source_authority_ref: "forgejo:cynic@aaaaaaaa".to_string(),
            builder_image: "docker.io/library/rust@sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_string(),
            expires_at_unix_s: 200,
            allowed_actions: vec![
                DevelopmentGenesisAction::MakeCheck,
                DevelopmentGenesisAction::BuildCli,
                DevelopmentGenesisAction::BuildMcp,
            ],
            requested_capabilities: vec!["build_artifact".to_string()],
            evidence_refs: vec!["source-capsule:sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".to_string()],
        }
    }

    fn source_request() -> SourceMaterializationRequest {
        SourceMaterializationRequest {
            schema: SOURCE_MATERIALIZATION_REQUEST_SCHEMA.to_string(),
            release_id: "development-genesis-builder-1".to_string(),
            repository_id: "cynic".to_string(),
            source_commit: "a".repeat(40),
            source_tree: Some("b".repeat(40)),
            snapshot_digest: format!("sha256:{}", "d".repeat(64)),
            snapshot_format: SOURCE_MATERIALIZATION_FORMAT.to_string(),
            target_node_id: "cynic-builder".to_string(),
            target_relative_path: "source/development-genesis-builder-1".to_string(),
            required_principal: "cynic".to_string(),
            retention_until_unix_ms: 200_000,
        }
    }

    fn source_observation() -> SourceMaterializationObservation {
        SourceMaterializationObservation {
            schema: SOURCE_MATERIALIZATION_OBSERVATION_SCHEMA.to_string(),
            observed_at_unix_ms: 100_000,
            target_node_id: "cynic-builder".to_string(),
            target_role: Some("build_runner".to_string()),
            node_identity: SecurityEvidenceState::Proven,
            running_as_required_principal: SecurityEvidenceState::Proven,
            snapshot_present: true,
            snapshot_digest_read_back: Some(format!("sha256:{}", "d".repeat(64))),
            snapshot_format_read_back: Some(SOURCE_MATERIALIZATION_FORMAT.to_string()),
            target_exists: true,
            target_is_isolated: SecurityEvidenceState::Proven,
            observed_commit: Some("a".repeat(40)),
            observed_tree: Some("b".repeat(40)),
            observed_snapshot_digest: Some(format!("sha256:{}", "d".repeat(64))),
            evidence_refs: vec!["source-materialization:read-back".to_string()],
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn bounded_development_genesis_request_is_admitted() {
        let admission = admit_development_genesis_build(&request(), 100);
        assert_eq!(admission.status, "admitted");
        assert!(admission.blockers.is_empty());
    }

    #[test]
    fn expired_development_genesis_request_is_blocked() {
        let admission = admit_development_genesis_build(&request(), 200);
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"development_genesis_request_expired".to_string())
        );
    }

    #[test]
    fn unpinned_image_and_arbitrary_action_are_blocked() {
        let mut request = request();
        request.builder_image = "rust:latest".to_string();
        request
            .allowed_actions
            .push(DevelopmentGenesisAction::Arbitrary {
                argv: vec!["sh".to_string(), "-c".to_string(), "id".to_string()],
            });
        request
            .requested_capabilities
            .push("runtime_install".to_string());

        let admission = admit_development_genesis_build(&request, 100);
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"development_genesis_builder_image_unpinned".to_string())
        );
        assert!(
            admission
                .blockers
                .contains(&"development_genesis_action_not_allowed".to_string())
        );
        assert!(
            admission.blockers.contains(
                &"development_genesis_capability_not_allowed:runtime_install".to_string()
            )
        );
    }

    #[test]
    fn admitted_actions_have_one_canonical_argv_projection() {
        assert_eq!(
            development_genesis_canonical_argv(&[
                DevelopmentGenesisAction::MakeCheck,
                DevelopmentGenesisAction::BuildCli,
                DevelopmentGenesisAction::BuildMcp,
            ]),
            Some(vec![
                vec!["make".to_string(), "check".to_string()],
                vec![
                    "cargo".to_string(),
                    "build".to_string(),
                    "--release".to_string(),
                    "-p".to_string(),
                    "cynic-cli".to_string(),
                ],
                vec![
                    "cargo".to_string(),
                    "build".to_string(),
                    "--release".to_string(),
                    "-p".to_string(),
                    "cynic-mcp".to_string(),
                ],
            ])
        );
        assert_eq!(
            development_genesis_canonical_argv(&[DevelopmentGenesisAction::Arbitrary {
                argv: vec!["sh".to_string(), "-c".to_string(), "id".to_string()],
            }]),
            None
        );
    }

    #[test]
    fn converged_source_and_admitted_request_produce_a_ready_build_plan() {
        let plan = plan_development_genesis_build(
            &request(),
            &source_request(),
            &source_observation(),
            100,
        );

        assert_eq!(plan.status, "ready");
        assert!(plan.blockers.is_empty());
        assert_eq!(
            plan.argv,
            development_genesis_canonical_argv(&request().allowed_actions).unwrap()
        );
    }
}
