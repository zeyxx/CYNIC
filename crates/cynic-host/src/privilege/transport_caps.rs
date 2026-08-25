//! Declarative transport capabilities: typed scp/ssh/git compositions.
//!
//! Problem this owns (user law, 2026-08-25): agents kept hand-writing ad-hoc
//! scp/ssh one-liners with divergent paths — directory chaos and unauditable
//! mutations. Transport moves are RUNTIME CAPABILITIES: declared as data,
//! endpoints resolved from the fleet registry, rendered into exact argv,
//! denied by default. An emergence becomes a policy entry, reviewed in diff,
//! never a pasted shell line.
//!
//! Authority: `authority_class` mirrors the secret classes —
//! `runtime_machine_identity` capabilities run head-less;
//! `human_admin_bootstrap` ones require an explicit same-run consent marker
//! (env `CYNIC_HUMAN_ADMIN_CONSENT=<capability-id>`); anything else is
//! refused. Residual risk until lease fencing lands: consent is process-scoped,
//! not signed — recorded as an explicit unknown in reports.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub const TRANSPORT_CAPABILITIES_SCHEMA: &str = "cynic.host.transport_capabilities.v1";
pub const DEFAULT_TRANSPORT_POLICY_PATH: &str =
    "infra/privilege/transport-capabilities.policy.json";

/// Typed parameter kinds — validated structurally, no regex engine needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportParamKind {
    /// Git branch/ref name: no spaces, no `..`, no leading `-`.
    BranchName,
    /// One remote path segment: `[A-Za-z0-9._-]+`, no `/`, no `..`.
    PathSegment,
    /// sha256 hex digest.
    Sha256Hex,
    /// Local filesystem path (existence checked at execute time, not render).
    LocalPath,
}

