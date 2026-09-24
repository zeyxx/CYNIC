//! Pure Forgejo observation port — trait + plan builder.
//!
//! I/O-free so it's testable without a host. The concrete observer
//! (`RuntimeForgejoObserver`) lives in `cynic-host` (needs `repo_access`).

use crate::{
    ForgejoDesiredState, ForgejoObservedState, ForgejoPlan, ForgejoTokenSpec,
    FORGEJO_TOKEN_SPEC_SCHEMA, plan_forgejo_convergence,
};

/// Observation port for Forgejo. Given a desired state, produce an observed state.
pub trait ForgejoObservationPort: Send + Sync {
    fn observe(&self, desired: &ForgejoDesiredState) -> ForgejoObservedState;
}

/// Build a ForgejoPlan from desired state + observer. Pure — no I/O.
pub fn build_forgejo_plan(
    desired: ForgejoDesiredState,
    observer: &dyn ForgejoObservationPort,
) -> ForgejoPlan {
    let observed = observer.observe(&desired);
    plan_forgejo_convergence(desired, &observed)
}

/// Build a ForgejoTokenSpec from the desired state + an optional token path.
pub fn forgejo_token_spec(
    desired: &ForgejoDesiredState,
    token_path: Option<&str>,
) -> ForgejoTokenSpec {
    ForgejoTokenSpec {
        schema: FORGEJO_TOKEN_SPEC_SCHEMA.to_string(),
        principal: desired.machine_principal.clone(),
        api_url: desired
            .api_url
            .strip_suffix("/api/v1")
            .unwrap_or(&desired.api_url)
            .trim_end_matches('/')
            .to_string(),
        target_path: token_path
            .map(|p| p.to_string())
            .unwrap_or_else(|| "/etc/cynic/secrets/forgejo-token".to_string()),
        mode: 0o640,
        scopes: vec![
            "write:repository".to_string(),
            "write:organization".to_string(),
            "write:admin".to_string(),
        ],
        expiry_seconds: Some(7_776_000),
    }
}
