//! Derive the secret-use policy from the declarative RBAC in `infra/agent-roles.json`.
//!
//! This loader exists so `secret_source_class` and `domains.secrets.mutation_policy`
//! stop being declarations without an executor. It reads the SAME file the
//! `cynic agent soc` surface already reads; it invents no new contract and no
//! new authority.
//!
//! Derivation, per role in `roles`:
//!   secret_classes  <- parse_secret_source_class(role.secret_source_class)
//!   purposes        <- SecretUsePurpose::consumption_set() when the role declares
//!                      at least one class; empty otherwise (declares nothing)
//!   requires_claim  <- true (value exposure always needs a claim)
//!
//! And once, for the policy:
//!   mutation_policy <- SecretMutationPolicy::parse(domains.secrets.mutation_policy)
//!
//! A missing or unparseable file yields `SecretUsePolicy::deny_all()` — the gate
//! must never fail open because a declaration could not be read.

use std::path::Path;

use cynic_protocol::secret_use::{
    SecretMutationPolicy, SecretUseGrant, SecretUsePolicy, SecretUsePurpose,
    parse_secret_source_class,
};

/// The declaration file this loader consumes. Single source, already read by
/// `cynic agent soc`; named here so the dependency is explicit.
pub const AGENT_ROLES_DECLARATION: &str = "infra/agent-roles.json";

/// Derive the policy from a parsed `agent-roles.json` document.
///
/// Pure: takes the JSON text, returns the policy. No I/O.
pub fn derive_policy_from_declaration(content: &str) -> SecretUsePolicy {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(content) else {
        return SecretUsePolicy::deny_all();
    };

    let mutation_policy = root
        .get("domains")
        .and_then(|d| d.get("secrets"))
        .and_then(|s| s.get("mutation_policy"))
        .and_then(|m| m.as_str())
        .map(SecretMutationPolicy::parse)
        // No declared mutation policy: fail closed.
        .unwrap_or(SecretMutationPolicy::ReadOnly);

    let Some(roles) = root.get("roles").and_then(|r| r.as_object()) else {
        return SecretUsePolicy::deny_all();
    };

    let mut grants = Vec::new();
    for (role_name, role_def) in roles {
        let raw_source = role_def
            .get("secret_source_class")
            .and_then(|s| s.as_str())
            .unwrap_or("None");

        let secret_classes = parse_secret_source_class(raw_source);

        // A role that declares no class consumes nothing; its grant carries an
        // empty purpose set so every request against it is denied on purpose.
        let purposes = if secret_classes.is_empty() {
            Vec::new()
        } else {
            SecretUsePurpose::consumption_set()
        };

        grants.push(SecretUseGrant {
            role: role_name.clone(),
            secret_classes,
            purposes,
            requires_claim: true,
        });
    }

    SecretUsePolicy::new(grants, mutation_policy)
}

/// Read the declaration file and derive the policy. A missing file denies all.
pub fn load_policy_from_file(path: &Path) -> SecretUsePolicy {
    match std::fs::read_to_string(path) {
        Ok(content) => derive_policy_from_declaration(&content),
        Err(_) => SecretUsePolicy::deny_all(),
    }
}

/// Resolve the declaration path from the workspace root.
///
/// `CYNIC_WORKSPACE_ROOT` overrides; otherwise `explicit_root` is used. Returns
/// `None` when neither yields a readable declaration, so callers can report an
/// explicit unknown rather than a silent empty policy.
pub fn resolve_declaration_path(explicit_root: Option<&Path>) -> Option<std::path::PathBuf> {
    let root = std::env::var("CYNIC_WORKSPACE_ROOT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| explicit_root.map(std::path::Path::to_path_buf))?;

    let candidate = root.join(AGENT_ROLES_DECLARATION);
    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::secret_reference::SecretClass;

    /// The real declaration, read from the repository at test time. This keeps
    /// the test honest: if the declaration changes shape, this fails.
    fn real_declaration() -> String {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../infra/agent-roles.json");
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    #[test]
    fn real_declaration_derives_read_only_mutation_policy() {
        let policy = derive_policy_from_declaration(&real_declaration());
        assert_eq!(policy.mutation_policy, SecretMutationPolicy::ReadOnly);
        assert!(
            !policy.mutation_policy.permits_mutation(),
            "the secrets domain declares read_only; mutation must be denied"
        );
    }

    #[test]
    fn scheduler_agent_consumes_runtime_machine_identity_only() {
        let policy = derive_policy_from_declaration(&real_declaration());
        let grant = policy
            .grants
            .iter()
            .find(|g| g.role == "scheduler_agent")
            .expect("scheduler_agent is declared");

        assert!(grant.permits_class(SecretClass::RuntimeMachineIdentity));
        assert!(!grant.permits_class(SecretClass::MachineEnrollment));
        assert!(!grant.permits_class(SecretClass::HumanAdmin));
    }

    #[test]
    fn roles_declaring_none_consume_nothing() {
        let policy = derive_policy_from_declaration(&real_declaration());
        for role in ["schema_warden", "surface_inspector", "throughput_auditor"] {
            let grant = policy
                .grants
                .iter()
                .find(|g| g.role == role)
                .unwrap_or_else(|| panic!("{role} is declared"));
            assert!(
                grant.secret_classes.is_empty(),
                "{role} declares secret_source_class None and must consume nothing"
            );
            assert!(
                grant.purposes.is_empty(),
                "{role} declaring no class must carry no permitted purpose"
            );
        }
    }

    #[test]
    fn build_runner_consumes_machine_enrollment() {
        let policy = derive_policy_from_declaration(&real_declaration());
        let grant = policy
            .grants
            .iter()
            .find(|g| g.role == "build_runner")
            .expect("build_runner is declared");
        assert!(grant.permits_class(SecretClass::MachineEnrollment));
        assert!(!grant.permits_class(SecretClass::RuntimeMachineIdentity));
    }

    #[test]
    fn human_admin_role_is_present_but_denied_by_the_decision_function() {
        use cynic_protocol::secret_reference::{SecretProvider, SecretReference};
        use cynic_protocol::secret_use::{
            SecretUseDecision, SecretUseDenialReason, SecretUseRequest, decide_secret_use,
        };

        let policy = derive_policy_from_declaration(&real_declaration());
        let req = SecretUseRequest::new(
            "agent-code",
            "human_admin",
            SecretReference {
                id: "root-password".to_string(),
                class: SecretClass::HumanAdmin,
                provider: SecretProvider::PrincipalOwnedFile,
                locator: "root-password".to_string(),
            },
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_1");

        assert_eq!(
            decide_secret_use(&req, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::HumanAdminRoleNotAssumable
            }
        );
    }

    #[test]
    fn unparseable_declaration_denies_all() {
        let policy = derive_policy_from_declaration("not json at all");
        assert!(policy.grants.is_empty());
        assert!(!policy.mutation_policy.permits_mutation());
    }

    #[test]
    fn missing_file_denies_all() {
        let policy = load_policy_from_file(std::path::Path::new("/nonexistent/agent-roles.json"));
        assert!(policy.grants.is_empty());
        assert!(!policy.mutation_policy.permits_mutation());
    }

    #[test]
    fn missing_mutation_policy_fails_closed() {
        let policy = derive_policy_from_declaration(
            r#"{"roles":{"some_role":{"secret_source_class":"RuntimeMachineIdentity"}}}"#,
        );
        assert_eq!(policy.mutation_policy, SecretMutationPolicy::ReadOnly);
        let grant = policy.grants.first().expect("one grant");
        assert!(grant.permits_class(SecretClass::RuntimeMachineIdentity));
    }
}
