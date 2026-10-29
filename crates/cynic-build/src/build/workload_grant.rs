use serde::{Deserialize, Serialize};

use cynic_agent::{
    AgentWorkloadState, DurableAgentWorkloadRecord, validate_durable_agent_workload_record,
};

use super::canonical_build_actions;

pub const BUILD_WORKLOAD_GRANT_SCHEMA: &str = "cynic.build.workload_grant.v1";
pub const SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA: &str = "cynic.signed_build_workload_grant.v1";
pub const BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE: &str = "cynic-build-workload-grant";
pub const BUILD_WORKLOAD_GRANT_MAX_TTL_SECONDS: u64 = 900;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildWorkloadGrant {
    pub schema: String,
    pub status: String,
    pub grant_id: String,
    pub issuer_node_id: String,
    pub workload_id: String,
    pub workload_revision: u64,
    pub workload_record_digest: String,
    pub node_id: String,
    pub role: String,
    pub source_commit: String,
    pub policy_digest: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub cancellation_generation: u64,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub allowed_actions: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedBuildWorkloadGrant {
    pub schema: String,
    pub authority_key_id: String,
    pub signature_namespace: String,
    pub grant: BuildWorkloadGrant,
    pub signature_hex: String,
}

pub fn build_workload_grant_signing_bytes(
    signed: &SignedBuildWorkloadGrant,
) -> Result<Vec<u8>, serde_json::Error> {
    cynic_protocol::canonical_json_bytes(&serde_json::json!({
        "schema": signed.schema,
        "authority_key_id": signed.authority_key_id,
        "signature_namespace": signed.signature_namespace,
        "grant": signed.grant,
    }))
}

pub fn validate_signed_build_workload_grant(signed: &SignedBuildWorkloadGrant) -> Vec<String> {
    let mut blockers = Vec::new();
    if signed.schema != SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA {
        blockers.push("signed_build_workload_grant_schema_mismatch".to_string());
    }
    if signed.signature_namespace != BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE {
        blockers.push("signed_build_workload_grant_namespace_mismatch".to_string());
    }
    if !valid_sha256(&signed.authority_key_id) {
        blockers.push("signed_build_workload_grant_authority_key_id_invalid".to_string());
    }
    if signed.signature_hex.len() != 128
        || !signed
            .signature_hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        blockers.push("signed_build_workload_grant_signature_invalid".to_string());
    }
    blockers
}

