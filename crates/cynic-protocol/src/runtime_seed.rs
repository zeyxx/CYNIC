use serde::{Deserialize, Serialize};

use crate::SecurityEvidenceState;

pub const RUNTIME_SEED_REQUEST_SCHEMA: &str = "cynic.runtime_seed.request.v4";
pub const RUNTIME_SEED_OBSERVATION_SCHEMA: &str = "cynic.runtime_seed.observation.v4";
pub const RUNTIME_SEED_PLAN_SCHEMA: &str = "cynic.runtime_seed.plan.v4";
pub const RUNTIME_SEED_BUNDLE_MARKER_SCHEMA: &str = "cynic.runtime_seed.bundle_marker.v3";
pub const RUNTIME_SEED_BUNDLE_MARKER_FILE: &str = ".cynic-runtime-bundle.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSeedBundleMarker {
    pub schema: String,
    pub release_id: String,
    pub release_epoch: u64,
    pub source_commit: String,
    pub build_proof_digest: String,
    pub registry_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeArtifactTransportKind {
    LocalFile,
    Https,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeArtifactSource {
    pub transport: RuntimeArtifactTransportKind,
    pub locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSeedRequest {
    pub schema: String,
    pub node_id: String,
    pub release_id: String,
    pub release_epoch: u64,
    pub source_commit: String,
    pub build_proof_digest: String,
    pub basket_digest: String,
    pub registry_digest: String,
    pub basket_source: RuntimeArtifactSource,
    pub basket_path: String,
    pub detached_signature_source: RuntimeArtifactSource,
    pub detached_signature_path: String,
    pub allowed_signers_path: String,
    pub registry_path: String,
    pub signer_identity: String,
    pub signature_namespace: String,
    pub required_seed_contract_revision: u64,
    pub install_root: String,
    pub staged_slot: String,
    pub current_link: String,
    pub previous_link: String,
    pub postflight_argv: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSeedObservation {
    pub schema: String,
    pub observed_at_unix_ms: u64,
    pub node_id: String,
    pub seed_contract_revision: Option<u64>,
    pub trust_root: SecurityEvidenceState,
    pub signature: SecurityEvidenceState,
    pub basket_present: bool,
    pub detached_signature_present: bool,
    pub basket_digest_read_back: Option<String>,
    pub current_release_id: Option<String>,
    pub current_release_epoch: Option<u64>,
    pub current_source_commit: Option<String>,
    pub current_build_proof_digest: Option<String>,
    pub current_basket_digest: Option<String>,
    pub current_registry_digest: Option<String>,
    pub installed_registry_digest: Option<String>,
    pub previous_release_id: Option<String>,
    pub slot_layout: SecurityEvidenceState,
    pub postflight: SecurityEvidenceState,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSeedAction {
    StageReleaseArtifacts,
    VerifySignatureAndDigest,
    ActivateStagedSlot,
    RestoreInstalledRegistry,
    RunPostflight,
    CommitPreviousPointer,
    RollbackToPrevious,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSeedPlan {
    pub schema: String,
    pub status: String,
    pub node_id: String,
    pub release_id: String,
    pub release_epoch: u64,
    pub source_commit: String,
    pub build_proof_digest: String,
    pub actions: Vec<RuntimeSeedAction>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub rollback_intent: String,
}

pub fn reconcile_runtime_seed(
    request: &RuntimeSeedRequest,
    observation: &RuntimeSeedObservation,
) -> RuntimeSeedPlan {
    let mut blockers = validate_runtime_seed_request(request);
    let mut unknowns = observation.unknowns.clone();
    if observation.schema != RUNTIME_SEED_OBSERVATION_SCHEMA {
        unknowns.push("runtime_seed_observation_schema_mismatch".to_string());
    }
    if request.node_id != observation.node_id {
        blockers.push("runtime_seed_node_identity_mismatch".to_string());
    }
    match observation.seed_contract_revision {
        Some(revision) if revision < request.required_seed_contract_revision => {
            blockers.push("runtime_seed_contract_too_old".to_string());
        }
        None => unknowns.push("runtime_seed_contract_unobserved".to_string()),
        _ => {}
    }
    if observation.trust_root == SecurityEvidenceState::Absent {
        blockers.push("runtime_seed_trust_root_absent".to_string());
    } else if observation.trust_root == SecurityEvidenceState::Unknown {
        unknowns.push("runtime_seed_trust_root_unproved".to_string());
    }
    if observation.slot_layout == SecurityEvidenceState::Absent {
        blockers.push("runtime_seed_slot_layout_unsafe".to_string());
    } else if observation.slot_layout == SecurityEvidenceState::Unknown {
        unknowns.push("runtime_seed_slot_layout_unproved".to_string());
    }
    if observation
        .current_release_epoch
        .is_some_and(|epoch| request.release_epoch < epoch)
    {
        blockers.push("runtime_seed_epoch_rollback_forbidden".to_string());
    }
    if observation.basket_present && observation.detached_signature_present {
        if observation.basket_digest_read_back.as_deref() != Some(&request.basket_digest) {
            blockers.push("runtime_seed_basket_digest_mismatch".to_string());
        }
        if observation.signature == SecurityEvidenceState::Absent {
            blockers.push("runtime_seed_signature_invalid".to_string());
        } else if observation.signature == SecurityEvidenceState::Unknown {
            unknowns.push("runtime_seed_signature_unproved".to_string());
        }
    }

    unknowns.sort();
    unknowns.dedup();
    let release_matches = observation.current_release_id.as_deref() == Some(&request.release_id)
        && observation.current_release_epoch == Some(request.release_epoch)
        && observation.current_source_commit.as_deref() == Some(&request.source_commit)
        && observation.current_build_proof_digest.as_deref() == Some(&request.build_proof_digest)
        && observation.current_basket_digest.as_deref() == Some(&request.basket_digest)
        && observation.current_registry_digest.as_deref() == Some(&request.registry_digest);
    let registry_matches =
        observation.installed_registry_digest.as_deref() == Some(&request.registry_digest);
    let converged = release_matches
        && registry_matches
        && observation.postflight == SecurityEvidenceState::Proven;
    let actions = if !blockers.is_empty() || !unknowns.is_empty() || converged {
        Vec::new()
    } else if release_matches && !registry_matches {
        let mut actions = Vec::new();
        if !observation.basket_present || !observation.detached_signature_present {
            actions.push(RuntimeSeedAction::StageReleaseArtifacts);
        }
        actions.extend([
            RuntimeSeedAction::VerifySignatureAndDigest,
            RuntimeSeedAction::RestoreInstalledRegistry,
            RuntimeSeedAction::RunPostflight,
        ]);
        actions
    } else if observation.basket_present && observation.detached_signature_present {
        vec![
            RuntimeSeedAction::VerifySignatureAndDigest,
            RuntimeSeedAction::ActivateStagedSlot,
            RuntimeSeedAction::RunPostflight,
            RuntimeSeedAction::CommitPreviousPointer,
        ]
    } else {
        vec![
            RuntimeSeedAction::StageReleaseArtifacts,
            RuntimeSeedAction::VerifySignatureAndDigest,
            RuntimeSeedAction::ActivateStagedSlot,
            RuntimeSeedAction::RunPostflight,
            RuntimeSeedAction::CommitPreviousPointer,
        ]
    };
    let status = if !blockers.is_empty() {
        "blocked"
    } else if !unknowns.is_empty() {
        "unknown"
    } else if converged {
        "converged"
    } else {
        "planned"
    };

    RuntimeSeedPlan {
        schema: RUNTIME_SEED_PLAN_SCHEMA.to_string(),
        status: status.to_string(),
        node_id: request.node_id.clone(),
        release_id: request.release_id.clone(),
        release_epoch: request.release_epoch,
        source_commit: request.source_commit.clone(),
        build_proof_digest: request.build_proof_digest.clone(),
        actions,
        blockers,
        unknowns,
        evidence_refs: observation.evidence_refs.clone(),
        rollback_intent:
            "restore previous_link and the previous installed registry when activation or postflight read-back fails"
                .to_string(),
    }
}

pub fn validate_runtime_seed_request(request: &RuntimeSeedRequest) -> Vec<String> {
    let mut blockers = Vec::new();
    if request.schema != RUNTIME_SEED_REQUEST_SCHEMA {
        blockers.push("runtime_seed_request_schema_mismatch".to_string());
    }
    if request.node_id.is_empty() {
        blockers.push("runtime_seed_node_identity_mismatch".to_string());
    }
    if request.release_id.is_empty() || request.release_epoch == 0 {
        blockers.push("runtime_seed_release_identity_invalid".to_string());
    }
    if !valid_source_commit(&request.source_commit) {
        blockers.push("runtime_seed_source_commit_invalid".to_string());
    }
    if !valid_sha256(&request.build_proof_digest) {
        blockers.push("runtime_seed_build_proof_digest_invalid".to_string());
    }
    if !valid_sha256(&request.basket_digest) {
        blockers.push("runtime_seed_basket_digest_invalid".to_string());
    }
    if !valid_sha256(&request.registry_digest) {
        blockers.push("runtime_seed_registry_digest_invalid".to_string());
    }
    validate_artifact_source(
        &request.basket_source,
        "runtime_seed_basket_source",
        &mut blockers,
    );
    validate_artifact_source(
        &request.detached_signature_source,
        "runtime_seed_signature_source",
        &mut blockers,
    );
    if request.signature_namespace
        != crate::release_publication::RUNTIME_RELEASE_SIGNATURE_NAMESPACE
    {
        blockers.push("runtime_seed_signature_namespace_mismatch".to_string());
    }
    if request.basket_path.is_empty()
        || request.detached_signature_path.is_empty()
        || request.allowed_signers_path.is_empty()
        || request.registry_path.is_empty()
        || request.signer_identity.is_empty()
        || request.install_root.is_empty()
    {
        blockers.push("runtime_seed_verification_input_missing".to_string());
    }
    if [
        &request.basket_path,
        &request.detached_signature_path,
        &request.allowed_signers_path,
        &request.registry_path,
        &request.install_root,
    ]
    .iter()
    .any(|path| !std::path::Path::new(path).is_absolute())
    {
        blockers.push("runtime_seed_operational_path_must_be_absolute".to_string());
    }
    if request.required_seed_contract_revision == 0 {
        blockers.push("runtime_seed_contract_revision_invalid".to_string());
    }
    if request.staged_slot == request.current_link
        || request.staged_slot == request.previous_link
        || request.current_link == request.previous_link
    {
        blockers.push("runtime_seed_slots_not_distinct".to_string());
    }
    if !safe_relative_path(&request.staged_slot)
        || !safe_relative_path(&request.current_link)
        || !safe_relative_path(&request.previous_link)
    {
        blockers.push("runtime_seed_slot_path_unsafe".to_string());
    }
    if std::path::Path::new(&request.current_link)
        .components()
        .count()
        != 1
        || std::path::Path::new(&request.previous_link)
            .components()
            .count()
            != 1
    {
        blockers.push("runtime_seed_activation_link_must_be_root_relative".to_string());
    }
    if std::path::Path::new(&request.staged_slot)
        .components()
        .next()
        != Some(std::path::Component::Normal(std::ffi::OsStr::new(
            "releases",
        )))
    {
        blockers.push("runtime_seed_staged_slot_outside_release_namespace".to_string());
    }
    if request.postflight_argv.is_empty()
        || request
            .postflight_argv
            .iter()
            .any(|argument| argument.trim().is_empty())
    {
        blockers.push("runtime_seed_postflight_argv_invalid".to_string());
    }
    if request
        .postflight_argv
        .first()
        .is_some_and(|command| matches!(command.as_str(), "sh" | "bash" | "dash" | "zsh"))
        || request
            .postflight_argv
            .iter()
            .any(|argument| argument == "-c" || argument == "--command")
    {
        blockers.push("runtime_seed_postflight_shell_forbidden".to_string());
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn valid_source_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_artifact_source(
    source: &RuntimeArtifactSource,
    blocker_prefix: &str,
    blockers: &mut Vec<String>,
) {
    let locator = source.locator.trim();
    if locator.is_empty() {
        blockers.push(format!("{blocker_prefix}_missing"));
        return;
    }
    let valid = match source.transport {
        RuntimeArtifactTransportKind::LocalFile => std::path::Path::new(locator).is_absolute(),
        RuntimeArtifactTransportKind::Https => {
            locator.starts_with("https://")
                && !locator["https://".len()..].contains('@')
                && !locator.contains('?')
                && !locator.contains('#')
        }
    };
    if !valid {
        blockers.push(format!("{blocker_prefix}_invalid"));
    }
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && std::path::Path::new(value)
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> RuntimeSeedRequest {
        RuntimeSeedRequest {
            schema: RUNTIME_SEED_REQUEST_SCHEMA.to_string(),
            node_id: "cynic-builder".to_string(),
            release_id: "release-2".to_string(),
            release_epoch: 2,
            source_commit: "c".repeat(40),
            build_proof_digest: format!("sha256:{}", "d".repeat(64)),
            basket_digest: format!("sha256:{}", "a".repeat(64)),
            registry_digest: format!("sha256:{}", "e".repeat(64)),
            basket_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::Https,
                locator: "https://artifacts.synapse.cynic/releases/release-2.tar".to_string(),
            },
            basket_path: "/var/lib/cynic/incoming/release-2.tar".to_string(),
            detached_signature_source: RuntimeArtifactSource {
                transport: RuntimeArtifactTransportKind::Https,
                locator: "https://artifacts.synapse.cynic/releases/release-2.tar.sig".to_string(),
            },
            detached_signature_path: "/var/lib/cynic/incoming/release-2.tar.sig".to_string(),
            allowed_signers_path: "/etc/cynic/runtime-seed-allowed-signers".to_string(),
            registry_path: "/etc/cynic/registry.json".to_string(),
            signer_identity: "cynic-runtime-release".to_string(),
            signature_namespace: "cynic-runtime-seed".to_string(),
            required_seed_contract_revision: 4,
            install_root: "/opt/cynic/runtime".to_string(),
            staged_slot: "releases/release-2".to_string(),
            current_link: "current".to_string(),
            previous_link: "previous".to_string(),
            postflight_argv: vec![
                "current/bin/cynic-mcp".to_string(),
                "--contract-json".to_string(),
            ],
        }
    }

    fn observation() -> RuntimeSeedObservation {
        RuntimeSeedObservation {
            schema: RUNTIME_SEED_OBSERVATION_SCHEMA.to_string(),
            observed_at_unix_ms: 1,
            node_id: "cynic-builder".to_string(),
            seed_contract_revision: Some(4),
            trust_root: SecurityEvidenceState::Proven,
            signature: SecurityEvidenceState::Proven,
            basket_present: true,
            detached_signature_present: true,
            basket_digest_read_back: Some(format!("sha256:{}", "a".repeat(64))),
            current_release_id: Some("release-1".to_string()),
            current_release_epoch: Some(1),
            current_source_commit: Some("b".repeat(40)),
            current_build_proof_digest: Some(format!("sha256:{}", "c".repeat(64))),
            current_basket_digest: Some(format!("sha256:{}", "b".repeat(64))),
            current_registry_digest: Some(format!("sha256:{}", "f".repeat(64))),
            installed_registry_digest: Some(format!("sha256:{}", "f".repeat(64))),
            previous_release_id: None,
            slot_layout: SecurityEvidenceState::Proven,
            postflight: SecurityEvidenceState::Unknown,
            evidence_refs: vec!["seed:/usr/local/libexec/cynic-host-exec".to_string()],
            unknowns: vec![],
        }
    }

    #[test]
    fn plans_atomic_activation_after_signature_and_digest_proof() {
        let plan = reconcile_runtime_seed(&request(), &observation());

        assert_eq!(plan.status, "planned");
        assert_eq!(
            plan.actions,
            vec![
                RuntimeSeedAction::VerifySignatureAndDigest,
                RuntimeSeedAction::ActivateStagedSlot,
                RuntimeSeedAction::RunPostflight,
                RuntimeSeedAction::CommitPreviousPointer,
            ]
        );
    }

    #[test]
    fn registry_drift_selects_the_bounded_registry_repair() {
        let request = request();
        let mut observation = observation();
        observation.current_release_id = Some(request.release_id.clone());
        observation.current_release_epoch = Some(request.release_epoch);
        observation.current_source_commit = Some(request.source_commit.clone());
        observation.current_build_proof_digest = Some(request.build_proof_digest.clone());
        observation.current_basket_digest = Some(request.basket_digest.clone());
        observation.current_registry_digest = Some(request.registry_digest.clone());
        observation.installed_registry_digest = Some(format!("sha256:{}", "f".repeat(64)));

        let plan = reconcile_runtime_seed(&request, &observation);

        assert_eq!(plan.status, "planned");
        assert_eq!(
            plan.actions,
            vec![
                RuntimeSeedAction::VerifySignatureAndDigest,
                RuntimeSeedAction::RestoreInstalledRegistry,
                RuntimeSeedAction::RunPostflight,
            ]
        );
    }

    #[test]
    fn forbids_epoch_rollback_even_with_a_valid_signature() {
        let mut desired = request();
        desired.release_epoch = 1;
        let mut observed = observation();
        observed.current_release_epoch = Some(2);

        let plan = reconcile_runtime_seed(&desired, &observed);

        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"runtime_seed_epoch_rollback_forbidden".to_string())
        );
    }

    #[test]
    fn unknown_trust_root_never_produces_actions() {
        let mut observed = observation();
        observed.trust_root = SecurityEvidenceState::Unknown;

        let plan = reconcile_runtime_seed(&request(), &observed);

        assert_eq!(plan.status, "unknown");
        assert!(plan.actions.is_empty());
    }

    #[test]
    fn release_without_admitted_build_proof_digest_is_blocked() {
        let mut desired = request();
        desired.build_proof_digest = "missing".to_string();

        let plan = reconcile_runtime_seed(&desired, &observation());

        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"runtime_seed_build_proof_digest_invalid".to_string())
        );
    }

    #[test]
    fn matching_release_identity_without_matching_source_never_converges() {
        let desired = request();
        let mut observed = observation();
        observed.current_release_id = Some(desired.release_id.clone());
        observed.current_release_epoch = Some(desired.release_epoch);
        observed.current_basket_digest = Some(desired.basket_digest.clone());
        observed.current_build_proof_digest = Some(desired.build_proof_digest.clone());
        observed.postflight = SecurityEvidenceState::Proven;

        let plan = reconcile_runtime_seed(&desired, &observed);

        assert_eq!(plan.status, "planned");
        assert!(!plan.actions.is_empty());
    }

    #[test]
    fn missing_local_artifacts_plan_transport_before_verification() {
        let mut observed = observation();
        observed.basket_present = false;
        observed.detached_signature_present = false;
        observed.basket_digest_read_back = None;
        observed.signature = SecurityEvidenceState::Unknown;

        let plan = reconcile_runtime_seed(&request(), &observed);

        assert_eq!(plan.status, "planned");
        assert_eq!(
            plan.actions.first(),
            Some(&RuntimeSeedAction::StageReleaseArtifacts)
        );
    }

    #[test]
    fn insecure_remote_artifact_locator_is_blocked() {
        let mut desired = request();
        desired.basket_source.locator = "http://storage/release-2.tar".to_string();

        let plan = reconcile_runtime_seed(&desired, &observation());

        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"runtime_seed_basket_source_invalid".to_string())
        );
    }
}
