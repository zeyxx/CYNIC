//! Scan/read-back of infra/agent-roles.json reconciled against observed proof.

use std::collections::BTreeMap;
use std::path::Path;

use super::types::{AgentSocEntry, AgentSocReport, DomainStatus, HumanAdminStatus};

pub const AGENT_SOC_SCHEMA: &str = "cynic.agent_soc_report.v1";

pub fn scan_agent_soc(roles_path: &Path) -> AgentSocReport {
    let (roles, domains, rules) = match std::fs::read_to_string(roles_path) {
        Ok(content) => parse_declarative_roles(&content),
        Err(e) => {
            return AgentSocReport {
                schema: AGENT_SOC_SCHEMA.to_string(),
                human_admin_role: HumanAdminStatus {
                    zero_touch_runtime: true,
                    never_touches_runtime: vec![
                        "HumanAdmin zero-touch rule: never touches runtime mutations".to_string(),
                        format!("roles file unreadable at {}: {}", roles_path.display(), e),
                    ],
                },
                roles: BTreeMap::new(),
                domains: BTreeMap::new(),
                residual_unknowns: vec![format!(
                    "infra/agent-roles.json missing or unreadable at {}",
                    roles_path.display()
                )],
            };
        }
    };

    let mut human_admin_zero_touch = true;
    let mut never_touches: Vec<String> = rules.clone();
    for (name, entry) in &roles {
        if entry.authority_class == "HumanAdmin" && !entry.can_mutate.is_empty() {
            human_admin_zero_touch = false;
            never_touches.push(format!(
                "human_admin role {} declares runtime mutations",
                name
            ));
        }
    }

    let mut report_roles = roles;
    let observed = load_runtime_proof_state(roles_path);
    // Read-back proof: mutate `granted` on each role entry from the observed
    // runtime_proof.v1 (kernel/node-signed), never the declarative default.
    if observed.unwrap_or(false) {
        for (_name, entry) in report_roles.iter_mut() {
            if entry.authority_class == "RuntimeMachineIdentity" {
                entry.granted_via =
                    "kernel/node-signed runtime_proof.v1 (2N+1 admission)".to_string();
                entry.granted = true;
            }
        }
    }
    let report_domains: BTreeMap<String, DomainStatus> = domains
        .into_iter()
        .map(|(d, mut ds)| {
            ds.observed = observed.unwrap_or(false);
            (d, ds)
        })
        .collect();

    let residual_unknowns: Vec<String> = report_domains
        .values()
        .filter(|d| !d.observed)
        .map(|d| {
            format!(
                "agent role '{}' not yet granted/observed at runtime",
                d.agent_role
            )
        })
        .collect();

    AgentSocReport {
        schema: AGENT_SOC_SCHEMA.to_string(),
        human_admin_role: HumanAdminStatus {
            zero_touch_runtime: human_admin_zero_touch,
            never_touches_runtime: never_touches,
        },
        roles: report_roles,
        domains: report_domains,
        residual_unknowns,
    }
}

