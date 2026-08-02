use crate::domain::agent_session::{AgentSessionRuntime, local_transport_binding_sha256};
use cynic_protocol::{
    AGENT_SESSION_PEER_OBSERVATION_SCHEMA, AgentSessionPeerObservation, AgentSessionTransport,
    SecurityEvidenceState,
};

use super::super::local_peer::LocalPeerCredentials;

pub(super) fn observe(
    runtime: &AgentSessionRuntime,
    peer: &LocalPeerCredentials,
    now_unix_ms: u64,
) -> Result<(String, AgentSessionPeerObservation), &'static str> {
    if peer.process_id.is_none() || peer.process_start_time_ticks.is_none() {
        return Err("local_peer_process_start_unavailable");
    }
    let binding = local_transport_binding_sha256(
        &runtime.node_id,
        &runtime.boot_id,
        peer.process_id,
        peer.process_start_time_ticks,
        peer.user_id,
        peer.group_id,
    )
    .map_err(|_| "local_peer_binding_unavailable")?;
    Ok((
        binding.clone(),
        AgentSessionPeerObservation {
            schema: AGENT_SESSION_PEER_OBSERVATION_SCHEMA.to_string(),
            observed_at_unix_ms: now_unix_ms,
            node_id: runtime.node_id.clone(),
            transport: AgentSessionTransport::LocalUnixPeer,
            principal: Some(format!("linux-uid:{}", peer.user_id)),
            process_id: peer.process_id,
            user_id: Some(peer.user_id),
            group_id: Some(peer.group_id),
            transport_binding_sha256: Some(binding.clone()),
            peer_identity: SecurityEvidenceState::Proven,
            evidence_refs: vec![format!("linux-so-peercred:{binding}")],
            unknowns: Vec::new(),
        },
    ))
}
