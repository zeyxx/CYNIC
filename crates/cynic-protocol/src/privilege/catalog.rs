//! Privileged capability catalog — pure types and admission.
//!
//! One catalog of capabilities, one admission function, shared by every
//! domain. A capability names an exact argv template and the parameters it
//! accepts; admission resolves a capability id plus validated parameters into
//! the one argv that may run. Nothing here touches the filesystem or a
//! process: the installed-catalog read and the exec live in adapters.
//!
//! Design gate answers (bottom-up):
//! - Desired state: a privileged action runs only if its capability is
//!   declared in the catalog AND installed on this node AND every parameter
//!   validates. Anything else is a named blocker.
//! - Pure selection rule: a capability is admitted when the id exists, it is
//!   installed, the supplied parameters are exactly the declared ones, and
//!   each value passes its kind's validator. No defaults, no partial fills.
//! - Stop condition: the first unmet condition blocks; blockers accumulate
//!   but never downgrade to a warning.
//!
//! Why this module exists: three implementations of the same `pct` argv grew
//! grew independently (`cynic-lxc::admit`, a shell script,
//! `proxmox_clone.rs` in Rust). Three implementations of one policy diverge
//! silently. This is the single one.

use serde::{Deserialize, Serialize};

pub const CAPABILITY_CATALOG_SCHEMA: &str = "cynic.capability_catalog.v1";
pub const INSTALLED_CATALOG_SCHEMA: &str = "cynic.capability_catalog.installed.v1";

/// Longest accepted parameter value. A bound, not a suggestion: an unbounded
/// parameter is an unbounded argv.
const MAX_PARAM_LEN: usize = 4096;

/// How a capability parameter is validated before it reaches an argv.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamKind {
    /// A git branch name: ref-safe, no traversal, no option lookalike.
    BranchName,
    /// An absolute path on the calling host.
    LocalPath,
    /// A single path segment: no separator, no traversal.
    PathSegment,
    /// A container/VM identifier.
    Vmid,
    /// An absolute path that must already exist on the target.
    AbsolutePath,
    /// A disk size delta, e.g. `+40G`.
    SizeDelta,
    /// A sha256 hex digest (64 lowercase hex chars).
    Sha256Hex,
    /// A transport endpoint (hostname or host:port): no whitespace, no shell
    /// metacharacter. Resolved by the adapter from the registry, never
    /// hardcoded in a capability.
    Endpoint,
}

