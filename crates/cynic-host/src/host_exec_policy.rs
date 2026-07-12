use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostExecPolicyDocument {
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
            return HostExecValidation {
                ok: false,
                violations: violations.into_iter().collect(),
            };
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

        HostExecValidation {
            ok: violations.is_empty(),
            violations: violations.into_iter().collect(),
        }
    }
}

pub fn service_from_argv(argv: &[String]) -> Option<String> {
    if argv.len() < 3 || argv.first().map(String::as_str) != Some("systemctl") {
        return None;
    }
    Some(argv[2].trim_end_matches(".service").to_string())
}

fn default_timeout_seconds() -> u64 {
    30
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
                  "argv_prefixes": [["hostname"], ["systemctl", "is-active"]]
                },
                "service_control": {
                  "allowed_roles": ["cortex_host"],
                  "argv_prefixes": [["systemctl", "restart"], ["systemctl", "status"]],
                  "requires_declared_service": true
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
        assert!(
            validation
                .violations
                .contains(&"role_not_allowed_for_purpose".to_string())
        );
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
        assert!(
            validation
                .violations
                .contains(&"shell_syntax_forbidden".to_string())
        );
    }
}