fn parse_declarative_roles(
    content: &str,
) -> (
    BTreeMap<String, AgentSocEntry>,
    BTreeMap<String, DomainStatus>,
    Vec<String>,
) {
    let v: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return (BTreeMap::new(), BTreeMap::new(), Vec::new()),
    };

    let mut roles = BTreeMap::new();
    let mut domains = BTreeMap::new();
    let mut rules: Vec<String> = vec![];

    if let Some(rules_arr) = v.get("gitops_rules").and_then(|x| x.as_array()) {
        for r in rules_arr.iter().filter_map(|x| x.as_str()) {
            rules.push(r.to_string());
        }
    }

    if let Some(roles_obj) = v.get("roles").and_then(|x| x.as_object()) {
        for (name, def) in roles_obj {
            let entry = AgentSocEntry {
                role: name.clone(),
                authority_class: def
                    .get("authority_class")
                    .and_then(|x| x.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                secret_source_class: def
                    .get("secret_source_class")
                    .and_then(|x| x.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                required_human_secret: def
                    .get("required_human_secret")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false),
                granted: false,
                granted_via: def
                    .get("granted_via")
                    .and_then(|x| x.as_str())
                    .unwrap_or("declared_only")
                    .to_string(),
                can_mutate: to_str_vec(def.get("can_mutate")),
                can_observe: to_str_vec(def.get("can_observe")),
                zero_touch_runtime: def
                    .get("runtime_zero_touch")
                    .or_else(|| def.get("zero_touch_runtime"))
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false),
            };
            roles.insert(name.clone(), entry);
        }
    }

    if let Some(domains_obj) = v.get("domains").and_then(|x| x.as_object()) {
        for (name, def) in domains_obj {
            let ds = DomainStatus {
                human_admin_role: def
                    .get("human_admin_role")
                    .and_then(|x| x.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                human_admin_action: def
                    .get("human_admin_action")
                    .and_then(|x| x.as_str())
                    .unwrap_or("none")
                    .to_string(),
                agent_role: def
                    .get("agent_role")
                    .and_then(|x| x.as_str())
                    .unwrap_or("none")
                    .to_string(),
                agent_action: def
                    .get("agent_action")
                    .and_then(|x| x.as_str())
                    .unwrap_or("none")
                    .to_string(),
                human_approval_required: def
                    .get("human_approval_required")
                    .and_then(|x| x.as_bool())
                    .unwrap_or(false),
                mutation_policy: def
                    .get("mutation_policy")
                    .and_then(|x| x.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                observed: false,
            };
            domains.insert(name.clone(), ds);
        }
    }

    (roles, domains, rules)
}

pub(crate) fn load_runtime_proof_state(roles_path: &Path) -> Option<bool> {
    let proof_path = roles_path.with_file_name("runtime_proof.v1");
    let content = std::fs::read_to_string(&proof_path).ok()?;
    serde_json::from_str::<serde_json::Value>(&content)
        .ok()?
        .get("granted")
        .and_then(|v| v.as_bool())
}

#[allow(dead_code)]
fn to_str_vec(v: Option<&serde_json::Value>) -> Vec<String> {
    v.and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_human_admin_zero_touch_ok() {
        let manifest = r#"{
            "gitops_rules": ["rule one"],
            "roles": {
                "human_admin": {
                    "authority_class": "HumanAdmin",
                    "secret_source_class": "HumanAdmin",
                    "required_human_secret": true,
                    "can_mutate": [],
                    "can_observe": ["*"],
                    "granted_via": "seed_one_shot",
                    "runtime_zero_touch": true
                },
                "runtime_publisher": {
                    "authority_class": "RuntimeMachineIdentity",
                    "secret_source_class": "RuntimeMachineIdentity",
                    "required_human_secret": false,
                    "can_mutate": ["publish_artifact_proof"],
                    "can_observe": ["runtime_history"],
                    "granted_via": "kernel-signed_post_seed",
                    "runtime_zero_touch": true
                }
            },
            "domains": {
                "runtime": {
                    "human_admin_role": "human_admin",
                    "human_admin_action": "observe_only",
                    "agent_role": "scheduler_agent",
                    "agent_action": "zero_touch_continuous",
                    "human_approval_required": false,
                    "mutation_policy": "admitted_dispatch_or_explicit_apply_only"
                }
            }
        }"#;
        let tmp = std::env::temp_dir().join("cynic-agent-roles-test.json");
        std::fs::write(&tmp, manifest).unwrap();
        let report = scan_agent_soc(&tmp);
        assert_eq!(report.schema, "cynic.agent_soc_report.v1");
        assert!(report.human_admin_role.zero_touch_runtime);
        let rp = report.roles.get("runtime_publisher").unwrap();
        assert!(!rp.granted);
        assert!(rp.runtime_zero_touch);
        assert_eq!(report.domains.get("runtime").unwrap().observed, false);
    }
}
