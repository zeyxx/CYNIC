use cynic_protocol::{
    MutationContract, PolicyAdmissionReport, PolicyAuthority, PolicyDecision, PolicyIntentRef,
    PolicyScope, PolicySetRef, PolicyViolation, RequiredProof,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

<<<<<<< HEAD
use crate::execution::digest_json;

=======
>>>>>>> github/main
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostExecRequest {
    pub node: String,
    pub role: String,
    pub purpose: String,
    #[serde(alias = "command")]
    pub argv: Vec<String>,
    pub agent_id: String,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub service: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostExecValidation {
    pub ok: bool,
    pub violations: Vec<String>,
    pub admission: PolicyAdmissionReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostExecPolicyDocument {
    #[serde(default = "default_policy_id")]
    pub policy_id: String,
    #[serde(default = "default_policy_version")]
    pub policy_version: String,
    #[serde(default = "default_helper_path")]
    pub helper_path: String,
    #[serde(default = "default_sudoers_subject")]
    pub sudoers_subject: String,
    #[serde(default = "default_max_timeout_seconds")]
    pub max_timeout_seconds: u64,
    #[serde(default)]
    pub forbidden_tokens: Vec<String>,
    #[serde(default)]
    pub allowed_purposes: std::collections::BTreeMap<String, HostExecPurposePolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostExecPurposePolicy {
    #[serde(default)]
    pub allowed_roles: Vec<String>,
    #[serde(default)]
    pub argv_prefixes: Vec<Vec<String>>,
    #[serde(default)]
    pub requires_declared_service: bool,
    #[serde(default)]
    pub mutation: Option<HostExecMutationPolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostExecMutationPolicy {
    pub kind: String,
    pub mutates: bool,
    pub requires_privilege: bool,
    pub requires_explicit_apply: bool,
    pub required_authority: PolicyAuthority,
    #[serde(default)]
    pub preconditions: Vec<String>,
    pub rollback_intent: String,
    #[serde(default)]
    pub required_proofs: Vec<String>,
}

impl HostExecPolicyDocument {
    pub fn load(path: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read policy {}: {error}", path.display()))?;
        serde_json::from_str(&raw)
            .map_err(|error| format!("failed to parse policy {}: {error}", path.display()))
    }

    pub fn sudoers_rule(&self) -> String {
        format!(
            "{} ALL=(root) NOPASSWD: {}",
            self.sudoers_subject, self.helper_path
        )
    }

    pub fn validate_request(
        &self,
        request: &HostExecRequest,
        declared_services: &BTreeSet<String>,
    ) -> HostExecValidation {
        let mut violations = BTreeSet::new();

        if request.node.trim().is_empty() {
            violations.insert("missing_node".to_string());
        }
        if request.agent_id.trim().is_empty() {
            violations.insert("missing_agent_id".to_string());
        }
        if request.argv.is_empty() {
            violations.insert("missing_command".to_string());
        }
        if request.timeout_seconds > self.max_timeout_seconds {
            violations.insert("timeout_exceeds_policy".to_string());
        }

        let Some(purpose_policy) = self.allowed_purposes.get(&request.purpose) else {
            violations.insert("purpose_not_allowed".to_string());
            return self.validation_report(request, None, violations);
        };

        if !purpose_policy.allowed_roles.is_empty()
            && !purpose_policy
                .allowed_roles
                .iter()
                .any(|role| role == &request.role)
        {
            violations.insert("role_not_allowed_for_purpose".to_string());
        }

        let forbidden_tokens = self.forbidden_tokens.iter().collect::<BTreeSet<_>>();
        if request
            .argv
            .iter()
            .any(|token| forbidden_tokens.contains(token))
        {
            violations.insert("forbidden_token".to_string());
        }
        if request.argv.iter().any(|token| {
            ["|", "&&", ";", ">", "$(", "`"]
                .iter()
                .any(|shell| token.contains(shell))
        }) {
            violations.insert("shell_syntax_forbidden".to_string());
        }

        let prefix_allowed = purpose_policy.argv_prefixes.iter().any(|prefix| {
            request.argv.len() >= prefix.len()
                && request
                    .argv
                    .iter()
                    .zip(prefix.iter())
                    .all(|(left, right)| left == right)
        });
        if !prefix_allowed {
            violations.insert("argv_prefix_not_allowed".to_string());
        }

        if purpose_policy.requires_declared_service {
            let service = request
                .service
                .clone()
                .or_else(|| service_from_argv(&request.argv));
            match service {
                Some(service) if declared_services.contains(&service) => {}
                _ => {
                    violations.insert("service_not_declared".to_string());
                }
            }
        }

        self.validation_report(request, Some(purpose_policy), violations)
    }

    fn validation_report(
        &self,
        request: &HostExecRequest,
        purpose_policy: Option<&HostExecPurposePolicy>,
        violations: BTreeSet<String>,
    ) -> HostExecValidation {
        let policy_digest = digest_json(self);
        let intent_digest = digest_json(request);
        let mutation = purpose_policy
            .and_then(|policy| policy.mutation.as_ref())
            .map(|policy| mutation_contract(request, policy))
            .unwrap_or_else(|| unknown_mutation_contract(request));

        let decision = if violations.is_empty() {
            match purpose_policy.and_then(|policy| policy.mutation.as_ref()) {
                Some(_) => PolicyDecision::Allow {
                    required_authority: mutation.required_authority,
                    required_proofs: mutation.expected_proofs.clone(),
                },
                None => PolicyDecision::Unknown {
                    missing_evidence: vec!["purpose_mutation_contract".to_string()],
                },
            }
        } else {
            PolicyDecision::Deny {
                violations: violations
                    .iter()
                    .map(|code| PolicyViolation {
                        policy_id: self.policy_id.clone(),
                        code: code.clone(),
                        message: format!("host exec request violates `{code}`"),
                    })
                    .collect(),
            }
        };

        let admission = PolicyAdmissionReport::new(
            PolicySetRef {
                id: self.policy_id.clone(),
                version: self.policy_version.clone(),
                digest: policy_digest,
            },
            PolicyIntentRef {
                id: intent_digest.clone(),
                kind: request.purpose.clone(),
                actor: request.agent_id.clone(),
                subject: request.node.clone(),
                digest: intent_digest,
                plan_digest: digest_json(&request.argv),
            },
            mutation,
            decision,
        );

        HostExecValidation {
            ok: admission.admitted(),
            violations: violations.into_iter().collect(),
            admission,
        }
    }
}

fn mutation_contract(
    request: &HostExecRequest,
    policy: &HostExecMutationPolicy,
) -> MutationContract {
    MutationContract {
        kind: policy.kind.clone(),
        scope: PolicyScope {
            domain: "host".to_string(),
            target: request.node.clone(),
        },
        mutates: policy.mutates,
        requires_privilege: policy.requires_privilege,
        requires_explicit_apply: policy.requires_explicit_apply,
        required_authority: policy.required_authority,
        preconditions: policy.preconditions.clone(),
        rollback_intent: policy.rollback_intent.clone(),
        expected_proofs: policy
            .required_proofs
            .iter()
            .map(|gate| RequiredProof {
                gate: gate.clone(),
                subject: request.node.clone(),
            })
            .collect(),
    }
}

fn unknown_mutation_contract(request: &HostExecRequest) -> MutationContract {
    MutationContract {
        kind: "unknown".to_string(),
        scope: PolicyScope {
            domain: "host".to_string(),
            target: request.node.clone(),
        },
        mutates: true,
        requires_privilege: true,
        requires_explicit_apply: true,
        required_authority: PolicyAuthority::RuntimeAuthority,
        preconditions: vec!["purpose_mutation_contract".to_string()],
        rollback_intent: "unknown mutation; execution is prohibited".to_string(),
        expected_proofs: Vec::new(),
    }
}

<<<<<<< HEAD
=======
fn digest_json<T: Serialize>(value: &T) -> String {
    cynic_protocol::digest_json(value).unwrap_or_else(|_| "sha256:unavailable".to_string())
}

>>>>>>> github/main
pub fn service_from_argv(argv: &[String]) -> Option<String> {
    if argv.len() < 3 || argv.first().map(String::as_str) != Some("systemctl") {
        return None;
    }
    Some(argv[2].trim_end_matches(".service").to_string())
}

fn default_timeout_seconds() -> u64 {
    30
}

fn default_policy_id() -> String {
    "cynic.host_exec".to_string()
}

fn default_policy_version() -> String {
    "v1".to_string()
}

fn default_helper_path() -> String {
    "/usr/local/libexec/cynic-host-exec".to_string()
}

fn default_sudoers_subject() -> String {
    "cynic".to_string()
}

fn default_max_timeout_seconds() -> u64 {
    60
}

#[cfg(test)]
mod tests {
    use super::{HostExecPolicyDocument, HostExecRequest};
    use std::collections::BTreeSet;

    fn policy() -> HostExecPolicyDocument {
        serde_json::from_str(
            r#"{
              "helper_path": "/usr/local/libexec/cynic-host-exec",
              "sudoers_subject": "cynic",
              "max_timeout_seconds": 60,
              "allowed_purposes": {
                "read_only": {
                  "allowed_roles": ["cortex_host", "inference_node", "hypervisor"],
                  "argv_prefixes": [["hostname"], ["systemctl", "is-active"]],
                  "mutation": {
                    "kind": "observe",
                    "mutates": false,
                    "requires_privilege": false,
                    "requires_explicit_apply": false,
                    "required_authority": "read_only",
                    "rollback_intent": "no mutation to roll back"
                  }
                },
                "service_control": {
                  "allowed_roles": ["cortex_host"],
                  "argv_prefixes": [["systemctl", "restart"], ["systemctl", "status"]],
                  "requires_declared_service": true,
                  "mutation": {
                    "kind": "service_control",
                    "mutates": true,
                    "requires_privilege": true,
                    "requires_explicit_apply": true,
                    "required_authority": "human_operator",
                    "preconditions": ["service_declared"],
                    "rollback_intent": "restore the previous service state",
                    "required_proofs": ["service_active"]
                  }
                }
              },
              "forbidden_tokens": ["sh", "bash", "-c", "|", "&&", ";"]
            }"#,
        )
        .expect("policy")
    }

    #[test]
    fn allows_hostname_for_cortex_host() {
        let validation = policy().validate_request(
            &HostExecRequest {
                node: "cynic-host-cortex".to_string(),
                role: "cortex_host".to_string(),
                purpose: "read_only".to_string(),
                argv: vec!["hostname".to_string()],
                agent_id: "cortex-test".to_string(),
                timeout_seconds: 30,
                service: None,
            },
            &BTreeSet::new(),
        );

        assert!(validation.ok);
        assert!(validation.violations.is_empty());
        assert!(validation.admission.admitted());
        assert!(!validation.admission.mutation.mutates);
    }

    #[test]
    fn rejects_role_outside_allowed_purpose() {
        let validation = policy().validate_request(
            &HostExecRequest {
                node: "cynic-host-proxmox".to_string(),
                role: "hypervisor".to_string(),
                purpose: "service_control".to_string(),
                argv: vec![
                    "systemctl".to_string(),
                    "restart".to_string(),
                    "proxmox.service".to_string(),
                ],
                agent_id: "cortex-test".to_string(),
                timeout_seconds: 30,
                service: None,
            },
            &BTreeSet::from(["proxmox".to_string()]),
        );

        assert!(!validation.ok);
        assert!(!validation.admission.admitted());
        assert!(
            validation
                .violations
                .contains(&"role_not_allowed_for_purpose".to_string())
        );
    }

    #[test]
    fn admits_declared_service_with_explicit_authority_and_proof_contract() {
        let validation = policy().validate_request(
            &HostExecRequest {
                node: "cynic-host-cortex".to_string(),
                role: "cortex_host".to_string(),
                purpose: "service_control".to_string(),
                argv: vec![
                    "systemctl".to_string(),
                    "restart".to_string(),
                    "cynic-node.service".to_string(),
                ],
                agent_id: "cortex-test".to_string(),
                timeout_seconds: 30,
                service: None,
            },
            &BTreeSet::from(["cynic-node".to_string()]),
        );

        assert!(validation.ok);
        assert!(validation.admission.mutation.mutates);
        assert!(validation.admission.mutation.requires_explicit_apply);
        assert_eq!(
            validation.admission.mutation.required_authority,
            cynic_protocol::PolicyAuthority::HumanOperator
        );
        assert_eq!(
            validation.admission.mutation.expected_proofs,
            vec![cynic_protocol::RequiredProof {
                gate: "service_active".to_string(),
                subject: "cynic-host-cortex".to_string(),
            }]
        );
        assert!(
            validation
                .admission
                .policy_set
                .digest
                .starts_with("sha256:")
        );
        assert!(validation.admission.intent.digest.starts_with("sha256:"));
    }

    #[test]
    fn service_control_requires_declared_service() {
        let validation = policy().validate_request(
            &HostExecRequest {
                node: "cynic-host-cortex".to_string(),
                role: "cortex_host".to_string(),
                purpose: "service_control".to_string(),
                argv: vec![
                    "systemctl".to_string(),
                    "restart".to_string(),
                    "cynic-node.service".to_string(),
                ],
                agent_id: "cortex-test".to_string(),
                timeout_seconds: 30,
                service: None,
            },
            &BTreeSet::from(["surrealdb".to_string()]),
        );

        assert!(!validation.ok);
        assert!(!validation.admission.admitted());
        assert!(
            validation
                .violations
                .contains(&"service_not_declared".to_string())
        );
    }

    #[test]
    fn rejects_forbidden_shell_syntax() {
        let validation = policy().validate_request(
            &HostExecRequest {
                node: "cynic-host-cortex".to_string(),
                role: "cortex_host".to_string(),
                purpose: "read_only".to_string(),
                argv: vec!["hostname;id".to_string()],
                agent_id: "cortex-test".to_string(),
                timeout_seconds: 30,
                service: None,
            },
            &BTreeSet::new(),
        );

        assert!(!validation.ok);
        assert!(!validation.admission.admitted());
        assert!(
            validation
                .violations
                .contains(&"shell_syntax_forbidden".to_string())
        );
    }

    #[test]
    fn missing_mutation_contract_is_unknown_and_fail_closed() {
        let mut policy = policy();
        policy
            .allowed_purposes
            .get_mut("read_only")
            .expect("read_only purpose")
            .mutation = None;

        let validation = policy.validate_request(
            &HostExecRequest {
                node: "cynic-host-cortex".to_string(),
                role: "cortex_host".to_string(),
                purpose: "read_only".to_string(),
                argv: vec!["hostname".to_string()],
                agent_id: "cortex-test".to_string(),
                timeout_seconds: 30,
                service: None,
            },
            &BTreeSet::new(),
        );

        assert!(!validation.ok);
        assert!(validation.violations.is_empty());
        assert_eq!(
            validation.admission.decision,
            cynic_protocol::PolicyDecision::Unknown {
                missing_evidence: vec!["purpose_mutation_contract".to_string()]
            }
        );
    }
}
