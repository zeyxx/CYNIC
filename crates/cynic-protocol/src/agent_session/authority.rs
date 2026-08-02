use crate::digest_json;

use super::types::AgentSessionAuthority;

/// Content address of the authority document, excluding its self-referential
/// digest field.
pub fn agent_session_authority_digest(
    authority: &AgentSessionAuthority,
) -> Result<String, serde_json::Error> {
    let mut canonical = authority.clone();
    canonical.authority_digest.clear();
    digest_json(&canonical)
}
