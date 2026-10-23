//! Desired systemd unit for a node service — pure rendering and drift detection.
//!
//! The host reconciler (`service-reconcile`) owns unit lifecycle (install,
//! enable, start) but never authors unit content: `ServiceUnitPolicy` carries
//! no `ExecStart`, binary path, or config path. This module is the missing
//! declared source: it renders the exact unit file bytes from explicit
//! parameters and decides whether an installed file converges with them.
//! No I/O, no clock; the CLI surface owns all filesystem reads.

use cynic_types::paths::CynicPaths;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const NODE_SERVICE_UNIT_RENDER_SCHEMA: &str = "cynic.node_service.unit.render.v1";
pub const NODE_SERVICE_UNIT_DRIFT_SCHEMA: &str = "cynic.node_service.unit.drift.v1";

/// Parameters that fully determine a node service unit file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeServiceUnitParams {
    pub unit_name: String,
    pub description: String,
    pub binary_path: String,
    pub config_path: String,
    pub run_user: String,
    pub after_target: String,
}

impl NodeServiceUnitParams {
    pub fn new(
        unit_name: String,
        description: String,
        binary_path: String,
        config_path: String,
        run_user: String,
    ) -> Self {
        Self {
            unit_name,
            description,
            binary_path,
            config_path,
            run_user,
            after_target: "network-online.target".to_string(),
        }
    }
}

/// Fail-closed validation: every path must be absolute, the unit name must be
/// a single `*.service` segment, and no field may carry whitespace or shell
/// metacharacters that would change the rendered meaning.
pub fn validate_service_unit_params(params: &NodeServiceUnitParams) -> Result<(), String> {
    if params.unit_name.is_empty()
        || !params.unit_name.ends_with(".service")
        || params.unit_name.contains(['/', ' ', '\t', '\n'])
    {
        return Err(format!(
            "invalid unit_name {:?}: expected a single <name>.service segment",
            params.unit_name
        ));
    }
    for (field, value) in [
        ("binary_path", params.binary_path.as_str()),
        ("config_path", params.config_path.as_str()),
    ] {
        if !value.starts_with('/') || value.contains([' ', '\t', '\n', ';', '&', '|', '$', '`']) {
            return Err(format!(
                "invalid {field} {value:?}: expected an absolute path without whitespace or shell metacharacters"
            ));
        }
    }
    if params.run_user.is_empty() || params.run_user.contains(['/', ' ', '\t', '\n', ':', ';']) {
        return Err(format!(
            "invalid run_user {:?}: expected a bare account name",
            params.run_user
        ));
    }
    if params.description.contains('\n') {
        return Err("invalid description: must be a single line".to_string());
    }
    if params.after_target.is_empty() || params.after_target.contains(char::is_whitespace) {
        return Err(format!(
            "invalid after_target {:?}: expected a single target name",
            params.after_target
        ));
    }
    Ok(())
}

/// Render the exact unit file bytes. Deterministic: same params, same bytes.
pub fn render_service_unit(params: &NodeServiceUnitParams) -> Result<String, String> {
    validate_service_unit_params(params)?;
    Ok(format!(
        "[Unit]\nDescription={description}\nAfter={after}\n\n[Service]\nType=simple\nUser={user}\nWorkingDirectory={working_dir}\nExecStart={binary} control --config {config}\nRestart=always\nRestartSec=5\nNoNewPrivileges=true\n\n[Install]\nWantedBy=multi-user.target\n",
        description = params.description,
        after = params.after_target,
        user = params.run_user,
        working_dir = CynicPaths::state_dir().display(),
        binary = params.binary_path,
        config = params.config_path,
    ))
}

/// Digest of rendered bytes, for reports and read-back comparison.
pub fn service_unit_sha256(rendered: &str) -> String {
    format!(
        "sha256:{}",
        hex::encode(Sha256::digest(rendered.as_bytes()))
    )
}

/// Drift between the declared rendering and the installed file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceUnitDriftReport {
    pub schema: String,
    pub status: String,
    pub unit_name: String,
    pub expected_digest: String,
    pub installed_digest: Option<String>,
}

pub fn detect_service_unit_drift(
    params: &NodeServiceUnitParams,
    installed: Option<&str>,
) -> Result<ServiceUnitDriftReport, String> {
    let expected = render_service_unit(params)?;
    let expected_digest = service_unit_sha256(&expected);
    let (status, installed_digest) = match installed {
        None => ("missing".to_string(), None),
        Some(content) => {
            if content == expected {
                ("converged".to_string(), Some(service_unit_sha256(content)))
            } else {
                (
                    "content_mismatch".to_string(),
                    Some(service_unit_sha256(content)),
                )
            }
        }
    };
    Ok(ServiceUnitDriftReport {
        schema: NODE_SERVICE_UNIT_DRIFT_SCHEMA.to_string(),
        status,
        unit_name: params.unit_name.clone(),
        expected_digest,
        installed_digest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_params() -> NodeServiceUnitParams {
        NodeServiceUnitParams::new(
            "cynic-node.service".to_string(),
            "CYNIC node receiver".to_string(),
            "/var/lib/cynic/bin/cynic-node".to_string(),
            "/var/lib/cynic/node-control.toml".to_string(),
            "cynic".to_string(),
        )
    }

    #[test]
    fn render_is_deterministic_and_pinned() {
        let first = render_service_unit(&sample_params()).expect("valid params");
        let second = render_service_unit(&sample_params()).expect("valid params");
        assert_eq!(first, second);
        assert!(first.contains("ExecStart=/var/lib/cynic/bin/cynic-node control --config /var/lib/cynic/node-control.toml"));
        assert!(first.contains("User=cynic"));
    }

    #[test]
    fn relative_binary_path_is_refused() {
        let mut params = sample_params();
        params.binary_path = "bin/cynic-node".to_string();
        assert!(render_service_unit(&params).is_err());
    }

    #[test]
    fn shell_metacharacters_in_config_path_are_refused() {
        let mut params = sample_params();
        params.config_path = "/var/lib/cynic/node-control.toml; reboot".to_string();
        assert!(render_service_unit(&params).is_err());
    }

    #[test]
    fn non_service_unit_name_is_refused() {
        let mut params = sample_params();
        params.unit_name = "../evil.timer".to_string();
        assert!(render_service_unit(&params).is_err());
    }

    #[test]
    fn drift_missing_converged_and_mismatch() {
        let params = sample_params();
        let expected = render_service_unit(&params).expect("valid params");
        let missing = detect_service_unit_drift(&params, None).expect("drift");
        assert_eq!(missing.status, "missing");
        assert_eq!(missing.installed_digest, None);
        let converged = detect_service_unit_drift(&params, Some(&expected)).expect("drift");
        assert_eq!(converged.status, "converged");
        assert_eq!(
            converged.installed_digest,
            Some(converged.expected_digest.clone())
        );
        let mismatch = detect_service_unit_drift(&params, Some("# hand-edited\n")).expect("drift");
        assert_eq!(mismatch.status, "content_mismatch");
        assert_ne!(
            mismatch.installed_digest.expect("digest"),
            mismatch.expected_digest
        );
    }
}