impl ParamKind {
    /// The sudoers character class that bounds this parameter's shape.
    ///
    /// WHY. sudoers CAN express bounded wildcards — measured on the builder,
    /// all parsed OK:
    ///   /usr/sbin/pct destroy [0-9]*              → parsed OK
    ///   /usr/sbin/pct resize [0-9]* rootfs [+][0-9]*[KMGT]  → parsed OK
    ///   /usr/sbin/pct clone [0-9]* [0-9]* --full → parsed OK
    ///
    /// WHAT A CLASS ACTUALLY BOUNDS — measured, and narrower than it looks.
    /// `[0-9]*` anchors the FIRST character; the tail is free. On the live
    /// sudoers, with `--param vmid=[0-9]*`:
    ///
    ///   ACCEPTED: 106, `106; id`, `1/../../etc/shadow`, `106 --dangerous`
    ///   REFUSED:  /etc/shadow, ../../etc/shadow, --privileged, abc
    ///
    /// So the membrane is a COARSE first filter, not the enforcement point. It
    /// stops a value that cannot begin with a digit at all; it does not stop a
    /// metacharacter appended after one. `admit_capability()` is the second
    /// layer and DOES refuse those — verified on the same four inputs.
    ///
    /// Kinds whose shape sudoers cannot express (paths with `/`) return `None`;
    /// the caller falls back to `*`. Documented, not hidden.
    pub fn sudoers_class(&self) -> Option<&'static str> {
        match self {
            ParamKind::Vmid => Some("[0-9]*"),
            ParamKind::SizeDelta => Some("[+][0-9]*[KMGT]"),
            ParamKind::BranchName => Some("[a-zA-Z0-9._/-]*"),
            ParamKind::PathSegment => Some("[a-zA-Z0-9._-]*"),
            ParamKind::Sha256Hex => Some("[0-9a-f]*"),
            ParamKind::Endpoint => Some("[a-zA-Z0-9._-]*"),
            // sudoers refuses `/` in arguments, so paths cannot be bounded.
            ParamKind::LocalPath | ParamKind::AbsolutePath => None,
        }
    }

    /// Validate one value for this kind. Empty result means accepted.
    pub fn validate(&self, value: &str) -> Vec<String> {
        let mut blockers = Vec::new();
        if value.is_empty() {
            blockers.push("param_empty".to_string());
            return blockers;
        }
        if value.len() > MAX_PARAM_LEN {
            blockers.push("param_too_long".to_string());
            return blockers;
        }
        // An option lookalike is refused for every kind: a value that starts
        // with `-` can be read as a flag by the executed program.
        if value.starts_with('-') {
            blockers.push("param_option_lookalike".to_string());
        }
        if value.contains('\0') || value.contains('\n') || value.contains('\r') {
            blockers.push("param_control_character".to_string());
        }
        match self {
            ParamKind::BranchName => {
                if value.contains("..") {
                    blockers.push("branch_traversal".to_string());
                }
                if value.contains(char::is_whitespace) {
                    blockers.push("branch_whitespace".to_string());
                }
                // ALLOWLIST, not denylist. A denylist of git-unsafe characters
                // (`~ ^ : ? * [ \`) is incomplete: it accepts `@ # + = %` and
                // every future metacharacter. Measured divergence against
                // cynic-host transport_caps, which already used an allowlist:
                // the catalogue accepted `a@b` where transport refused it.
                // One concept, one validator — this is the stricter of the two.
                if !value
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '/' | '.' | '_' | '-'))
                {
                    blockers.push("branch_unsafe_character".to_string());
                }
                if value.len() > 128 {
                    blockers.push("branch_too_long".to_string());
                }
                if value.starts_with('/') || value.ends_with('/') || value.ends_with(".lock") {
                    blockers.push("branch_invalid_shape".to_string());
                }
            }
            ParamKind::LocalPath | ParamKind::AbsolutePath => {
                if !value.starts_with('/') {
                    blockers.push("path_not_absolute".to_string());
                }
                if value.split('/').any(|segment| segment == "..") {
                    blockers.push("path_traversal".to_string());
                }
                if value.contains(['*', '?', '`', '$', '|', '&', ';', '<', '>', '"', '\'', '\\']) {
                    blockers.push("path_shell_metacharacter".to_string());
                }
            }
            ParamKind::PathSegment => {
                if value.contains('/') {
                    blockers.push("segment_contains_separator".to_string());
                }
                if value == "." || value == ".." {
                    blockers.push("segment_traversal".to_string());
                }
                if value.contains(char::is_whitespace) {
                    blockers.push("segment_whitespace".to_string());
                }
                if value.contains(['*', '?', '`', '$', '|', '&', ';', '<', '>', '"', '\'', '\\']) {
                    blockers.push("segment_shell_metacharacter".to_string());
                }
            }
            ParamKind::Vmid => match value.parse::<u32>() {
                Ok(vmid) if (100..=999_999_999).contains(&vmid) => {}
                Ok(_) => blockers.push("vmid_out_of_range".to_string()),
                Err(_) => blockers.push("vmid_not_an_integer".to_string()),
            },
            ParamKind::SizeDelta => {
                let Some(rest) = value.strip_prefix('+') else {
                    blockers.push("size_delta_missing_sign".to_string());
                    return blockers;
                };
                let (digits, unit) = rest.split_at(rest.len().saturating_sub(1));
                if !matches!(unit, "K" | "M" | "G" | "T") {
                    blockers.push("size_delta_unit_invalid".to_string());
                }
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    blockers.push("size_delta_magnitude_invalid".to_string());
                }
            }
            ParamKind::Sha256Hex => {
                if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
                    blockers.push("sha256_not_64_hex".to_string());
                }
            }
            ParamKind::Endpoint => {
                if value.contains(char::is_whitespace) {
                    blockers.push("endpoint_whitespace".to_string());
                }
                if value.contains(['/', '\\', '`', '$', '|', '&', ';', '<', '>', '"', '\'', '{', '}']) {
                    blockers.push("endpoint_metacharacter".to_string());
                }
            }
        }
        blockers
    }
}

/// A declared parameter of a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityParam {
    pub name: String,
    pub kind: ParamKind,
}

