use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConnectionObservation {
    pub agent_id: String,
    pub kernel_reachable: bool,
    pub http_status: Option<u16>,
    pub response_body: String,
    pub local_credential_present: bool,
    pub registered: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentConnectionReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub agent_id: String,
    pub kernel_reachable: bool,
    pub kernel_authentication: &'static str,
    pub registered: Option<bool>,
    pub local_credential_present: bool,
    pub blockers: Vec<&'static str>,
    pub next_action: Option<&'static str>,
    pub http_status: Option<u16>,
    pub secret_exposed: bool,
}

pub fn assess_agent_connection(observed: AgentConnectionObservation) -> AgentConnectionReport {
    let common = |status, authentication, blockers, next_action| AgentConnectionReport {
        schema: "cynic.agent_connection_report.v1",
        status,
        agent_id: observed.agent_id.clone(),
        kernel_reachable: observed.kernel_reachable,
        kernel_authentication: authentication,
        registered: observed.registered,
        local_credential_present: observed.local_credential_present,
        blockers,
        next_action,
        http_status: observed.http_status,
        secret_exposed: false,
    };

    if !observed.kernel_reachable {
        return common(
            "blocked_disconnected",
            "unknown",
            vec!["kernel_unreachable"],
            Some("repair the declared kernel transport, then repeat read-back"),
        );
    }
    if observed
        .http_status
        .is_some_and(|status| (200..300).contains(&status))
    {
        return if observed.registered == Some(true) {
            common("ready", "accepted", Vec::new(), None)
        } else {
            common(
                "registration_required",
                "accepted",
                vec!["agent_not_registered"],
                Some("call cynic_coord_register, then repeat cynic_agent_connection_status"),
            )
        };
    }
    if observed
        .response_body
        .contains("No workload identity or compatibility bearer authority configured")
        || observed
            .response_body
            .contains("API keys not configured and no signature provided")
    {
        return common(
            "authentication_required",
            "kernel_authority_missing",
            vec!["agent_session_authority_missing"],
            Some(
                "observe the local peer, admit its delegation through the agent-session authority, then repeat read-back",
            ),
        );
    }
    if observed.local_credential_present {
        common(
            "authentication_required",
            "rejected",
            vec!["kernel_rejected_agent_session"],
            Some(
                "read back the agent-session admission, expiry, capability subset, and revocation state",
            ),
        )
    } else {
        common(
            "authentication_required",
            "rejected",
            vec!["mcp_agent_session_missing"],
            Some(
                "bootstrap a bounded MCP session through the Rust local-transport adapter, then repeat read-back",
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentConnectionObservation, assess_agent_connection};

    #[test]
    fn missing_kernel_auth_authority_is_not_reported_as_disconnected() {
        let report = assess_agent_connection(AgentConnectionObservation {
            agent_id: "codex-session".to_string(),
            kernel_reachable: true,
            http_status: Some(401),
            response_body: "No workload identity or compatibility bearer authority configured"
                .to_string(),
            local_credential_present: true,
            registered: None,
        });
        assert_eq!(report.status, "authentication_required");
        assert_eq!(report.kernel_authentication, "kernel_authority_missing");
        assert_eq!(report.blockers, vec!["agent_session_authority_missing"]);
        assert!(!report.secret_exposed);
    }
}