pub fn derive_build_workload_grant(
    durable: &DurableAgentWorkloadRecord,
    issuer_node_id: &str,
    now_unix_s: u64,
) -> Result<BuildWorkloadGrant, Vec<String>> {
    let mut blockers = validate_durable_agent_workload_record(durable);
    let record = &durable.record;
    let envelope = &record.envelope;

    if issuer_node_id.trim().is_empty() {
        blockers.push("build_workload_grant_issuer_missing".to_string());
    }
    if record.state != AgentWorkloadState::Claimed {
        blockers.push("build_workload_not_claimed".to_string());
    }
    // Reservation check: at least one action must have been consumed
    // (submit-time authorization and/or scheduler execution reservation).
    // Was `!= 1`, which assumed a single-consume flow — but the lifecycle
    // consumes twice (orchestrator at submit, scheduler at prepare), so
    // every natively submitted workload failed here after the budget fix.
    // Over-consumption cannot slip through: consume itself is fenced by
    // the delegation budget.
    if record.consumed_actions == 0 {
        blockers.push("build_workload_action_not_reserved".to_string());
    }
    let claim = record.claim.as_ref();
    let claim_expiry_unix_s = claim.map(|claim| claim.expires_at_ms / 1_000);
    if claim_expiry_unix_s.is_none_or(|expiry| expiry <= now_unix_s) {
        blockers.push("build_workload_claim_expired".to_string());
    }
    if claim.is_some_and(|claim| {
        claim.cancellation_generation != record.cancellation_generation
            || claim.guard.fencing_token != record.last_fencing_token
    }) {
        blockers.push("build_workload_claim_stale".to_string());
    }
    if envelope.delegation.node_id.trim().is_empty() {
        blockers.push("build_workload_target_node_missing".to_string());
    }
    if !envelope
        .delegation
        .capabilities
        .iter()
        .any(|capability| capability.id == "repository_validation" && capability.mutates)
    {
        blockers.push("build_workload_capability_missing".to_string());
    }
    if envelope.policy_admission.mutation.scope.domain != "build" {
        blockers.push("build_workload_policy_scope_mismatch".to_string());
    }

    let source_commit = envelope.policy_admission.intent.subject.as_str();
    if !is_commit_id(source_commit)
        || envelope.policy_admission.mutation.scope.target != source_commit
    {
        blockers.push("build_workload_source_commit_unbound".to_string());
    }
    if envelope
        .action
        .working_directory
        .as_deref()
        .is_none_or(str::is_empty)
        || envelope
            .action
            .input_digest
            .as_deref()
            .is_none_or(str::is_empty)
    {
        blockers.push("build_workload_source_materialization_unbound".to_string());
    }

    let allowed_actions = match canonical_build_action_from_argv(&envelope.action.argv)
        .and_then(|action| canonical_build_actions(action).ok())
    {
        Some(actions) => actions,
        None => {
            blockers.push("build_workload_action_not_canonical".to_string());
            Vec::new()
        }
    };

    if !blockers.is_empty() {
        blockers.sort();
        blockers.dedup();
        return Err(blockers);
    }

    let (claim, claim_expiry_unix_s) = match (claim, claim_expiry_unix_s) {
        (Some(claim), Some(expiry)) => (claim, expiry),
        _ => return Err(vec!["build_workload_claim_missing".to_string()]),
    };
    let expires_at_unix_s = now_unix_s
        .saturating_add(BUILD_WORKLOAD_GRANT_MAX_TTL_SECONDS)
        .min(claim_expiry_unix_s);
    if expires_at_unix_s <= now_unix_s {
        return Err(vec!["build_workload_grant_expiry_invalid".to_string()]);
    }

    Ok(BuildWorkloadGrant {
        schema: BUILD_WORKLOAD_GRANT_SCHEMA.to_string(),
        status: "admitted".to_string(),
        grant_id: format!(
            "build-workload:{}:{}:{}",
            envelope.workload_id, durable.revision, claim.guard.fencing_token
        ),
        issuer_node_id: issuer_node_id.to_string(),
        workload_id: envelope.workload_id.clone(),
        workload_revision: durable.revision,
        workload_record_digest: durable.record_digest.clone(),
        node_id: envelope.delegation.node_id.clone(),
        role: "build_runner".to_string(),
        source_commit: source_commit.to_string(),
        policy_digest: envelope.policy_admission.policy_set.digest.clone(),
        lease_id: claim.guard.lease_id.clone(),
        fencing_token: claim.guard.fencing_token,
        cancellation_generation: record.cancellation_generation,
        issued_at_unix_s: now_unix_s,
        expires_at_unix_s,
        allowed_actions,
        evidence_refs: vec![
            format!("workload-record:{}", durable.record_digest),
            format!("workload-revision:{}", durable.revision),
            format!(
                "workload-lease:{}:{}",
                claim.guard.lease_id, claim.guard.fencing_token
            ),
            format!("policy:{}", envelope.policy_admission.policy_set.digest),
        ],
    })
}

