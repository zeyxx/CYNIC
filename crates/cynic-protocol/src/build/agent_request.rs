use serde::{Deserialize, Serialize};

pub const BUILD_AGENT_REQUEST_SCHEMA: &str = "cynic.build.agent_request.v3";
pub const BUILD_AGENT_REQUEST_ADMISSION_SCHEMA: &str = "cynic.build.agent_request_admission.v1";
pub const BUILD_SOURCE_REFERENCE_SCHEMA: &str = "cynic.build.source_reference.v1";
pub const BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA: &str =
    "cynic.build.release_publication_intent.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildSourceReference {
    pub schema: String,
    pub repository_id: String,
    pub transport_uri: String,
    pub revision: String,
    pub authority_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildAgentRequest {
    pub schema: String,
    pub request_id: String,
    pub requested_by: String,
    pub source_commit: String,
    pub source: BuildSourceReference,
    pub action: String,
    pub desired_role: String,
    pub mutation_authorized: bool,
    pub proof_requirements: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_publication: Option<BuildReleasePublicationIntent>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildReleasePublicationIntent {
    pub schema: String,
    pub release_id: String,
    pub release_epoch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildAgentRequestAdmission {
    pub schema: String,
    pub status: String,
    pub request_id: String,
    pub source_commit: String,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub fn plan_build_agent_request(request: &BuildAgentRequest) -> BuildAgentRequestAdmission {
    assess(request, false)
}

pub fn admit_build_agent_request(request: &BuildAgentRequest) -> BuildAgentRequestAdmission {
    assess(request, true)
}

fn assess(
    request: &BuildAgentRequest,
    require_mutation_authority: bool,
) -> BuildAgentRequestAdmission {
    let mut blockers = Vec::new();
    if request.schema != BUILD_AGENT_REQUEST_SCHEMA {
        blockers.push("build_agent_request_schema_mismatch".to_string());
    }
    if request.request_id.trim().is_empty() {
        blockers.push("build_agent_request_id_missing".to_string());
    }
    if request.requested_by.trim().is_empty() {
        blockers.push("build_agent_request_principal_missing".to_string());
    }
    if !is_commit_id(&request.source_commit) {
        blockers.push("build_agent_request_source_commit_invalid".to_string());
    }
    blockers.extend(validate_build_source_reference(&request.source));
    if request.source.revision != request.source_commit {
        blockers.push("build_source_revision_mismatch".to_string());
    }
    if !matches!(
        request.action.as_str(),
        "make:check" | "make:build" | "make:build-release"
    ) {
        blockers.push("build_agent_request_action_not_allowed".to_string());
    }
    if request.desired_role != "build_runner" {
        blockers.push("build_agent_request_role_mismatch".to_string());
    }
    if require_mutation_authority && !request.mutation_authorized {
        blockers.push("build_agent_request_dispatch_not_authorized".to_string());
    }
    if request.proof_requirements.is_empty() {
        blockers.push("build_agent_request_proof_requirements_missing".to_string());
    }
    match (
        request.action.as_str(),
        request.release_publication.as_ref(),
    ) {
        ("make:build-release", Some(intent)) => {
            blockers.extend(validate_release_publication_intent(intent));
            for proof in [
                "release_binary_artifact_read_back",
                "signed_release_basket_read_back",
            ] {
                if !request.proof_requirements.iter().any(|item| item == proof) {
                    blockers.push(format!("build_release_required_proof_missing:{proof}"));
                }
            }
        }
        ("make:build-release", None) => {
            blockers.push("build_release_publication_intent_missing".to_string());
        }
        (_, Some(_)) => blockers.push("build_release_publication_intent_unexpected".to_string()),
        (_, None) => {}
    }
    if request.evidence_refs.is_empty() {
        blockers.push("build_agent_request_source_evidence_missing".to_string());
    }
    BuildAgentRequestAdmission {
        schema: BUILD_AGENT_REQUEST_ADMISSION_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            if require_mutation_authority {
                "admitted"
            } else {
                "planned"
            }
        } else {
            "blocked"
        }
        .to_string(),
        request_id: request.request_id.clone(),
        source_commit: request.source_commit.clone(),
        blockers,
        evidence_refs: request.evidence_refs.clone(),
    }
}

pub fn validate_release_publication_intent(intent: &BuildReleasePublicationIntent) -> Vec<String> {
    let mut blockers = Vec::new();
    if intent.schema != BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA {
        blockers.push("build_release_publication_intent_schema_mismatch".to_string());
    }
    if !valid_release_publication_id(&intent.release_id) {
        blockers.push("build_release_publication_id_invalid".to_string());
    }
    if intent.release_epoch == 0 {
        blockers.push("build_release_publication_epoch_invalid".to_string());
    }
    blockers
}

pub fn valid_release_publication_id(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.len() <= 120
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

pub fn validate_build_source_reference(source: &BuildSourceReference) -> Vec<String> {
    let mut blockers = Vec::new();
    if source.schema != BUILD_SOURCE_REFERENCE_SCHEMA {
        blockers.push("build_source_reference_schema_mismatch".to_string());
    }
    if !valid_repository_id(&source.repository_id) {
        blockers.push("build_source_repository_id_invalid".to_string());
    }
    if source.transport_uri.trim().is_empty() {
        blockers.push("build_source_transport_missing".to_string());
    } else if source_contains_inline_secret(&source.transport_uri) {
        blockers.push("build_source_transport_contains_credentials".to_string());
    }
    if !is_commit_id(&source.revision) {
        blockers.push("build_source_revision_invalid".to_string());
    }
    if source.authority_ref.trim().is_empty() {
        blockers.push("build_source_authority_missing".to_string());
    }
    blockers
}

fn is_commit_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_repository_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn source_contains_inline_secret(source: &str) -> bool {
    let Some((scheme, remainder)) = source.split_once("://") else {
        return true;
    };
    let authority = remainder.split('/').next().unwrap_or_default();
    let Some((userinfo, _)) = authority.rsplit_once('@') else {
        return false;
    };
    scheme != "ssh" || userinfo.is_empty() || userinfo.contains(':')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(mutation_authorized: bool) -> BuildAgentRequest {
        BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "validate-abc".to_string(),
            requested_by: "codex-session-1".to_string(),
            source_commit: "a".repeat(40),
            source: BuildSourceReference {
                schema: BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".to_string(),
                transport_uri: "ssh://forgejo@forgejo.synapse.cynic:22/owner/CYNIC.git".to_string(),
                revision: "a".repeat(40),
                authority_ref: "infra/repo-remotes.json:forgejo".to_string(),
            },
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized,
            proof_requirements: vec![
                "canonical_gate_report".to_string(),
                "source_commit_read_back".to_string(),
            ],
            release_publication: None,
            evidence_refs: vec!["forgejo:refs/heads/change@aaaaaaaa".to_string()],
        }
    }

    #[test]
    fn planning_does_not_require_dispatch_authority() {
        assert_eq!(plan_build_agent_request(&request(false)).status, "planned");
    }

    #[test]
    fn dispatch_requires_explicit_authority() {
        let admission = admit_build_agent_request(&request(false));
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"build_agent_request_dispatch_not_authorized".to_string())
        );
    }

    #[test]
    fn shell_or_arbitrary_build_actions_are_not_admitted() {
        let mut request = request(true);
        request.action = "bash -c make check".to_string();
        assert!(
            admit_build_agent_request(&request)
                .blockers
                .contains(&"build_agent_request_action_not_allowed".to_string())
        );
    }

    #[test]
    fn canonical_artifact_build_is_admissible() {
        let mut request = request(true);
        request.action = "make:build".to_string();
        assert_eq!(admit_build_agent_request(&request).status, "admitted");
    }

    #[test]
    fn canonical_release_build_is_admissible() {
        let mut request = request(true);
        request.action = "make:build-release".to_string();
        request.proof_requirements = vec![
            "release_binary_artifact_read_back".to_string(),
            "signed_release_basket_read_back".to_string(),
        ];
        request.release_publication = Some(BuildReleasePublicationIntent {
            schema: BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA.to_string(),
            release_id: "release-a".to_string(),
            release_epoch: 1,
        });
        assert_eq!(admit_build_agent_request(&request).status, "admitted");
    }

    #[test]
    fn release_build_without_publication_intent_is_blocked() {
        let mut request = request(true);
        request.action = "make:build-release".to_string();
        let admission = admit_build_agent_request(&request);

        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"build_release_publication_intent_missing".to_string())
        );
    }

    #[test]
    fn source_revision_and_transport_are_bound_without_inline_secrets() {
        let mut mismatched = request(true);
        mismatched.source.revision = "b".repeat(40);
        assert!(
            admit_build_agent_request(&mismatched)
                .blockers
                .contains(&"build_source_revision_mismatch".to_string())
        );

        let mut secret = request(true);
        secret.source.transport_uri = "https://operator:secret@example.invalid/CYNIC.git".into();
        assert!(
            admit_build_agent_request(&secret)
                .blockers
                .contains(&"build_source_transport_contains_credentials".to_string())
        );
    }
}
