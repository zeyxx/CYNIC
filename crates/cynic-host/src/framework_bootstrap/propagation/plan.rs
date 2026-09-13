use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::registry::resolve_framework_transport;
use crate::{adapters::remote_execution::SshRemoteExecutionAdapter, ports::RemoteExecutionPort};
use cynic_lifecycle::ArgvRunner;

use super::decision::decide_framework_propagation;
use super::observe::observe_framework_propagation_with_transport;
use super::types::{FrameworkPropagationOperation, FrameworkPropagationPlan};

const REMOTE_SEED_ROOT: &str = "/tmp/cynic-framework-bootstrap";

#[derive(Deserialize)]
struct SeedManifest {
    seed_id: String,
    enrollment: SeedEnrollment,
}

#[derive(Deserialize)]
struct SeedEnrollment {
    target_role: String,
}

pub fn plan_framework_propagation(
    bundle_dir: &str,
    registry_path: Option<&str>,
    runner: &dyn ArgvRunner,
) -> Result<FrameworkPropagationPlan, String> {
    plan_framework_propagation_with_transport(
        bundle_dir,
        registry_path,
        runner,
        &SshRemoteExecutionAdapter,
    )
}

pub fn plan_framework_propagation_with_transport(
    bundle_dir: &str,
    registry_path: Option<&str>,
    runner: &dyn ArgvRunner,
    remote_execution: &dyn RemoteExecutionPort,
) -> Result<FrameworkPropagationPlan, String> {
    let bundle = Path::new(bundle_dir);
    let manifest_path = bundle.join("seed-manifest.json");
    let checksum_path = bundle.join("seed-artifacts.sha256");
    let mut blockers = Vec::new();
    if !manifest_path.is_file() {
        blockers.push("seed_manifest_missing".to_string());
    }
    if !checksum_path.is_file() {
        blockers.push("seed_checksums_missing".to_string());
    }
    let manifest_bytes = if manifest_path.is_file() {
        Some(std::fs::read(&manifest_path).map_err(|error| {
            format!(
                "failed to read seed manifest {}: {error}",
                manifest_path.display()
            )
        })?)
    } else {
        None
    };
    let manifest = manifest_bytes
        .as_ref()
        .map(|content| {
            serde_json::from_slice::<SeedManifest>(content)
                .map_err(|error| format!("invalid seed manifest: {error}"))
        })
        .transpose()?;
    let target_role = manifest
        .as_ref()
        .map(|manifest| manifest.enrollment.target_role.as_str())
        .filter(|role| !role.trim().is_empty())
        .ok_or_else(|| "seed_target_role_missing".to_string())?;
    let transport = resolve_framework_transport(registry_path, target_role)?;
    let seed_id = manifest
        .as_ref()
        .map(|manifest| manifest.seed_id.clone())
        .unwrap_or_default();
    if manifest
        .as_ref()
        .is_some_and(|manifest| !valid_seed_id(&manifest.seed_id))
    {
        blockers.push("seed_id_invalid".to_string());
    }
    let expected_runtime_basket_digest = manifest_bytes
        .as_ref()
        .map(|content| format!("sha256:{}", hex::encode(Sha256::digest(content))))
        .unwrap_or_default();
    let observation = observe_framework_propagation_with_transport(
        &transport.ssh_target,
        runner,
        remote_execution,
    );
    let decision = decide_framework_propagation(
        &expected_runtime_basket_digest,
        &transport.node_id,
        &transport.enrollment_authority_class,
        &transport.enrollment_principal,
        &observation,
        blockers,
    );
    let operations = if decision.status == "enroll_with_machine_authority" {
        enrollment_operations_with_transport(
            bundle_dir,
            &transport.ssh_target,
            &seed_id,
            &expected_runtime_basket_digest,
            remote_execution,
        )
    } else {
        Vec::new()
    };
    Ok(FrameworkPropagationPlan {
        schema: "cynic.framework_bootstrap.propagation_plan.v2",
        status: match decision.status {
            "enroll_with_machine_authority" => "planned_enrollment",
            "converged" => "converged",
            _ => "blocked",
        },
        mutates: false,
        node_id: transport.node_id,
        role: transport.role,
        ssh_target: transport.ssh_target,
        trust: transport.trust,
        enrollment_authority_class: transport.enrollment_authority_class,
        enrollment_principal: transport.enrollment_principal,
        enrollment_credential_ref: transport.enrollment_credential_ref,
        bundle_dir: bundle_dir.to_string(),
        seed_id,
        expected_runtime_basket_digest,
        blockers: decision.blockers.clone(),
        observation,
        decision,
        operations,
        stop_condition: "stop on the first non-zero transport, enrollment, or read-back command; execute only through the registry-declared machine enrollment principal",
        rollback_intent: "remove only the content-addressed transferred seed; installed runtime state is reconciled through signed release surfaces",
    })
}

