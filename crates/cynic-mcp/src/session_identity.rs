use serde::Serialize;

const MAX_COMPONENT_LEN: usize = 12;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionIdentitySource {
    DeclaredEnvironment,
    GeneratedProcess,
}

#[derive(Clone, Debug, Serialize)]
pub struct SessionIdentity {
    pub agent_id: String,
    pub agent_type: String,
    pub source: SessionIdentitySource,
    pub stable_for: &'static str,
    pub secret: bool,
}

impl SessionIdentity {
    pub fn resolve() -> Result<Self, String> {
        Self::resolve_from(
            std::env::var("CYNIC_AGENT_ID").ok(),
            std::env::var("CYNIC_AGENT_TYPE").ok(),
            observed_node_component(),
            observed_boot_component(),
            || uuid::Uuid::new_v4().simple().to_string(),
        )
    }

    fn resolve_from<F>(
        declared_id: Option<String>,
        declared_type: Option<String>,
        node: String,
        boot: String,
        nonce: F,
    ) -> Result<Self, String>
    where
        F: FnOnce() -> String,
    {
        let agent_type = component(declared_type.as_deref().unwrap_or("codex"), "agent");
        if let Some(agent_id) = declared_id.filter(|value| !value.trim().is_empty()) {
            validate(&agent_id)?;
            return Ok(Self {
                agent_id,
                agent_type,
                source: SessionIdentitySource::DeclaredEnvironment,
                stable_for: "mcp_process",
                secret: false,
            });
        }

        let agent_id = format!(
            "{}:{}:{}:{}",
            agent_type,
            component(&node, "node"),
            component(&boot, "boot"),
            component(&nonce(), "session")
        );
        validate(&agent_id)?;
        Ok(Self {
            agent_id,
            agent_type,
            source: SessionIdentitySource::GeneratedProcess,
            stable_for: "mcp_process",
            secret: false,
        })
    }
}

fn component(value: &str, fallback: &str) -> String {
    let normalized: String = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .take(MAX_COMPONENT_LEN)
        .collect();
    if normalized.is_empty() {
        fallback.to_string()
    } else {
        normalized
    }
}

fn observed_node_component() -> String {
    std::env::var("CYNIC_NODE_ID")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "node".to_string())
}

fn observed_boot_component() -> String {
    std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| "boot".to_string())
}

fn validate(agent_id: &str) -> Result<(), String> {
    if agent_id.is_empty() || agent_id.chars().count() > 64 {
        return Err("agent_id must be 1-64 characters".to_string());
    }
    if agent_id.chars().any(char::is_control) {
        return Err("agent_id contains invalid characters".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_a_declared_principal() {
        let identity = SessionIdentity::resolve_from(
            Some("human-approved-agent".into()),
            Some("codex".into()),
            "node-a".into(),
            "boot-a".into(),
            || "unused".into(),
        )
        .unwrap();
        assert_eq!(identity.agent_id, "human-approved-agent");
        assert!(matches!(
            identity.source,
            SessionIdentitySource::DeclaredEnvironment
        ));
    }

    #[test]
    fn concurrent_processes_get_distinct_principals() {
        let first = SessionIdentity::resolve_from(
            None,
            Some("codex".into()),
            "node-a".into(),
            "boot-a".into(),
            || "nonce-one".into(),
        )
        .unwrap();
        let second = SessionIdentity::resolve_from(
            None,
            Some("codex".into()),
            "node-a".into(),
            "boot-a".into(),
            || "nonce-two".into(),
        )
        .unwrap();
        assert_ne!(first.agent_id, second.agent_id);
        assert!(first.agent_id.starts_with("codex:node-a:boot-a:"));
    }

    #[test]
    fn generated_principal_is_bounded_for_long_host_values() {
        let identity = SessionIdentity::resolve_from(
            None,
            Some("codex-worker-with-a-long-name".into()),
            "node-with-a-very-long-provider-name".into(),
            "boot-identifier-that-is-also-long".into(),
            || "1234567890abcdef1234567890abcdef".into(),
        )
        .unwrap();
        assert!(identity.agent_id.len() <= 64);
    }
}