/// One capability: an exact argv template plus its declared parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub domain: String,
    /// argv with `{param}` placeholders. Never a shell string.
    pub argv_template: Vec<String>,
    #[serde(default)]
    pub params: Vec<CapabilityParam>,
    #[serde(default)]
    pub requires_root: bool,
    /// Mandatory capabilities are installed on every node and cannot be
    /// deselected. An empty catalog of mandatory capabilities is a valid
    /// install; a node missing one is a blocked install.
    #[serde(default)]
    pub mandatory: bool,
    pub purpose: String,
}

/// Escape the characters sudoers treats as separators.
///
/// MEASURED: `cynic ALL=(root) NOPASSWD: ... --scopes write:repository` is
/// rejected by `visudo -c` with `syntax error` because `:` separates the
/// user-list from the command-list. `write\:repository` parses OK.
///
/// Every literal that reaches a sudoers rule goes through this: a rule that
/// fails to parse is an unparseable sudoers file, which can lock out `sudo`.
fn escape_sudoers(value: &str) -> String {
    value.replace(':', "\\:")
}

/// The full catalog, authored as data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityCatalog {
    pub schema: String,
    pub capabilities: Vec<Capability>,
}

impl CapabilityCatalog {
    /// Project the catalog into sudoers rules for the given subject.
    ///
    /// WHY THE RULE NAMES THE BINARY, NOT THE RESOLVED ARGV. sudoers cannot
    /// express the admission policy. Measured: `visudo -c` rejects a rule
    /// containing `--scopes write:repository` because `:` is a sudoers
    /// separator, and it cannot validate that a vmid is an integer or that a
    /// value is not an option lookalike. Those checks live in
    /// `admit_capability`, which is the enforcement point.
    ///
    /// So the membrane is deliberately coarse — the binary plus the capability
    /// id — and the binary performs the fine admission. Defense in depth:
    /// sudoers bounds WHICH capability may run as root, the binary bounds
    /// WHICH parameters that capability may carry. A `*` tail is required
    /// because sudoers cannot enumerate typed parameters, and sudoers refuses
    /// any argument containing `/`, which is strictly more restrictive than
    /// the binary.
    ///
    /// `{param}` placeholders never reach the rule: only the capability id
    /// does. Deterministic (sorted by id).
    pub fn sudoers_rules(&self, subject: &str, binary_path: &str) -> Vec<String> {
        let mut rules = Vec::new();
        let mut caps: Vec<&Capability> = self
            .capabilities
            .iter()
            .filter(|c| c.requires_root)
            .collect();
        caps.sort_by(|a, b| a.id.cmp(&b.id));
        for cap in caps {
            // The rule MUST match the argv the binary actually receives:
            //   <binary> privilege exec <cap-id> [--param name=value]...
            //
            // NOT the resolved argv (`/usr/sbin/pct clone 105 106 --full`):
            // the resolved argv is never passed on the command line, it is
            // built inside the binary from the catalog. A rule naming it would
            // never match any real invocation — the membrane would be dead
            // while looking strict.
            let mut parts = vec![format!(
                "{} privilege exec {}",
                escape_sudoers(binary_path),
                escape_sudoers(&cap.id)
            )];
            for param in &cap.params {
                let class = param.kind.sudoers_class().unwrap_or("*");
                parts.push(format!(
                    "--param {}={}",
                    escape_sudoers(&param.name),
                    class
                ));
            }
            rules.push(format!("{subject} ALL=(root) NOPASSWD: {}", parts.join(" ")));
        }
        rules
    }

    /// All sudoers rules as one file body, newline-terminated.
    pub fn sudoers_file(&self, subject: &str, binary_path: &str) -> String {
        let rules = self.sudoers_rules(subject, binary_path);
        format!("{}\n", rules.join("\n"))
    }
}

/// The set of capabilities installed on this node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledCapabilities {
    pub schema: String,
    pub capability_ids: Vec<String>,
}

impl InstalledCapabilities {
    pub fn contains(&self, id: &str) -> bool {
        self.capability_ids.iter().any(|candidate| candidate == id)
    }
}

/// The outcome of admitting a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum CapabilityDecision {
    /// The exact argv that may run, and whether it needs root.
    Admitted {
        capability_id: String,
        argv: Vec<String>,
        requires_root: bool,
        domain: String,
    },
    Blocked { blockers: Vec<String> },
}