pub fn validate_build_workload_grant(
    grant: &BuildWorkloadGrant,
    durable: &DurableAgentWorkloadRecord,
    observed_node_id: &str,
    observed_source_commit: &str,
    requested_action: &str,
    now_unix_s: u64,
) -> Result<(), Vec<String>> {
    let mut blockers = validate_durable_agent_workload_record(durable);
    let record = &durable.record;
    let envelope = &record.envelope;
    let claim = record.claim.as_ref();

    if grant.schema != BUILD_WORKLOAD_GRANT_SCHEMA || grant.status != "admitted" {
        blockers.push("build_workload_grant_not_admitted".to_string());
    }
    if grant.grant_id.trim().is_empty() || grant.issuer_node_id.trim().is_empty() {
        blockers.push("build_workload_grant_identity_missing".to_string());
    }
    if grant.workload_id != envelope.workload_id
        || grant.workload_revision != durable.revision
        || grant.workload_record_digest != durable.record_digest
    {
        blockers.push("build_workload_grant_record_mismatch".to_string());
    }
    if record.state != AgentWorkloadState::Claimed
        || claim.is_none_or(|claim| {
            grant.lease_id != claim.guard.lease_id
                || grant.fencing_token != claim.guard.fencing_token
                || grant.cancellation_generation != claim.cancellation_generation
                || claim.cancellation_generation != record.cancellation_generation
                || claim.expires_at_ms / 1_000 <= now_unix_s
        })
    {
        blockers.push("build_workload_grant_claim_not_live".to_string());
    }
    if grant.node_id != observed_node_id
        || grant.node_id != envelope.delegation.node_id
        || grant.role != "build_runner"
    {
        blockers.push("build_workload_grant_node_mismatch".to_string());
    }
    if grant.source_commit != observed_source_commit
        || grant.source_commit != envelope.policy_admission.intent.subject
        || grant.source_commit != envelope.policy_admission.mutation.scope.target
    {
        blockers.push("build_workload_grant_source_mismatch".to_string());
    }
    if grant.policy_digest != envelope.policy_admission.policy_set.digest {
        blockers.push("build_workload_grant_policy_mismatch".to_string());
    }
    let expected_actions = canonical_build_actions(requested_action).ok();
    if expected_actions.as_ref() != Some(&grant.allowed_actions) {
        blockers.push("build_workload_grant_action_mismatch".to_string());
    }
    if grant.issued_at_unix_s == 0
        || grant.expires_at_unix_s <= grant.issued_at_unix_s
        || now_unix_s < grant.issued_at_unix_s
        || now_unix_s >= grant.expires_at_unix_s
        || grant.expires_at_unix_s - grant.issued_at_unix_s > BUILD_WORKLOAD_GRANT_MAX_TTL_SECONDS
        || claim.is_some_and(|claim| grant.expires_at_unix_s > claim.expires_at_ms / 1_000)
    {
        blockers.push("build_workload_grant_expired_or_invalid".to_string());
    }
    if grant.evidence_refs.is_empty() {
        blockers.push("build_workload_grant_evidence_missing".to_string());
    }

    blockers.sort();
    blockers.dedup();
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

pub fn canonical_build_action_from_argv(argv: &[String]) -> Option<&'static str> {
    match argv {
        // The retired `make:*` spellings are refused, not renamed. The
        // canonical action list (`cynic_protocol::argv_allow::
        // canonical_build_actions`) admits only `cargo:*` and `audit:*`,
        // because the Makefile targets these verbs named do not exist, so
        // no argv they admit can execute. Mapping `make check` to
        // `make:check` produced an action the policy refuses downstream:
        // the request was composed with a name that could never be
        // admitted. A retired spelling resolves to no action at all.
        //
        // Native audit argv resolves to the canonical name, not the alias:
        // the action says what runs.
        [program, run, _q, _p, _cli, _dd, audit, crate_]
            if program == "cargo" && run == "run" && audit == "audit" && crate_ == "crate" =>
        {
            Some("audit:crate")
        }
        [program, run, _q, _p, _cli, _dd, audit, all_]
            if program == "cargo" && run == "run" && audit == "audit" && all_ == "all" =>
        {
            Some("audit:all")
        }
        [program, tool, _ws, _targets] if program == "cargo" && tool == "clippy" => {
            Some("cargo:clippy")
        }
        [program, tool, _all, _check] if program == "cargo" && tool == "fmt" => {
            Some("cargo:fmt-check")
        }
        [program, target, ..] if program == "cargo" && target == "check" => Some("cargo:check"),
        [program, target] if program == "cargo" && target == "build" => Some("cargo:build"),
        [program, target, release]
            if program == "cargo" && target == "build" && release == "--release" =>
        {
            Some("cargo:build-release")
        }
        [program, target, ..] if program == "cargo" && target == "test" => Some("cargo:test"),
        _ => None,
    }
}

/// Translate a canonical build action into its argv representation.
///
/// Single reader of `infra/command-registry.json` (via
/// `cynic_protocol::command_registry`): the argv a builder executes for an
/// action is data, declared exactly once in the registry. Inverse of
/// [`canonical_build_action_from_argv`] for the actions the registry
/// composes. Returns `None` for actions the registry does not compose
/// (unwired `cargo:run`, retired `make:*`, unknown actions) — this asymmetry
/// is the bounded-action gap measured in the build workflow audit: policy
/// admits verbs the executor cannot construct.
pub fn canonical_argv_from_build_action(action: &str) -> Option<Vec<String>> {
    cynic_protocol::command_registry::registry_argv(action)
}

