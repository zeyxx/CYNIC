use std::path::PathBuf;

use crate::surface_audit::types::ExternalProjectionFinding;

pub(crate) fn audit_external_projection_files() -> Vec<ExternalProjectionFinding> {
    vec![
        audit_projection_file(
            "systemd_kernel_user_unit",
            "infra/systemd/cynic-kernel.service",
            "operator_projection",
            &[
                "WorkingDirectory=/var/lib/cynic",
                "ExecStart=/usr/local/bin/cynic-kernel",
                "EnvironmentFile=/etc/cynic/env",
            ],
            "runtime systemd projection is explicitly labeled as operator_projection",
            "operator_projection systemd projection should avoid user home scoped paths and declare operator_projection explicitly",
        ),
        audit_projection_file(
            "systemd_node_user_unit",
            "infra/systemd/cynic-node.service",
            "operator_projection",
            &[
                "WorkingDirectory=/var/lib/cynic",
                "ExecStart=/usr/local/bin/cynic-node control --config /etc/cynic/node-control.toml",
                "EnvironmentFile=/etc/cynic/env",
            ],
            "runtime systemd projection is explicitly labeled as operator_projection",
            "operator_projection systemd projection should avoid user home scoped paths and declare operator_projection explicitly",
        ),
        audit_projection_file(
            "systemd_kernel_installed_unit",
            "infra/systemd/installed/cynic-kernel.service",
            "runtime_authority_adapter",
            &[
                "User=cynic",
                "Group=cynic",
                "WorkingDirectory=/var/lib/cynic",
                "ExecStart=/usr/local/bin/cynic-kernel",
                "Environment=\"CYNIC_REGISTRY=/etc/cynic/registry.json\"",
                "Environment=\"CYNIC_INFERENCE_CONFIG=/etc/cynic/inference.toml\"",
                "Environment=\"CYNIC_STORAGE_CONFIG=/etc/cynic/storage.toml\"",
            ],
            "installed kernel systemd unit is explicitly labeled as runtime_authority_adapter",
            "installed kernel systemd unit must use the cynic principal, installed paths, and declare runtime_authority_adapter explicitly",
        ),
        audit_projection_file(
            "systemd_node_installed_unit",
            "infra/systemd/installed/cynic-node.service",
            "runtime_authority_adapter",
            &[
                "User=cynic",
                "Group=cynic",
                "WorkingDirectory=/var/lib/cynic",
                "ExecStart=/usr/local/bin/cynic-node control --config /etc/cynic/node-control.toml",
                "Environment=\"CYNIC_REGISTRY=/etc/cynic/registry.json\"",
                "Environment=\"CYNIC_INFERENCE_CONFIG=/etc/cynic/inference.toml\"",
            ],
            "installed node systemd unit is explicitly labeled as runtime_authority_adapter",
            "installed node systemd unit must use the cynic principal, installed paths, and declare runtime_authority_adapter explicitly",
        ),
    ]
}

fn audit_projection_file(
    id: &'static str,
    relative_path: &'static str,
    projection_class: &'static str,
    required_markers: &[&'static str],
    ok_detail: &'static str,
    blocked_detail: &'static str,
) -> ExternalProjectionFinding {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path);
    let file_path = path.display().to_string();
    let Ok(source) = std::fs::read_to_string(&path) else {
        return ExternalProjectionFinding {
            id,
            status: "blocked",
            projection_class,
            file_path,
            evidence: vec!["file_missing"],
            detail: "cannot read external projection file".to_string(),
        };
    };
    let matched_markers = required_markers
        .iter()
        .copied()
        .filter(|marker| source.contains(marker))
        .collect::<Vec<_>>();
    let projection_marker = format!("# projection_class: {projection_class}");
    let has_projection_marker = source.contains(&projection_marker);
    let status = if matched_markers.len() == required_markers.len() && has_projection_marker {
        "ok"
    } else {
        "blocked"
    };
    let mut evidence = matched_markers;
    if has_projection_marker {
        evidence.push(match projection_class {
            "operator_projection" => "# projection_class: operator_projection",
            "runtime_authority_adapter" => "# projection_class: runtime_authority_adapter",
            _ => "projection_class_marker",
        });
    }
    ExternalProjectionFinding {
        id,
        status,
        projection_class,
        file_path,
        evidence,
        detail: if status == "ok" {
            ok_detail.to_string()
        } else {
            blocked_detail.to_string()
        },
    }
}