impl CapabilityCatalog {
    /// Find a capability by id.
    pub fn find(&self, id: &str) -> Option<&Capability> {
        self.capabilities.iter().find(|c| c.id == id)
    }

    /// Ids that must be installed on every node.
    pub fn mandatory_ids(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .filter(|c| c.mandatory)
            .map(|c| c.id.clone())
            .collect()
    }

    /// Structural validation of the catalog itself. A catalog that fails this
    /// cannot admit anything: an invalid catalog is not a permissive one.
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != CAPABILITY_CATALOG_SCHEMA {
            blockers.push("catalog_schema_mismatch".to_string());
        }
        if self.capabilities.is_empty() {
            blockers.push("catalog_empty".to_string());
        }
        let mut seen: Vec<&str> = Vec::new();
        for capability in &self.capabilities {
            if capability.id.trim().is_empty() {
                blockers.push("capability_id_empty".to_string());
                continue;
            }
            if seen.contains(&capability.id.as_str()) {
                blockers.push(format!("capability_id_duplicate:{}", capability.id));
            }
            seen.push(&capability.id);
            if capability.domain.trim().is_empty() {
                blockers.push(format!("capability_domain_empty:{}", capability.id));
            }
            if capability.purpose.trim().is_empty() {
                blockers.push(format!("capability_purpose_empty:{}", capability.id));
            }
            if capability.argv_template.is_empty() {
                blockers.push(format!("capability_argv_empty:{}", capability.id));
                continue;
            }
            // Every placeholder in the template must have a declared param,
            // and every declared param must appear in the template. A mismatch
            // is how a template silently keeps a literal `{param}`.
            let placeholders = placeholders_of(&capability.argv_template);
            for placeholder in &placeholders {
                if !capability.params.iter().any(|p| &p.name == placeholder) {
                    blockers.push(format!(
                        "capability_placeholder_undeclared:{}:{}",
                        capability.id, placeholder
                    ));
                }
            }
            for param in &capability.params {
                if param.name.trim().is_empty() {
                    blockers.push(format!("capability_param_name_empty:{}", capability.id));
                }
                if !placeholders.contains(&param.name) {
                    blockers.push(format!(
                        "capability_param_unused:{}:{}",
                        capability.id, param.name
                    ));
                }
            }
        }
        blockers
    }
}

/// Admit a capability: resolve `id` + validated parameters into the one argv
/// that may run.
///
/// Fail-closed at every step. A capability absent from the catalog, absent
/// from the installed set, missing a parameter, carrying an undeclared
/// parameter, or holding a value that fails its kind is a `Blocked` decision
/// with a named blocker — never a partial argv.
pub fn admit_capability(
    catalog: &CapabilityCatalog,
    installed: &InstalledCapabilities,
    id: &str,
    parameters: &[(String, String)],
) -> CapabilityDecision {
    let mut blockers = Vec::new();

    if catalog.schema != CAPABILITY_CATALOG_SCHEMA {
        blockers.push("catalog_schema_mismatch".to_string());
    }
    if installed.schema != INSTALLED_CATALOG_SCHEMA {
        blockers.push("installed_catalog_schema_mismatch".to_string());
    }
    if !blockers.is_empty() {
        return CapabilityDecision::Blocked { blockers };
    }

    let Some(capability) = catalog.find(id) else {
        return CapabilityDecision::Blocked {
            blockers: vec![format!("capability_not_in_catalog:{id}")],
        };
    };

    if !installed.contains(id) {
        blockers.push(format!("capability_not_installed:{id}"));
    }

    // The supplied parameter set must equal the declared one: no missing, no
    // extra. An extra parameter has nowhere to go and is refused rather than
    // dropped.
    for declared in &capability.params {
        if !parameters.iter().any(|(name, _)| name == &declared.name) {
            blockers.push(format!("capability_param_missing:{id}:{}", declared.name));
        }
    }
    for (name, _) in parameters {
        if !capability.params.iter().any(|p| &p.name == name) {
            blockers.push(format!("capability_param_undeclared:{id}:{name}"));
        }
    }

    // Validate each declared value.
    for declared in &capability.params {
        if let Some((_, value)) = parameters.iter().find(|(name, _)| name == &declared.name) {
            for blocker in declared.kind.validate(value) {
                blockers.push(format!("capability_param_invalid:{id}:{}:{blocker}", declared.name));
            }
        }
    }

    if !blockers.is_empty() {
        return CapabilityDecision::Blocked { blockers };
    }

    // Render the argv. Every placeholder is substituted; a leftover `{` is a
    // bug in the template and must not reach a process.
    let mut argv: Vec<String> = Vec::with_capacity(capability.argv_template.len());
    for part in &capability.argv_template {
        let mut rendered = part.clone();
        for declared in &capability.params {
            if let Some((_, value)) = parameters.iter().find(|(name, _)| name == &declared.name) {
                rendered = rendered.replace(&format!("{{{}}}", declared.name), value);
            }
        }
        if rendered.contains('{') || rendered.contains('}') {
            return CapabilityDecision::Blocked {
                blockers: vec![format!("capability_template_unresolved:{id}")],
            };
        }
        argv.push(rendered);
    }

    CapabilityDecision::Admitted {
        capability_id: capability.id.clone(),
        argv,
        requires_root: capability.requires_root,
        domain: capability.domain.clone(),
    }
}