impl TransportParamKind {
    pub fn validate(&self, value: &str) -> Result<(), TransportPolicyError> {
        let reject = |reason: &str| {
            Err(TransportPolicyError(format!(
                "param value {value:?} rejected as {self:?}: {reason}"
            )))
        };
        match self {
            Self::BranchName => {
                if value.is_empty()
                    || value.len() > 128
                    || value.starts_with('-')
                    || value.contains("..")
                    || value.contains(char::is_whitespace)
                    || !value
                        .chars()
                        .all(|c| c.is_alphanumeric() || matches!(c, '/' | '.' | '_' | '-'))
                {
                    return reject("not a safe branch/ref token");
                }
            }
            Self::PathSegment => {
                if value.is_empty()
                    || !value
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
                    || value.contains("..")
                {
                    return reject("not a safe path segment");
                }
            }
            Self::Sha256Hex => {
                if value.len() != 64 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
                    return reject("not a sha256 hex digest");
                }
            }
            Self::LocalPath => {
                if value.is_empty() || value.contains('\0') {
                    return reject("empty or NUL-bearing path");
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportCapabilityParam {
    pub name: String,
    pub kind: TransportParamKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityClass {
    /// Runs under the machine identity; no human in the loop.
    RuntimeMachineIdentity,
    /// HumanAdmin bootstrap gesture; needs explicit per-run consent.
    HumanAdminBootstrap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportCapability {
    pub id: String,
    /// Fleet-registry node whose `transports.ssh.target` resolves `{endpoint}`.
    pub node: String,
    pub authority_class: AuthorityClass,
    /// Exact argv skeleton; `{endpoint}` and `{param}` slots are substituted,
    /// anything else braced is a policy error.
    pub argv_template: Vec<String>,
    #[serde(default)]
    pub params: Vec<TransportCapabilityParam>,
    pub purpose: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportCapabilities {
    pub schema: String,
    #[serde(default)]
    pub default_endpoint_overrides: BTreeMap<String, String>,
    pub capabilities: Vec<TransportCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportPolicyError(pub String);

impl fmt::Display for TransportPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "transport capability error: {}", self.0)
    }
}

impl std::error::Error for TransportPolicyError {}

impl TransportCapabilities {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, TransportPolicyError> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path)
            .map_err(|error| TransportPolicyError(format!("read {}: {error}", path.display())))?;
        let policy: Self = serde_json::from_str(&raw)
            .map_err(|error| TransportPolicyError(format!("parse {}: {error}", path.display())))?;
        policy.validate()?;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), TransportPolicyError> {
        if self.schema != TRANSPORT_CAPABILITIES_SCHEMA {
            return Err(TransportPolicyError(format!(
                "schema mismatch: expected {TRANSPORT_CAPABILITIES_SCHEMA}, got {}",
                self.schema
            )));
        }
        let mut seen = std::collections::BTreeSet::new();
        for capability in &self.capabilities {
            if !valid_id(&capability.id) {
                return Err(TransportPolicyError(format!(
                    "capability id {:?} must match ^[a-z0-9-]{{1,64}}$",
                    capability.id
                )));
            }
            if !seen.insert(capability.id.clone()) {
                return Err(TransportPolicyError(format!(
                    "duplicate capability id {}",
                    capability.id
                )));
            }
            let mut param_names = std::collections::BTreeSet::new();
            for param in &capability.params {
                if !valid_id(&param.name) {
                    return Err(TransportPolicyError(format!(
                        "param name {:?} must match ^[a-z0-9-]{{1,64}}$",
                        param.name
                    )));
                }
                if !param_names.insert(param.name.clone()) {
                    return Err(TransportPolicyError(format!(
                        "duplicate param {} in capability {}",
                        param.name, capability.id
                    )));
                }
            }
            // Every template slot must be `{endpoint}` or a declared param;
            // undeclared braces are policy errors, not runtime surprises.
            for part in &capability.argv_template {
                for slot in brace_slots(part) {
                    let ok = slot == "endpoint"
                        || capability.params.iter().any(|param| param.name == slot);
                    if !ok {
                        return Err(TransportPolicyError(format!(
                            "capability {} references undeclared slot {{{slot}}}",
                            capability.id
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn capability(&self, id: &str) -> Result<&TransportCapability, TransportPolicyError> {
        self.capabilities
            .iter()
            .find(|capability| capability.id == id)
            .ok_or_else(|| {
                TransportPolicyError(format!(
                    "unknown capability {id:?} (declared: {:?})",
                    self.capabilities
                        .iter()
                        .map(|capability| capability.id.as_str())
                        .collect::<Vec<_>>()
                ))
            })
    }

    /// Resolve `{endpoint}` for a capability: explicit override wins, then the
    /// caller-supplied registry view, else error (never guessed).
    pub fn resolve_endpoint(
        &self,
        capability: &TransportCapability,
        registry_view: &BTreeMap<String, String>,
    ) -> Result<String, TransportPolicyError> {
        if let Some(endpoint) = self.default_endpoint_overrides.get(&capability.node) {
            return Ok(endpoint.clone());
        }
        registry_view.get(&capability.node).cloned().ok_or_else(|| {
            TransportPolicyError(format!(
                "no ssh endpoint resolved for node {:?} (registry view lacks transports.ssh.target)",
                capability.node
            ))
        })
    }

    /// Render the exact argv. Denies by construction: missing params, extra
    /// params, unsafe values, and undeclared slots are hard errors.
    pub fn render(
        &self,
        capability: &TransportCapability,
        params: &BTreeMap<String, String>,
        endpoint: &str,
    ) -> Result<Vec<String>, TransportPolicyError> {
        if endpoint.is_empty() || endpoint.contains(char::is_whitespace) {
            return Err(TransportPolicyError("invalid endpoint".into()));
        }
        let mut rendered = Vec::with_capacity(capability.argv_template.len());
        for part in &capability.argv_template {
            let mut out = part.clone();
            out = out.replace("{endpoint}", endpoint);
            for param in &capability.params {
                let slot = format!("{{{}}}", param.name);
                if out.contains(&slot) {
                    let value = params.get(&param.name).ok_or_else(|| {
                        TransportPolicyError(format!(
                            "missing param {} for capability {}",
                            param.name, capability.id
                        ))
                    })?;
                    param.kind.validate(value)?;
                    out = out.replace(&slot, value);
                }
            }
            if let Some(leftover) = brace_slots(&out).first() {
                return Err(TransportPolicyError(format!(
                    "unresolved slot {{{leftover}}} in capability {}",
                    capability.id
                )));
            }
            rendered.push(out);
        }
        for name in params.keys() {
            if !capability.params.iter().any(|param| &param.name == name) {
                return Err(TransportPolicyError(format!(
                    "undeclared param {name} supplied to capability {}",
                    capability.id
                )));
            }
        }
        Ok(rendered)
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Extract `{{slot}}` tokens from a template part.
fn brace_slots(part: &str) -> Vec<String> {
    let mut slots = Vec::new();
    let bytes = part.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'{' {
            if let Some(close) = part[index + 1..].find('}') {
                slots.push(part[index + 1..index + 1 + close].to_string());
                index += close + 2;
                continue;
            }
        }
        index += 1;
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> TransportCapabilities {
        serde_json::from_str(
            r#"{
            "schema": "cynic.host.transport_capabilities.v1",
            "default_endpoint_overrides": {},
            "capabilities": [
                {
                    "id": "builder-push-branch",
                    "node": "cynic-builder",
                    "authority_class": "runtime_machine_identity",
                    "argv_template": ["git", "push",
                        "ssh://cynic@10.10.10.119/var/lib/cynic/source/repositories/CYNIC/.git",
                        "{branch}"],
                    "params": [{"name": "branch", "kind": "branch_name"}],
                    "purpose": "publish an agent branch to the builder canonical repo"
                },
                {
                    "id": "builder-stage-file",
                    "node": "cynic-builder",
                    "authority_class": "runtime_machine_identity",
                    "argv_template": ["scp", "{source}", "{endpoint}:/tmp/{remote_name}"],
                    "params": [
                        {"name": "source", "kind": "local_path"},
                        {"name": "remote_name", "kind": "path_segment"}
                    ],
                    "purpose": "stage an artifact into the builder staging dir"
                },
                {
                    "id": "pve-install-bundle",
                    "node": "cynic-host-proxmox",
                    "authority_class": "human_admin_bootstrap",
                    "argv_template": ["ssh", "-t", "{endpoint}",
                        "sudo install -m {mode} /tmp/{artifact} {target}"],
                    "params": [
                        {"name": "mode", "kind": "path_segment"},
                        {"name": "artifact", "kind": "path_segment"},
                        {"name": "target", "kind": "path_segment"}
                    ],
                    "purpose": "one-shot HumanAdmin install of projected helper artifacts"
                }
            ]
        }"#,
        )
        .expect("sample parses")
    }

    fn view() -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                "cynic-builder".to_string(),
                "cynic-builder-build".to_string(),
            ),
            (
                "cynic-host-proxmox".to_string(),
                "proxmox-forge".to_string(),
            ),
        ])
    }

    #[test]
    fn validates_and_renders_exact_argv() {
        let policy = sample();
        policy.validate().expect("valid");
        let capability = policy.capability("builder-stage-file").expect("exists");
        let endpoint = policy
            .resolve_endpoint(capability, &view())
            .expect("resolved");
        let argv = policy
            .render(
                capability,
                &BTreeMap::from([
                    (
                        "source".to_string(),
                        "/tmp/pve-bundle/pct-bounded-exec".to_string(),
                    ),
                    ("remote_name".to_string(), "pct-bounded-exec".to_string()),
                ]),
                &endpoint,
            )
            .expect("renders");
        assert_eq!(
            argv,
            vec![
                "scp",
                "/tmp/pve-bundle/pct-bounded-exec",
                "cynic-builder-build:/tmp/pct-bounded-exec"
            ]
        );
    }

    #[test]
    fn branch_kind_rejects_unsafe_values() {
        let policy = sample();
        let capability = policy.capability("builder-push-branch").expect("exists");
        let endpoint = policy
            .resolve_endpoint(capability, &view())
            .expect("resolved");
        for bad in ["main; rm -rf /", "--upload-pack=x", "a..b", "has space"] {
            assert!(
                policy
                    .render(
                        capability,
                        &BTreeMap::from([("branch".to_string(), bad.to_string())]),
                        &endpoint
                    )
                    .is_err(),
                "should reject {bad}"
            );
        }
        policy
            .render(
                capability,
                &BTreeMap::from([(
                    "branch".to_string(),
                    "agent/hermes/pattern-gc-l1".to_string(),
                )]),
                &endpoint,
            )
            .expect("good branch renders");
    }

    #[test]
    fn denies_missing_and_undeclared_params_and_unknown_capability() {
        let policy = sample();
        let capability = policy.capability("builder-stage-file").expect("exists");
        let endpoint = policy.resolve_endpoint(capability, &view()).unwrap();
        assert!(
            policy
                .render(capability, &BTreeMap::new(), &endpoint)
                .is_err()
        );
        let mut extra = BTreeMap::from([
            ("source".to_string(), "x".to_string()),
            ("remote_name".to_string(), "y".to_string()),
            ("sneaky".to_string(), "z".to_string()),
        ]);
        extra.insert("evil".to_string(), "-oProxyCommand=evil".to_string());
        assert!(policy.render(capability, &extra, &endpoint).is_err());
        assert!(policy.capability("nope").is_err());
    }

    #[test]
    fn unresolved_slots_are_policy_errors_not_runtime_surprises() {
        let mut policy = sample();
        policy.capabilities[0].argv_template[3] = "{nope}".into();
        assert!(policy.validate().is_err());
    }

    #[test]
    fn human_admin_class_is_flagged_for_consent_gating() {
        let policy = sample();
        let capability = policy.capability("pve-install-bundle").expect("exists");
        assert_eq!(
            capability.authority_class,
            AuthorityClass::HumanAdminBootstrap
        );
    }
}
