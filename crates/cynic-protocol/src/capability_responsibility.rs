use serde::Serialize;
use std::collections::BTreeSet;

pub const CAPABILITY_RESPONSIBILITY_CATALOG_SCHEMA: &str =
    "cynic.capability_responsibility_catalog.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilityResponsibilityContract {
    pub capability_id: &'static str,
    pub accountable_owner: &'static str,
    pub domain_owner: &'static str,
    pub state_owner: &'static str,
    pub authority_owner: &'static str,
    pub observation_owner: &'static str,
    pub execution_owner: &'static str,
    pub proof_owner: &'static str,
    pub lifecycle_owner: &'static str,
    pub resource_owner: &'static str,
    pub coordination_owner: &'static str,
    pub domain_rule: &'static str,
    pub lifecycle_rule: &'static str,
    pub resource_rule: &'static str,
    pub coordination_rule: &'static str,
    pub recovery_rule: &'static str,
    pub retention_rule: &'static str,
    pub collection_rule: &'static str,
    pub delegation_rule: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilityResponsibilityCatalog {
    pub schema: &'static str,
    pub contracts: Vec<CapabilityResponsibilityContract>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityResponsibilityViolation {
    pub capability_id: String,
    pub code: &'static str,
}

impl CapabilityResponsibilityCatalog {
    pub fn validate(&self) -> Vec<CapabilityResponsibilityViolation> {
        let mut violations = Vec::new();
        let mut ids = BTreeSet::new();
        if self.schema != CAPABILITY_RESPONSIBILITY_CATALOG_SCHEMA {
            violations.push(CapabilityResponsibilityViolation {
                capability_id: "catalog".into(),
                code: "invalid_schema",
            });
        }
        for contract in &self.contracts {
            if !ids.insert(contract.capability_id) {
                violations.push(violation(contract, "duplicate_capability"));
            }
            for (code, value) in [
                ("accountable_owner_missing", contract.accountable_owner),
                ("domain_owner_missing", contract.domain_owner),
                ("state_owner_missing", contract.state_owner),
                ("authority_owner_missing", contract.authority_owner),
                ("observation_owner_missing", contract.observation_owner),
                ("execution_owner_missing", contract.execution_owner),
                ("proof_owner_missing", contract.proof_owner),
                ("lifecycle_owner_missing", contract.lifecycle_owner),
                ("resource_owner_missing", contract.resource_owner),
                ("coordination_owner_missing", contract.coordination_owner),
                ("domain_rule_missing", contract.domain_rule),
                ("lifecycle_rule_missing", contract.lifecycle_rule),
                ("resource_rule_missing", contract.resource_rule),
                ("coordination_rule_missing", contract.coordination_rule),
                ("recovery_rule_missing", contract.recovery_rule),
                ("retention_rule_missing", contract.retention_rule),
                ("collection_rule_missing", contract.collection_rule),
                ("delegation_rule_missing", contract.delegation_rule),
            ] {
                if value.trim().is_empty() {
                    violations.push(violation(contract, code));
                }
            }
        }
        violations
    }
}

fn violation(
    contract: &CapabilityResponsibilityContract,
    code: &'static str,
) -> CapabilityResponsibilityViolation {
    CapabilityResponsibilityViolation {
        capability_id: contract.capability_id.into(),
        code,
    }
}

fn contract(
    capability_id: &'static str,
    accountable_owner: &'static str,
    domain_owner: &'static str,
    observation_owner: &'static str,
    execution_owner: &'static str,
    domain_rule: &'static str,
) -> CapabilityResponsibilityContract {
    CapabilityResponsibilityContract {
        capability_id,
        accountable_owner,
        domain_owner,
        state_owner: accountable_owner,
        authority_owner: accountable_owner,
        observation_owner,
        execution_owner,
        proof_owner: accountable_owner,
        lifecycle_owner: accountable_owner,
        resource_owner: accountable_owner,
        coordination_owner: accountable_owner,
        domain_rule,
        lifecycle_rule: "the accountable owner retains creation, recovery, retention, and collection duties",
        resource_rule: "resource admission and consumption must fit an observed owner-controlled budget",
        coordination_rule: "concurrent mutation requires an owner-issued expiring claim and monotonic fencing",
        recovery_rule: "the accountable owner recovers or blocks orphaned work after executor or node failure",
        retention_rule: "retention is explicit, reference-aware, and cannot silently discard required proof",
        collection_rule: "collection requires observed expiry, absence of protected roots, bounded action, and capacity read-back",
        delegation_rule: "a capability or lease delegates bounded execution but never transfers ownership",
    }
}

pub fn capability_responsibility_catalog() -> CapabilityResponsibilityCatalog {
    CapabilityResponsibilityCatalog {
        schema: CAPABILITY_RESPONSIBILITY_CATALOG_SCHEMA,
        contracts: vec![
            contract(
                "process",
                "cynic-host",
                "cynic-host::process",
                "cynic-host",
                "cynic-host",
                "admit only a declared principal, executable, and capability subset",
            ),
            contract(
                "principal",
                "cynic-host",
                "cynic-host::principal",
                "cynic-host",
                "cynic-host",
                "principal identity, privilege, ownership, and secret placement are host law",
            ),
            contract(
                "filesystem",
                "cynic-host",
                "cynic-host::filesystem",
                "cynic-host",
                "cynic-host",
                "declared paths, modes, atomicity, and read-back govern filesystem convergence",
            ),
            contract(
                "source",
                "cynic-node",
                "cynic-workload",
                "cynic-node",
                "cynic-node",
                "source mutation requires an admitted workload, preimage binding, and fenced workspace claim",
            ),
            contract(
                "build",
                "cynic-node",
                "cynic-protocol::build",
                "cynic-node",
                "build_runner",
                "only registry-selected admitted build inputs may execute the canonical gate",
            ),
            contract(
                "artifact",
                "cynic-host",
                "cynic-protocol::artifact_release",
                "cynic-host",
                "cynic-host",
                "only verified role-compatible artifacts may be activated or rolled back",
            ),
            contract(
                "service",
                "cynic-host",
                "cynic-host::service",
                "cynic-host",
                "cynic-host",
                "installed, active, healthy, and proven are distinct service states",
            ),
            contract(
                "host_os",
                "cynic-host",
                "cynic-host::runtime_authority",
                "cynic-host",
                "cynic-host-exec",
                "privileged host changes require allowlisted argv, consent, rollback intent, and postflight",
            ),
            contract(
                "network",
                "cynic-node",
                "cynic-node::network",
                "cynic-host",
                "cynic-node",
                "network policy preserves management reachability and separates intent from substrate adapters",
            ),
            contract(
                "node",
                "cynic-node",
                "cynic-node::runtime",
                "cynic-node",
                "cynic-node",
                "a node is admitted from registry intent plus node-local read-back, never declaration alone",
            ),
            contract(
                "provider",
                "cynic-node",
                "cynic-protocol::provider",
                "provider_adapter",
                "provider_adapter",
                "provider truth is injected through ports and cannot redefine node policy",
            ),
            contract(
                "orchestrator",
                "cynic-node",
                "cynic-workload",
                "orchestrator_adapter",
                "node_scheduler",
                "workload placement and execution remain admitted, scoped, budgeted, and recoverable",
            ),
            contract(
                "project",
                "cynic-host",
                "cynic-protocol::source_control",
                "cynic-host",
                "cynic-host",
                "project governance binds portable policy to an observed repository authority",
            ),
            contract(
                "fleet",
                "cynic-node",
                "cynic-node::fleet",
                "cynic-node",
                "node_scheduler",
                "fleet convergence aggregates node proofs without replacing node-local authority",
            ),
            contract(
                "federation",
                "cynic-kernel",
                "cynic-protocol::agent_recursion",
                "cynic-node",
                "peer_node",
                "cross-node work is a newly admitted peer request, never local child recursion",
            ),
            contract(
                "human_boundary",
                "cynic-host",
                "cynic-protocol::policy",
                "cynic-host",
                "authorized_operator",
                "human consent, bootstrap, and breakglass remain explicit and auditable",
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_catalog_is_complete_and_valid() {
        let catalog = capability_responsibility_catalog();
        assert_eq!(catalog.contracts.len(), 16);
        assert!(catalog.validate().is_empty());
    }

    #[test]
    fn delegation_never_transfers_ownership() {
        for contract in capability_responsibility_catalog().contracts {
            assert!(
                contract
                    .delegation_rule
                    .contains("never transfers ownership")
            );
        }
    }

    #[test]
    fn missing_domain_semantics_is_rejected() {
        let mut catalog = capability_responsibility_catalog();
        catalog.contracts[0].domain_rule = "";
        assert!(
            catalog
                .validate()
                .iter()
                .any(|item| item.code == "domain_rule_missing")
        );
    }

    #[test]
    fn lifecycle_duties_stay_with_the_accountable_owner() {
        for contract in capability_responsibility_catalog().contracts {
            assert_eq!(contract.state_owner, contract.accountable_owner);
            assert_eq!(contract.authority_owner, contract.accountable_owner);
            assert_eq!(contract.proof_owner, contract.accountable_owner);
            assert_eq!(contract.lifecycle_owner, contract.accountable_owner);
            assert_eq!(contract.resource_owner, contract.accountable_owner);
            assert_eq!(contract.coordination_owner, contract.accountable_owner);
            assert!(contract.collection_rule.contains("capacity read-back"));
        }
    }
}