fn is_commit_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod action_tests {
    use super::canonical_argv_from_build_action;
    use super::canonical_build_action_from_argv;

    #[test]
    fn canonical_action_is_derived_once_from_exact_argv() {
        assert_eq!(
            canonical_build_action_from_argv(&["cargo".to_string(), "check".to_string()]),
            Some("cargo:check")
        );
        assert_eq!(
            canonical_build_action_from_argv(&[
                "bash".to_string(),
                "-c".to_string(),
                "make check".to_string()
            ]),
            None
        );
        // The `make:*` spellings are retired. They name Makefile targets
        // that do not exist, and the canonical action list admits only
        // `cargo:*` and `audit:*`, so a retired spelling resolves to no
        // action rather than to a name the policy refuses downstream.
        assert_eq!(
            canonical_build_action_from_argv(&["make".to_string(), "check".to_string()]),
            None
        );
        assert_eq!(
            canonical_build_action_from_argv(&["make".to_string(), "build-release".to_string()]),
            None
        );
    }

    /// Slice-scoped argv (`cargo check --package …`) is the same canonical
    /// action as the bare form: the scheduler must recognize what the slice
    /// translator emits, or admitted workloads never run.
    #[test]
    fn package_scoped_check_argv_resolves_to_cargo_check() {
        assert_eq!(
            canonical_build_action_from_argv(&[
                "cargo".to_string(),
                "check".to_string(),
                "--package".to_string(),
                "cynic-host".to_string(),
                "--lib".to_string(),
            ]),
            Some("cargo:check")
        );
    }

    /// Invariant: every action the forward translator produces must round-trip
    /// through the reverse translator. If this fails, policy can admit a verb
    /// whose argv cannot be constructed — the bounded-action gap.
    /// The forward set is the registry's composed set: this test locks
    /// composition to `infra/command-registry.json`, so a registry edit
    /// without a matching reverse reader fails here, not in production.
    #[test]
    fn every_translatable_argv_roundtrips_through_its_action() {
        let actions =
            cynic_protocol::command_registry::registry_composed_actions().expect("registry parses");
        assert!(
            !actions.is_empty(),
            "registry must compose at least one action"
        );
        for action in &actions {
            let argv = canonical_argv_from_build_action(action)
                .unwrap_or_else(|| panic!("action {action} must be translatable"));
            let roundtripped = canonical_build_action_from_argv(&argv);
            assert_eq!(
                roundtripped,
                Some(action.as_str()),
                "argv for {action} did not round-trip: got {roundtripped:?}"
            );
        }
    }
    /// Invariant: actions admitted by policy that have no canonical argv must
    /// return `None`. These are the known gaps; this test pins them so that
    /// adding a translation does not silently leave the gap.
    #[test]
    fn untranslatable_admitted_actions_return_none() {
        for action in ["cargo:run", "make:build-host-exec"] {
            assert_eq!(
                canonical_argv_from_build_action(action),
                None,
                "action {action} is admitted by policy but has no argv"
            );
        }
    }

    /// Task P: cargo build (debug) is a distinct canonical action from
    /// cargo build --release. The old mapping collapsed both into
    /// "cargo:build" (release), making debug builds impossible to express.
    #[test]
    fn cargo_build_without_release_maps_to_cargo_build() {
        assert_eq!(
            canonical_build_action_from_argv(&["cargo".to_string(), "build".to_string()]),
            Some("cargo:build")
        );
    }

    #[test]
    fn cargo_build_release_maps_to_cargo_build_release() {
        assert_eq!(
            canonical_build_action_from_argv(&[
                "cargo".to_string(),
                "build".to_string(),
                "--release".to_string()
            ]),
            Some("cargo:build-release")
        );
    }

    #[test]
    fn cargo_test_maps_to_cargo_test() {
        assert_eq!(
            canonical_build_action_from_argv(&["cargo".to_string(), "test".to_string()]),
            Some("cargo:test")
        );
    }

    #[test]
    fn cargo_build_argv_roundtrips() {
        let argv = canonical_argv_from_build_action("cargo:build").expect("translatable");
        assert_eq!(argv, vec!["cargo", "build"]);
        assert_eq!(canonical_build_action_from_argv(&argv), Some("cargo:build"));
    }

    #[test]
    fn cargo_build_release_argv_roundtrips() {
        let argv = canonical_argv_from_build_action("cargo:build-release").expect("translatable");
        assert_eq!(argv, vec!["cargo", "build", "--release"]);
        assert_eq!(
            canonical_build_action_from_argv(&argv),
            Some("cargo:build-release")
        );
    }

    #[test]
    fn cargo_test_argv_roundtrips() {
        let argv = canonical_argv_from_build_action("cargo:test").expect("translatable");
        assert_eq!(argv, vec!["cargo", "test"]);
        assert_eq!(canonical_build_action_from_argv(&argv), Some("cargo:test"));
    }
}
