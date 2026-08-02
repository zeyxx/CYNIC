use std::path::PathBuf;
use std::sync::Arc;

use crate::domain::agent_session::AgentSessionRuntime;
use crate::infra::agent_session_sqlite::SqliteAgentSessionStore;
use cynic_protocol::{
    AGENT_SESSION_AUTHORITY_SCHEMA, AgentSessionAuthority, agent_session_authority_digest,
};

pub fn load_agent_session_runtime() -> Result<Option<Arc<AgentSessionRuntime>>, String> {
    let authority_path = declared_authority_path();
    if !authority_path.is_file() {
        tracing::warn!(
            path = %authority_path.display(),
            "agent-session authority absent; local agent sessions remain inactive"
        );
        return Ok(None);
    }

    let raw = std::fs::read_to_string(&authority_path).map_err(|error| {
        format!(
            "failed to read agent-session authority {}: {error}",
            authority_path.display()
        )
    })?;
    let authority: AgentSessionAuthority = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "invalid agent-session authority {}: {error}",
            authority_path.display()
        )
    })?;
    validate_authority(&authority)?;

    let store_path = declared_store_path();
    let store = SqliteAgentSessionStore::open(&store_path).map_err(|error| {
        format!(
            "failed to open agent-session store {}: {error}",
            store_path.display()
        )
    })?;
    let node_id = crate::api::rest::discovery::observed_node_id();
    let boot_id = observed_boot_id()?;
    if authority
        .enrollments
        .iter()
        .any(|enrollment| enrollment.node_id != node_id)
    {
        return Err(format!(
            "agent-session authority contains an enrollment outside observed node `{node_id}`"
        ));
    }

    Ok(Some(Arc::new(AgentSessionRuntime {
        authority,
        store: Arc::new(store),
        node_id,
        boot_id,
    })))
}

fn validate_authority(authority: &AgentSessionAuthority) -> Result<(), String> {
    if authority.schema != AGENT_SESSION_AUTHORITY_SCHEMA || authority.revision == 0 {
        return Err("agent-session authority schema or revision is invalid".to_string());
    }
    let observed = agent_session_authority_digest(authority)
        .map_err(|error| format!("agent-session authority cannot be digested: {error}"))?;
    if observed != authority.authority_digest {
        return Err("agent-session authority content digest does not match read-back".to_string());
    }
    if authority.enrollments.is_empty() {
        return Err("agent-session authority has no enrolled principal".to_string());
    }
    Ok(())
}

fn declared_authority_path() -> PathBuf {
    std::env::var_os("CYNIC_AGENT_SESSION_AUTHORITY_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            cynic_host::runtime_authority::default_runtime_authority_paths()
                .config_root
                .join("agent-session-authority.json")
        })
}

fn declared_store_path() -> PathBuf {
    std::env::var_os("CYNIC_AGENT_SESSION_STORE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            cynic_host::runtime_authority::default_runtime_principal_layout()
                .state_root
                .join("agent-sessions.sqlite")
        })
}

fn observed_boot_id() -> Result<String, String> {
    let path = std::env::var_os("CYNIC_BOOT_ID_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/proc/sys/kernel/random/boot_id"));
    std::fs::read_to_string(&path)
        .map(|value| value.trim().to_string())
        .map_err(|error| {
            format!(
                "failed to observe boot identity {}: {error}",
                path.display()
            )
        })
        .and_then(|value| {
            if value.is_empty() {
                Err("observed boot identity is empty".to_string())
            } else {
                Ok(value)
            }
        })
}
