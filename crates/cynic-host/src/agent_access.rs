mod access_apply;
mod authority_apply;
mod authority_bundle;
mod authority_projection;
mod types;

// Domaine pur : L0 (cynic-protocol). Ré-exports de compat pour les callers hôte.
pub use access_apply::{AgentAccessApplyReport, apply_agent_access};
pub use authority_apply::{AgentAuthorityApplyReport, apply_agent_authority_projection};
pub use authority_bundle::{
    AgentAuthorityBundleReport, AgentAuthorityBundleRequest, write_agent_authority_bundle,
};
pub use authority_projection::{
    installed_agent_authority_spec, observe_agent_authority_projection,
};
pub use types::desired_agent_access_policy;
