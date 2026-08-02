use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{AgentEnrollmentPolicy, compose_agent_session_authority};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAuthorityBundleRequest {
    pub node_id: String,
    pub revision: u64,
    pub peer_observation_max_age_ms: u64,
    pub enrollments: Vec<AgentEnrollmentPolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentAuthorityBundleReport {
    pub schema: &'static str,
    pub status: String,
    pub output_path: String,
    pub node_id: String,
    pub authority_revision: u64,
    pub authority_digest: String,
    pub enrolled_principals: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

pub fn write_agent_authority_bundle(
    output_path: impl AsRef<Path>,
    request: &AgentAuthorityBundleRequest,
) -> Result<AgentAuthorityBundleReport, String> {
    let authority = compose_agent_session_authority(
        &request.node_id,
        request.revision,
        request.peer_observation_max_age_ms,
        &request.enrollments,
    )?;
    let output_path = output_path.as_ref();
    let parent = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        return Err(format!(
            "agent authority output directory is absent: {}",
            parent.display()
        ));
    }
    let bytes = serde_json::to_vec_pretty(&authority)
        .map_err(|error| format!("agent authority serialization failed: {error}"))?;
    atomic_write(output_path, &bytes)?;
    let read_back = fs::read(output_path)
        .map_err(|error| format!("agent authority read-back failed: {error}"))?;
    let observed: cynic_protocol::AgentSessionAuthority = serde_json::from_slice(&read_back)
        .map_err(|error| format!("agent authority read-back is invalid: {error}"))?;
    if observed != authority {
        return Err("agent authority read-back differs from desired content".to_string());
    }
    let enrolled_principals = authority
        .enrollments
        .iter()
        .map(|enrollment| enrollment.principal.clone())
        .collect();
    Ok(AgentAuthorityBundleReport {
        schema: "cynic.host.agent_authority_bundle_report.v1",
        status: "completed".to_string(),
        output_path: output_path.display().to_string(),
        node_id: authority
            .enrollments
            .first()
            .map(|enrollment| enrollment.node_id.clone())
            .unwrap_or_else(|| request.node_id.clone()),
        authority_revision: authority.revision,
        authority_digest: authority.authority_digest.clone(),
        enrolled_principals,
        evidence_refs: vec![
            format!("agent_authority_file:{}", output_path.display()),
            format!("agent_authority_digest:{}", authority.authority_digest),
        ],
        unknowns: Vec::new(),
    })
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = temporary_path(path);
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("agent authority staging failed: {error}"))?;
        file.write_all(bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("agent authority staging write failed: {error}"))?;
        fs::rename(&temporary, path)
            .map_err(|error| format!("agent authority activation failed: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn temporary_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("agent-session-authority.json");
    path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_access::default_local_agent_capabilities;

    #[test]
    fn bundle_write_is_content_addressed_and_read_back() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("authority.json");
        let report = write_agent_authority_bundle(
            &output,
            &AgentAuthorityBundleRequest {
                node_id: "node-a".to_string(),
                revision: 1,
                peer_observation_max_age_ms: 30_000,
                enrollments: vec![AgentEnrollmentPolicy {
                    observed_linux_uid: 1000,
                    agent_types: vec!["codex".to_string()],
                    max_ttl_ms: 900_000,
                    capabilities: default_local_agent_capabilities(),
                }],
            },
        )
        .unwrap();
        assert_eq!(report.status, "completed");
        assert_eq!(report.enrolled_principals, vec!["linux-uid:1000"]);
        assert!(output.is_file());
    }
}