/// Installed capabilities must include every mandatory one. A node missing a
/// mandatory capability is a blocked install, not a degraded node.
pub fn validate_install(catalog: &CapabilityCatalog, installed: &InstalledCapabilities) -> Vec<String> {
    let mut blockers = Vec::new();
    if installed.schema != INSTALLED_CATALOG_SCHEMA {
        blockers.push("installed_catalog_schema_mismatch".to_string());
    }
    for id in catalog.mandatory_ids() {
        if !installed.contains(&id) {
            blockers.push(format!("mandatory_capability_missing:{id}"));
        }
    }
    for id in &installed.capability_ids {
        if catalog.find(id).is_none() {
            blockers.push(format!("installed_capability_not_in_catalog:{id}"));
        }
    }
    blockers
}

/// Distinct `{name}` placeholders across a template.
fn placeholders_of(template: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for part in template {
        let mut rest = part.as_str();
        while let Some(start) = rest.find('{') {
            let after = &rest[start + 1..];
            let Some(end) = after.find('}') else { break };
            let name = &after[..end];
            if !name.is_empty() && !found.iter().any(|f| f == name) {
                found.push(name.to_string());
            }
            rest = &after[end + 1..];
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> CapabilityCatalog {
        CapabilityCatalog {
            schema: CAPABILITY_CATALOG_SCHEMA.to_string(),
            capabilities: vec![
                Capability {
                    id: "pct-list".to_string(),
                    domain: "hypervisor".to_string(),
                    argv_template: vec!["/usr/sbin/pct".to_string(), "list".to_string()],
                    params: vec![],
                    requires_root: true,
                    mandatory: false,
                    purpose: "CT inventory (read-only)".to_string(),
                },
                Capability {
                    id: "pct-clone".to_string(),
                    domain: "hypervisor".to_string(),
                    argv_template: vec![
                        "/usr/sbin/pct".to_string(),
                        "clone".to_string(),
                        "{source}".to_string(),
                        "{target}".to_string(),
                        "--full".to_string(),
                    ],
                    params: vec![
                        CapabilityParam {
                            name: "source".to_string(),
                            kind: ParamKind::Vmid,
                        },
                        CapabilityParam {
                            name: "target".to_string(),
                            kind: ParamKind::Vmid,
                        },
                    ],
                    requires_root: true,
                    mandatory: false,
                    purpose: "clone a container".to_string(),
                },
                Capability {
                    id: "node-control".to_string(),
                    domain: "runtime".to_string(),
                    argv_template: vec!["node".to_string(), "control".to_string()],
                    params: vec![],
                    requires_root: false,
                    mandatory: true,
                    purpose: "supervised node runtime".to_string(),
                },
            ],
        }
    }

    fn installed(ids: &[&str]) -> InstalledCapabilities {
        InstalledCapabilities {
            schema: INSTALLED_CATALOG_SCHEMA.to_string(),
            capability_ids: ids.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn catalog_is_structurally_valid() {
        assert!(catalog().validate().is_empty());
    }

    #[test]
    fn placeholder_without_param_is_rejected() {
        let mut c = catalog();
        c.capabilities[1].params.pop();
        assert!(c
            .validate()
            .iter()
            .any(|b| b.starts_with("capability_placeholder_undeclared:pct-clone")));
    }

    #[test]
    fn param_without_placeholder_is_rejected() {
        let mut c = catalog();
        c.capabilities[1].argv_template = vec!["/usr/sbin/pct".to_string(), "clone".to_string()];
        assert!(c
            .validate()
            .iter()
            .any(|b| b.starts_with("capability_param_unused:pct-clone")));
    }

    #[test]
    fn duplicate_capability_id_is_rejected() {
        let mut c = catalog();
        c.capabilities.push(c.capabilities[0].clone());
        assert!(c
            .validate()
            .contains(&"capability_id_duplicate:pct-list".to_string()));
    }

    #[test]
    fn admits_rendered_argv() {
        let decision = admit_capability(
            &catalog(),
            &installed(&["node-control", "pct-clone"]),
            "pct-clone",
            &[
                ("source".to_string(), "105".to_string()),
                ("target".to_string(), "106".to_string()),
            ],
        );
        match decision {
            CapabilityDecision::Admitted { argv, requires_root, .. } => {
                assert_eq!(argv, vec!["/usr/sbin/pct", "clone", "105", "106", "--full"]);
                assert!(requires_root);
            }
            other => panic!("expected admitted, got {other:?}"),
        }
    }

    #[test]
    fn uninstalled_capability_is_blocked() {
        let decision = admit_capability(&catalog(), &installed(&["node-control"]), "pct-list", &[]);
        assert!(matches!(
            decision,
            CapabilityDecision::Blocked { blockers }
                if blockers.contains(&"capability_not_installed:pct-list".to_string())
        ));
    }

    #[test]
    fn unknown_capability_is_blocked() {
        let decision = admit_capability(&catalog(), &installed(&["node-control"]), "rm-rf", &[]);
        assert!(matches!(
            decision,
            CapabilityDecision::Blocked { blockers }
                if blockers.contains(&"capability_not_in_catalog:rm-rf".to_string())
        ));
    }

    #[test]
    fn missing_param_is_blocked() {
        let decision = admit_capability(
            &catalog(),
            &installed(&["pct-clone"]),
            "pct-clone",
            &[("source".to_string(), "105".to_string())],
        );
        assert!(matches!(
            decision,
            CapabilityDecision::Blocked { blockers }
                if blockers.contains(&"capability_param_missing:pct-clone:target".to_string())
        ));
    }

    #[test]
    fn undeclared_param_is_blocked() {
        let decision = admit_capability(
            &catalog(),
            &installed(&["pct-clone"]),
            "pct-clone",
            &[
                ("source".to_string(), "105".to_string()),
                ("target".to_string(), "106".to_string()),
                ("inject".to_string(), "--privileged".to_string()),
            ],
        );
        assert!(matches!(
            decision,
            CapabilityDecision::Blocked { blockers }
                if blockers.contains(&"capability_param_undeclared:pct-clone:inject".to_string())
        ));
    }

    #[test]
    fn vmid_injection_is_blocked() {
        let decision = admit_capability(
            &catalog(),
            &installed(&["pct-clone"]),
            "pct-clone",
            &[
                ("source".to_string(), "105".to_string()),
                ("target".to_string(), "106 --privileged".to_string()),
            ],
        );
        assert!(matches!(decision, CapabilityDecision::Blocked { .. }));
    }

    #[test]
    fn option_lookalike_param_is_blocked() {
        let decision = admit_capability(
            &catalog(),
            &installed(&["pct-clone"]),
            "pct-clone",
            &[
                ("source".to_string(), "-1".to_string()),
                ("target".to_string(), "106".to_string()),
            ],
        );
        assert!(matches!(
            decision,
            CapabilityDecision::Blocked { blockers }
                if blockers.iter().any(|b| b.contains("param_option_lookalike"))
        ));
    }

    #[test]
    fn path_traversal_is_blocked() {
        assert!(ParamKind::LocalPath
            .validate("/tmp/../../etc/shadow")
            .contains(&"path_traversal".to_string()));
    }

    #[test]
    fn branch_traversal_is_blocked() {
        assert!(ParamKind::BranchName
            .validate("main..evil")
            .contains(&"branch_traversal".to_string()));
    }

    #[test]
    fn size_delta_validates() {
        assert!(ParamKind::SizeDelta.validate("+40G").is_empty());
        assert!(ParamKind::SizeDelta
            .validate("40G")
            .contains(&"size_delta_missing_sign".to_string()));
        assert!(ParamKind::SizeDelta
            .validate("+40X")
            .contains(&"size_delta_unit_invalid".to_string()));
    }

    #[test]
    fn install_missing_mandatory_is_blocked() {
        let blockers = validate_install(&catalog(), &installed(&["pct-list"]));
        assert!(blockers.contains(&"mandatory_capability_missing:node-control".to_string()));
    }

    #[test]
    fn install_with_unknown_id_is_blocked() {
        let blockers = validate_install(&catalog(), &installed(&["node-control", "ghost"]));
        assert!(blockers.contains(&"installed_capability_not_in_catalog:ghost".to_string()));
    }

    #[test]
    fn install_with_all_mandatory_is_clean() {
        assert!(validate_install(&catalog(), &installed(&["node-control", "pct-list"])).is_empty());
    }
}

#[cfg(test)]
mod absorb_transport_tests {
    use super::*;

    fn transport_catalog() -> CapabilityCatalog {
        CapabilityCatalog {
            schema: CAPABILITY_CATALOG_SCHEMA.to_string(),
            capabilities: vec![Capability {
                id: "builder-push-branch".to_string(),
                domain: "transport".to_string(),
                argv_template: vec![
                    "git".to_string(),
                    "push".to_string(),
                    "{endpoint}:/var/lib/cynic/source/repositories/CYNIC/.git".to_string(),
                    "{branch}".to_string(),
                ],
                params: vec![
                    CapabilityParam {
                        name: "endpoint".to_string(),
                        kind: ParamKind::Endpoint,
                    },
                    CapabilityParam {
                        name: "branch".to_string(),
                        kind: ParamKind::BranchName,
                    },
                ],
                requires_root: false,
                mandatory: false,
                purpose: "publish an agent branch".to_string(),
            }],
        }
    }

    #[test]
    fn absorbs_transport_capability_render() {
        let decision = admit_capability(
            &transport_catalog(),
            &InstalledCapabilities {
                schema: INSTALLED_CATALOG_SCHEMA.to_string(),
                capability_ids: vec!["builder-push-branch".to_string()],
            },
            "builder-push-branch",
            &[
                ("endpoint".to_string(), "cynic-builder-build".to_string()),
                ("branch".to_string(), "agent/hermes/knowledge-canon".to_string()),
            ],
        );
        match decision {
            CapabilityDecision::Admitted { argv, .. } => assert_eq!(
                argv,
                vec![
                    "git",
                    "push",
                    "cynic-builder-build:/var/lib/cynic/source/repositories/CYNIC/.git",
                    "agent/hermes/knowledge-canon"
                ]
            ),
            other => panic!("expected admitted, got {other:?}"),
        }
    }

    #[test]
    fn endpoint_with_metacharacter_is_blocked() {
        let decision = admit_capability(
            &transport_catalog(),
            &InstalledCapabilities {
                schema: INSTALLED_CATALOG_SCHEMA.to_string(),
                capability_ids: vec!["builder-push-branch".to_string()],
            },
            "builder-push-branch",
            &[
                ("endpoint".to_string(), "host;rm -rf /".to_string()),
                ("branch".to_string(), "main".to_string()),
            ],
        );
        assert!(matches!(decision, CapabilityDecision::Blocked { .. }));
    }

    #[test]
    fn branch_name_uses_allowlist_not_denylist() {
        // Regression lock: the catalogue and cynic-host transport_caps must
        // agree on BranchName. A denylist accepted `a@b` where transport
        // refused it — one concept must have one validator.
        for accepted in ["main", "feature/foo", "release-1.2", "a_b.c"] {
            assert!(
                ParamKind::BranchName.validate(accepted).is_empty(),
                "{accepted} must be accepted"
            );
        }
        for refused in ["a@b", "a#b", "a+b", "a=b", "a%b", "a~b", "a b", "a..b"] {
            assert!(
                ParamKind::BranchName
                    .validate(refused)
                    .contains(&"branch_unsafe_character".to_string())
                    || ParamKind::BranchName
                        .validate(refused)
                        .contains(&"branch_traversal".to_string())
                    || ParamKind::BranchName
                        .validate(refused)
                        .contains(&"branch_whitespace".to_string()),
                "{refused} must be refused, got {:?}",
                ParamKind::BranchName.validate(refused)
            );
        }
    }

    #[test]
    fn sha256_kind_validates() {
        assert!(ParamKind::Sha256Hex.validate(&"a".repeat(64)).is_empty());
        assert!(ParamKind::Sha256Hex
            .validate("abc")
            .contains(&"sha256_not_64_hex".to_string()));
    }

    fn catalog_with_root_caps() -> CapabilityCatalog {
        serde_json::from_str(
            r#"{
            "schema": "cynic.capability_catalog.v1",
            "capabilities": [
                {"id":"pct-list","domain":"hypervisor","argv_template":["/usr/sbin/pct","list"],"requires_root":true,"purpose":"list"},
                {"id":"pct-clone","domain":"hypervisor","argv_template":["/usr/sbin/pct","clone","{source}","{target}","--full"],"params":[{"name":"source","kind":"vmid"},{"name":"target","kind":"vmid"}],"requires_root":true,"purpose":"clone"},
                {"id":"node-control","domain":"runtime","argv_template":["node","control"],"requires_root":false,"purpose":"runtime"}
            ]
        }"#,
        )
        .expect("valid catalog")
    }

    #[test]
    fn sudoers_rules_only_includes_root_capabilities() {
        let catalog = catalog_with_root_caps();
        let rules = catalog.sudoers_rules("cynic", "/usr/local/bin/cynic");
        assert_eq!(rules.len(), 2);
        assert!(rules
            .iter()
            .all(|r| r.contains(" privilege exec ")));
        assert!(!rules.iter().any(|r| r.contains("node-control")));
    }

    #[test]
    fn sudoers_rules_name_the_capability_id_not_the_argv() {
        let catalog = catalog_with_root_caps();
        let rules = catalog.sudoers_rules("cynic", "/usr/local/bin/cynic");
        let clone_rule = rules
            .iter()
            .find(|r| r.contains("pct-clone"))
            .expect("clone rule");
        assert!(
            !clone_rule.contains("/usr/sbin/pct"),
            "the rule must not embed the resolved argv: {clone_rule}"
        );
        assert!(
            !clone_rule.contains('{'),
            "no unresolved placeholders: {clone_rule}"
        );
        // The rule matches what the binary RECEIVES: --param name=<class>
        assert!(
            clone_rule.contains("--param source="),
            "params must appear as --param name=class: {clone_rule}"
        );
    }

    #[test]
    fn sudoers_params_use_bounded_classes_not_bare_wildcard() {
        let catalog = catalog_with_root_caps();
        let rules = catalog.sudoers_rules("cynic", "/usr/local/bin/cynic");
        let clone_rule = rules
            .iter()
            .find(|r| r.contains("pct-clone"))
            .expect("clone rule");
        // source/target are declared as Vmid in the fixture → [0-9]*
        assert!(
            clone_rule.contains("--param source=[0-9]*"),
            "Vmid must render as [0-9]*, not *: {clone_rule}"
        );
        assert!(
            !clone_rule.ends_with(" *"),
            "no trailing bare wildcard: {clone_rule}"
        );
    }

    #[test]
    fn sudoers_escapes_colon_separator() {
        // MEASURED: a bare `:` in a sudoers rule is a syntax error
        // (visudo -c: "syntax error"). It must be escaped.
        assert_eq!(escape_sudoers("write:repository"), "write\\:repository");
        assert_eq!(escape_sudoers("plain"), "plain");
    }

    #[test]
    fn sudoers_rules_are_deterministic() {
        let catalog = catalog_with_root_caps();
        let a = catalog.sudoers_rules("cynic", "/usr/local/bin/cynic");
        let b = catalog.sudoers_rules("cynic", "/usr/local/bin/cynic");
        assert_eq!(a, b);
    }

    #[test]
    fn sudoers_file_is_newline_terminated() {
        let catalog = catalog_with_root_caps();
        let file = catalog.sudoers_file("cynic", "/usr/local/bin/cynic");
        assert!(file.ends_with('\n'), "sudoers file must end with a newline");
        assert_eq!(file.lines().count(), 2);
    }

    #[test]
    fn sudoers_rules_empty_when_no_root_caps() {
        let catalog: CapabilityCatalog = serde_json::from_str(
            r#"{"schema":"cynic.capability_catalog.v1","capabilities":[{"id":"x","domain":"d","argv_template":["/bin/true"],"requires_root":false,"purpose":"p"}]}"#,
        )
        .expect("valid");
        assert!(catalog.sudoers_rules("cynic", "/usr/local/bin/cynic").is_empty());
    }
}