#[allow(dead_code)]
pub(super) fn enrollment_operations(
    bundle_dir: &str,
    ssh_target: &str,
    seed_id: &str,
    expected_runtime_basket_digest: &str,
) -> Vec<FrameworkPropagationOperation> {
    enrollment_operations_with_transport(
        bundle_dir,
        ssh_target,
        seed_id,
        expected_runtime_basket_digest,
        &SshRemoteExecutionAdapter,
    )
}

pub(super) fn enrollment_operations_with_transport(
    bundle_dir: &str,
    ssh_target: &str,
    seed_id: &str,
    expected_runtime_basket_digest: &str,
    remote_execution: &dyn RemoteExecutionPort,
) -> Vec<FrameworkPropagationOperation> {
    let remote_seed_dir = remote_seed_dir(seed_id);
    let mut prepare = remote_execution
        .exec_argv(ssh_target, &["mkdir", "-p", &remote_seed_dir])
        .unwrap_or_default();
    let read_back = remote_execution
        .exec_argv(
            ssh_target,
            &[
                "sha256sum",
                &format!("{remote_seed_dir}/seed-manifest.json"),
            ],
        )
        .unwrap_or_default();

    vec![
        FrameworkPropagationOperation {
            phase: "preflight",
            argv: remote_execution
                .exec_argv(ssh_target, &["id", "-un"])
                .unwrap_or_default(),
            mutates: false,
            requires_privilege: false,
            proof: Some("machine_enrollment_principal_read_back"),
            expected_stdout_sha256: None,
        },
        FrameworkPropagationOperation {
            phase: "prepare",
            argv: std::mem::take(&mut prepare),
            mutates: true,
            requires_privilege: false,
            proof: None,
            expected_stdout_sha256: None,
        },
        FrameworkPropagationOperation {
            phase: "transfer",
            argv: remote_execution
                .copy_argv(
                    &format!("{bundle_dir}/."),
                    &format!("{ssh_target}:{remote_seed_dir}"),
                )
                .unwrap_or_default(),
            mutates: true,
            requires_privilege: false,
            proof: Some("seed_transferred"),
            expected_stdout_sha256: None,
        },
        FrameworkPropagationOperation {
            phase: "stage_read_back",
            argv: read_back,
            mutates: false,
            requires_privilege: false,
            proof: Some("seed_manifest_digest_read_back"),
            expected_stdout_sha256: Some(expected_runtime_basket_digest.to_string()),
        },
        FrameworkPropagationOperation {
            phase: "machine_enrollment",
            argv: remote_execution
                .exec_argv(ssh_target, &[&format!("{remote_seed_dir}/enroll.sh")])
                .unwrap_or_default(),
            mutates: true,
            requires_privilege: true,
            proof: Some("machine_enrollment_completed"),
            expected_stdout_sha256: None,
        },
        FrameworkPropagationOperation {
            phase: "runtime_marker_read_back",
            argv: remote_execution
                .exec_argv(
                    ssh_target,
                    &[
                        "cat",
                        "/var/lib/cynic/runtime/current/.cynic-runtime-release.json",
                    ],
                )
                .unwrap_or_default(),
            mutates: false,
            requires_privilege: false,
            proof: None,
            expected_stdout_sha256: None,
        },
        FrameworkPropagationOperation {
            phase: "node_observation_read_back",
            argv: remote_execution
                .exec_argv(
                    ssh_target,
                    &[
                        "cat",
                        "/var/lib/cynic/bootstrap/node-registry-observation.json",
                    ],
                )
                .unwrap_or_default(),
            mutates: false,
            requires_privilege: false,
            proof: None,
            expected_stdout_sha256: None,
        },
    ]
}

fn remote_seed_dir(seed_id: &str) -> String {
    let seed_digest = seed_id.trim_start_matches("sha256:");
    format!("{REMOTE_SEED_ROOT}/{seed_digest}")
}

pub(super) fn valid_seed_id(seed_id: &str) -> bool {
    seed_id.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}
