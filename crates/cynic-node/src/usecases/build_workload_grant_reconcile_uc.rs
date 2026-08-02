use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use cynic_host::registry::resolve_current_host_profile_resolution;
use cynic_protocol::{
    SignedBuildWorkloadGrant, canonical_build_action_from_argv, derive_build_workload_grant,
};

use crate::{
    adapters::{
        ed25519_build_workload_grant::Ed25519BuildWorkloadGrantSigner,
        local_agent_workload_store::LocalAgentWorkloadStore,
        local_build_workload_grant_store::LocalBuildWorkloadGrantStore,
    },
    ports::{
        agent_workload_store_port::AgentWorkloadStorePort,
        build_workload_grant_store_port::BuildWorkloadGrantStorePort,
    },
    usecases::build_workload_grant_uc::{
        issue_signed_build_workload_grant, verify_signed_build_workload_grant,
    },
};

pub fn execute_build_workload_grant_reconcile(
    workload_id: &str,
    state_root: &str,
    signing_key_path: &str,
    registry_path: &str,
    apply: bool,
) -> ExitCode {
    let workload_store =
        LocalAgentWorkloadStore::new(PathBuf::from(state_root).join("agent-scheduler/workloads"));
    let durable = match workload_store.read(workload_id) {
        Ok(Some(durable)) => durable,
        Ok(None) => return emit_blocked(workload_id, vec!["build_workload_record_missing".into()]),
        Err(failure) => return emit_blocked(workload_id, vec![failure.code]),
    };
    let Some(requested_action) =
        canonical_build_action_from_argv(&durable.record.envelope.action.argv)
    else {
        return emit_blocked(
            workload_id,
            vec!["build_workload_action_not_canonical".into()],
        );
    };
    let target_node_id = durable.record.envelope.delegation.node_id.clone();
    let source_commit = durable
        .record
        .envelope
        .policy_admission
        .intent
        .subject
        .clone();
    let issuer_node_id = match resolve_current_host_profile_resolution(Some(registry_path)) {
        Ok(resolution) => match resolution.matched_node {
            Some(node_id) => node_id,
            None => {
                return emit_blocked(
                    workload_id,
                    vec!["build_grant_issuer_identity_unresolved".into()],
                );
            }
        },
        Err(_) => {
            return emit_blocked(
                workload_id,
                vec!["build_grant_issuer_identity_unobservable".into()],
            );
        }
    };
    let now_unix_s = chrono::Utc::now().timestamp().max(0) as u64;
    let desired = match derive_build_workload_grant(&durable, &issuer_node_id, now_unix_s) {
        Ok(grant) => grant,
        Err(blockers) => return emit_blocked(workload_id, blockers),
    };
    let signer = match Ed25519BuildWorkloadGrantSigner::load_hex_file(Path::new(signing_key_path)) {
        Ok(signer) => signer,
        Err(_) => {
            return emit_blocked(
                workload_id,
                vec!["build_grant_machine_signer_unavailable".into()],
            );
        }
    };
    let verifier = signer.verifier();
    let grant_store = LocalBuildWorkloadGrantStore::new(
        PathBuf::from(state_root).join("agent-scheduler/build-grants"),
    );
    let latest = match grant_store.latest(workload_id) {
        Ok(latest) => latest,
        Err(failure) => return emit_blocked(workload_id, vec![failure.code]),
    };
    let superseded_blockers = latest
        .as_ref()
        .and_then(|persisted| {
            verify_signed_build_workload_grant(
                &persisted.grant,
                &durable,
                &target_node_id,
                &source_commit,
                requested_action,
                now_unix_s,
                &verifier,
            )
            .err()
        })
        .unwrap_or_default();
    if let Some(persisted) = latest.filter(|_| superseded_blockers.is_empty()) {
        return emit_report(
            "converged",
            false,
            workload_id,
            Some(&persisted.grant),
            Some(&persisted.evidence_ref),
            Vec::new(),
        );
    }
    if !apply {
        return emit(
            &serde_json::json!({
                "schema": "cynic.node.build_workload_grant_reconcile.v1",
                "status": "planned",
                "mutates_runtime": true,
                "apply_required": true,
                "workload_id": workload_id,
                "desired_grant": desired,
                "superseded_grant_blockers": superseded_blockers,
                "transport_delivery": "unknown",
                "runner_live_verification": "unknown",
                "secret_exposed": false,
            }),
            true,
        );
    }
    let signed =
        match issue_signed_build_workload_grant(&durable, &issuer_node_id, now_unix_s, &signer) {
            Ok(signed) => signed,
            Err(blockers) => return emit_blocked(workload_id, blockers),
        };
    let persisted = match grant_store.persist(&signed) {
        Ok(persisted) => persisted,
        Err(failure) => return emit_blocked(workload_id, vec![failure.code]),
    };
    let blockers = verify_signed_build_workload_grant(
        &persisted.grant,
        &durable,
        &target_node_id,
        &source_commit,
        requested_action,
        now_unix_s,
        &verifier,
    )
    .err()
    .unwrap_or_default();
    if !blockers.is_empty() {
        return emit_blocked(workload_id, blockers);
    }
    emit_report(
        "issued",
        true,
        workload_id,
        Some(&persisted.grant),
        Some(&persisted.evidence_ref),
        superseded_blockers,
    )
}

fn emit_report(
    status: &str,
    mutates_runtime: bool,
    workload_id: &str,
    grant: Option<&SignedBuildWorkloadGrant>,
    evidence_ref: Option<&str>,
    superseded_grant_blockers: Vec<String>,
) -> ExitCode {
    emit(
        &serde_json::json!({
            "schema": "cynic.node.build_workload_grant_reconcile.v1",
            "status": status,
            "mutates_runtime": mutates_runtime,
            "workload_id": workload_id,
            "read_back": grant,
            "evidence_ref": evidence_ref,
            "superseded_grant_blockers": superseded_grant_blockers,
            "transport_delivery": "unknown",
            "runner_live_verification": "unknown",
            "secret_exposed": false,
        }),
        true,
    )
}

fn emit_blocked(workload_id: &str, blockers: Vec<String>) -> ExitCode {
    emit(
        &serde_json::json!({
            "schema": "cynic.node.build_workload_grant_reconcile.v1",
            "status": "blocked",
            "mutates_runtime": false,
            "workload_id": workload_id,
            "blockers": blockers,
            "transport_delivery": "unknown",
            "runner_live_verification": "unknown",
            "secret_exposed": false,
        }),
        false,
    )
}

fn emit(value: &impl serde::Serialize, success: bool) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(_) => {
            eprintln!("build workload grant report serialization failed");
            return ExitCode::FAILURE;
        }
    }
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
