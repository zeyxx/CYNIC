use serde::Deserialize;
use std::path::Path;

use crate::registry::resolve_build_runner_transport;

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
) -> Result<FrameworkPropagationPlan, String> {
    let transport = resolve_build_runner_transport(registry_path)?;
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
    let manifest = if manifest_path.is_file() {
        let content = std::fs::read(&manifest_path).map_err(|error| {
            format!(
                "failed to read seed manifest {}: {error}",
                manifest_path.display()
            )
        })?;
        Some(
            serde_json::from_slice::<SeedManifest>(&content)
                .map_err(|error| format!("invalid seed manifest: {error}"))?,
        )
    } else {
        None
    };
    if manifest
        .as_ref()
        .is_some_and(|manifest| manifest.enrollment.target_role != transport.role)
    {
        blockers.push("seed_role_does_not_match_registry_role".to_string());
    }
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
    let operations = if blockers.is_empty() {
        propagation_operations(bundle_dir, &transport.ssh_target, &seed_id)
    } else {
        Vec::new()
    };

    Ok(FrameworkPropagationPlan {
        schema: "cynic.framework_bootstrap.propagation_plan.v1",
        status: if blockers.is_empty() {
            "planned"
        } else {
            "blocked"
        },
        mutates: false,
        node_id: transport.node_id,
        role: transport.role,
        ssh_target: transport.ssh_target,
        trust: transport.trust,
        bundle_dir: bundle_dir.to_string(),
        seed_id,
        blockers,
        operations,
        stop_condition: "stop on the first non-zero transport, install, integrity, or read-back command",
        rollback_intent: "remove the transferred seed; installed role activation remains forbidden until node read-back and enrollment admission succeed",
    })
}

pub(super) fn propagation_operations(
    bundle_dir: &str,
    ssh_target: &str,
    seed_id: &str,
) -> Vec<FrameworkPropagationOperation> {
    let seed_digest = seed_id.trim_start_matches("sha256:");
    let remote_seed_dir = format!("{REMOTE_SEED_ROOT}/{seed_digest}");
    let ssh_base = || {
        vec![
            "ssh".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            ssh_target.to_string(),
        ]
    };
    let mut prepare = ssh_base();
    prepare.extend([
        "mkdir".to_string(),
        "-p".to_string(),
        remote_seed_dir.clone(),
    ]);
    let mut install = ssh_base();
    install.extend(["sudo".to_string(), format!("{remote_seed_dir}/install.sh")]);
    let mut verify = ssh_base();
    verify.extend(["sudo".to_string(), format!("{remote_seed_dir}/verify.sh")]);
    let mut read_back = ssh_base();
    read_back.extend([
        "sudo".to_string(),
        "cat".to_string(),
        "/var/lib/cynic/bootstrap/node-registry-observation.json".to_string(),
    ]);

    vec![
        FrameworkPropagationOperation {
            phase: "preflight",
            argv: vec![
                "ssh".to_string(),
                "-o".to_string(),
                "BatchMode=yes".to_string(),
                "-o".to_string(),
                "StrictHostKeyChecking=yes".to_string(),
                ssh_target.to_string(),
                "true".to_string(),
            ],
            mutates: false,
            requires_privilege: false,
            proof: Some("ssh_identity_and_authentication_observed"),
        },
        FrameworkPropagationOperation {
            phase: "prepare",
            argv: prepare,
            mutates: true,
            requires_privilege: false,
            proof: None,
        },
        FrameworkPropagationOperation {
            phase: "transfer",
            argv: vec![
                "scp".to_string(),
                "-r".to_string(),
                "-o".to_string(),
                "BatchMode=yes".to_string(),
                "-o".to_string(),
                "StrictHostKeyChecking=yes".to_string(),
                format!("{bundle_dir}/."),
                format!("{ssh_target}:{remote_seed_dir}"),
            ],
            mutates: true,
            requires_privilege: false,
            proof: Some("seed_transferred"),
        },
        FrameworkPropagationOperation {
            phase: "install",
            argv: install,
            mutates: true,
            requires_privilege: true,
            proof: Some("seed_artifacts_integrity_verified"),
        },
        FrameworkPropagationOperation {
            phase: "read_back",
            argv: verify,
            mutates: false,
            requires_privilege: true,
            proof: Some("bootstrap_verification_completed"),
        },
        FrameworkPropagationOperation {
            phase: "evidence_read_back",
            argv: read_back,
            mutates: false,
            requires_privilege: true,
            proof: None,
        },
    ]
}

pub(super) fn valid_seed_id(seed_id: &str) -> bool {
    seed_id.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}
