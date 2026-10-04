//! Module inversion rule: the hexagonal direction, enforced at module level.
//!
//! WHY. `ARCH-004` guards crate-level direction: a crate must not depend on a
//! higher layer. Nothing guards the same rule one level down, and the module
//! graph is where it is violated:
//!
//!   cynic-node  usecases/network_surfaces_uc.rs:10-25 imports 15
//!               infrastructure modules DIRECTLY (dns_activation_status,
//!               dns_bundle, network_activation_chain, network_actuator,
//!               network_actuator_status, network_evidence,
//!               network_mesh_apply_plan, network_mesh_bundle, network_mesh_plan,
//!               network_repair, network_root_policy, network_runtime_status,
//!               network_shadow_proof, provider_fallback_hosts,
//!               provider_fallback_status)
//!   cynic-host  domain -> execution, domain -> runtime_layout,
//!               ports -> bridge, ports -> execution
//!
//! Measured with a Tarjan SCC pass: cynic-host 2 strongly-connected components
//! (largest 6 members), cynic-node 1 (23 members).
//!
//! LAYER MODEL, and why `domain <-> ports` is NOT reported.
//! In hexagonal architecture the ports ARE the domain's interface:
//! `domain/shared.rs` declares `SubstrateDomainContract` and
//! `domain/{services,privilege,packages,principal}.rs` implement it. A module
//! cannot implement a trait without the trait in scope, so the import runs both
//! ways and no file layout removes it. It is a Rust language artifact, documented
//! as permitted in `ports/mod.rs`. Reporting it would be a false positive that
//! trains the reader to ignore the rule.
//!
//!   inside  : domain + ports          the hexagon's interior and its interface
//!   app     : usecases                the application layer
//!   outside : adapters + every other top-level module of the crate
//!
//! An inversion is `inside -> outside` or `app -> outside`. Nothing else.
//!
//! LEDGER. `infra/module-inversion-ledger.json` declares the inversions present
//! today. Shrink-only, same discipline as `audit-debt-ledger.json`: a new edge
//! fails, and a ledger entry with no observed edge fails too.

use std::collections::{BTreeMap, BTreeSet};

use cynic_protocol::{AuditCategory, AuditRuleSpec, AuditSeverity, AuditViolation};

use super::rules::AuditRule;
use super::scanner::WorkspaceSourceIndex;

const RULE_ID: &str = "ARCH-005";
const LEDGER_PATH: &str = "infra/module-inversion-ledger.json";

/// Modules that form the hexagon's interior. They may depend on each other.
const INSIDE: &[&str] = &["domain", "ports"];

/// The application layer. It may call inside, never outside.
const APP: &[&str] = &["usecases", "application"];

pub struct ModuleInversionRule;

/// Top-level module name for a source file, relative to its crate's `src/`.
///
/// `src/a/mod.rs` and `src/a.rs` both yield `a`. The crate root (`lib.rs`,
/// `main.rs`) yields `None`: it is the assembly point, and a root importing
/// anything is not a layering violation.
fn module_of(relative_path: &str) -> Option<String> {
    // crates/<crate>/src/<module>/...
    let after_src = relative_path.split("/src/").nth(1)?;
    let stem = after_src.strip_suffix(".rs")?;
    let mut parts: Vec<&str> = stem.split('/').collect();
    if matches!(parts.last(), Some(&"mod") | Some(&"lib") | Some(&"main")) {
        parts.pop();
    }
    parts.first().map(|s| s.to_string())
}

/// The crate a source file belongs to.
fn crate_of(relative_path: &str) -> Option<String> {
    let rest = relative_path.strip_prefix("crates/")?;
    rest.split('/').next().map(|s| s.to_string())
}

/// `use crate::X` / `pub use crate::X` targets, with the line number.
fn crate_imports(content: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut in_test_region = false;
    let mut depth: i32 = 0;
    let mut test_started = false;

    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        // Skip `#[cfg(test)]` regions: a test reaching across modules is not an
        // architectural dependency, and counting it invents cycles the compiled
        // library does not have.
        if line.contains("cfg(test)") {
            in_test_region = true;
            depth = 0;
            test_started = false;
        }
        if in_test_region {
            depth += line.matches('{').count() as i32 - line.matches('}').count() as i32;
            if line.contains('{') {
                test_started = true;
            }
            if test_started && depth <= 0 {
                in_test_region = false;
            }
            continue;
        }
        if line.starts_with("//") {
            continue;
        }
        let Some(rest) = line
            .strip_prefix("use crate::")
            .or_else(|| line.strip_prefix("pub use crate::"))
            .or_else(|| {
                // `use crate::x::{` may follow attributes on the same line
                line.find("use crate::")
                    .map(|i| &line[i + "use crate::".len()..])
            })
        else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            found.push((idx + 1, name));
        }
    }
    found
}

fn is_inversion(from: &str, to: &str) -> bool {
    let from_inside = INSIDE.contains(&from);
    let from_app = APP.contains(&from);
    if !from_inside && !from_app {
        return false;
    }
    // A dependency on inside is always fine; on anything else is not.
    !INSIDE.contains(&to)
}

/// The declared inversions, keyed `crate|from -> to`.
fn declared(root: &std::path::Path) -> Option<BTreeMap<String, usize>> {
    let text = std::fs::read_to_string(root.join(LEDGER_PATH)).ok()?;
    let doc: serde_json::Value = serde_json::from_str(&text).ok()?;
    let mut out = BTreeMap::new();
    for entry in doc.get("entries")?.as_array()? {
        out.insert(
            entry.get("identity")?.as_str()?.to_string(),
            entry.get("count")?.as_u64()? as usize,
        );
    }
    Some(out)
}

impl AuditRule for ModuleInversionRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: RULE_ID.to_string(),
            name: "module_inversion".to_string(),
            description: format!(
                "At module level, `domain`/`ports` must not depend on infrastructure, and \
                 `usecases` must not either. The application layer reaching a concrete module \
                 means a port is missing. Declared inversions live in {LEDGER_PATH} and the \
                 list may only shrink. `domain <-> ports` is intra-layer and never reported."
            ),
            category: AuditCategory::Boundary,
            severity: AuditSeverity::Critical,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();

        // Build the observed inversion set with its evidence.
        let mut observed: BTreeMap<String, (usize, String)> = BTreeMap::new();
        for file in &index.rust_files {
            let Some(krate) = crate_of(&file.relative_path) else {
                continue;
            };
            let Some(from) = module_of(&file.relative_path) else {
                continue;
            };
            for (line, target) in crate_imports(&file.content) {
                if target == from || !is_inversion(&from, &target) {
                    continue;
                }
                let identity = format!("{krate}|{from} -> {target}");
                let entry = observed.entry(identity).or_insert((0, String::new()));
                entry.0 += 1;
                if entry.1.is_empty() {
                    entry.1 = format!("{}:{}", file.relative_path, line);
                }
            }
        }

        // Find the workspace root from any source file: all paths are
        // repo-relative, so walk up from the first one.
        let Some(root) = index.rust_files.first().map(|f| {
            let mut p = f.absolute_path.clone();
            for _ in 0..f.relative_path.matches('/').count() + 1 {
                p.pop();
            }
            p
        }) else {
            return violations;
        };

        let Some(declared) = declared(&root) else {
            violations.push(AuditViolation {
                code: format!("{RULE_ID}-LEDGER-UNREADABLE"),
                message: format!(
                    "{LEDGER_PATH} could not be read or parsed; the module inversion \
                     comparator could not run. This is an explicit unknown, not a pass."
                ),
                severity: AuditSeverity::Critical,
                category: AuditCategory::ResidualUnknown,
                span: None,
                remediation: Some(format!("Restore {LEDGER_PATH} as valid JSON.")),
            });
            return violations;
        };

        for (identity, (count, evidence)) in &observed {
            match declared.get(identity) {
                None => violations.push(AuditViolation {
                    code: format!("{RULE_ID}-NEW-INVERSION"),
                    message: format!(
                        "New module inversion `{identity}` ({count} edge(s)), first at \
                         {evidence}. A lower layer depending on a concrete module means the \
                         boundary is missing."
                    ),
                    severity: AuditSeverity::Critical,
                    category: AuditCategory::Boundary,
                    span: None,
                    remediation: Some(format!(
                        "Introduce the port the caller needs, or record the inversion \
                         deliberately in {LEDGER_PATH}."
                    )),
                }),
                Some(&allowed) if count > &allowed => violations.push(AuditViolation {
                    code: format!("{RULE_ID}-INVERSION-GREW"),
                    message: format!(
                        "Module inversion `{identity}` was recorded {allowed}x and is now \
                         observed {count}x. The list may only shrink."
                    ),
                    severity: AuditSeverity::Critical,
                    category: AuditCategory::Boundary,
                    span: None,
                    remediation: Some(format!(
                        "Remove the new edge, or raise the count in {LEDGER_PATH}."
                    )),
                }),
                Some(_) => {}
            }
        }

        for identity in declared.keys() {
            if !observed.contains_key(identity) {
                violations.push(AuditViolation {
                    code: format!("{RULE_ID}-STALE-ENTRY"),
                    message: format!(
                        "{LEDGER_PATH} records `{identity}` but no such edge was observed. \
                         Paid debt left in the list hides that the list is stale."
                    ),
                    severity: AuditSeverity::Critical,
                    category: AuditCategory::Boundary,
                    span: None,
                    remediation: Some(format!("Delete the `{identity}` entry from {LEDGER_PATH}.")),
                });
            }
        }

        violations
    }
}

/// Guard the layer model itself.
///
/// A rule that silently reports nothing because `INSIDE`/`APP` stopped matching
/// the workspace's module names would look like a clean pass. This asserts the
/// model still describes something real.
pub struct LayerModelDeclaredRule;

impl AuditRule for LayerModelDeclaredRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "ARCH-006".to_string(),
            name: "layer_model_declared".to_string(),
            description: "The module inversion rule only means something while `domain`, \
                          `ports`, and `usecases` exist as top-level modules. If they are \
                          renamed, ARCH-005 would pass vacuously."
                .to_string(),
            category: AuditCategory::ResidualUnknown,
            severity: AuditSeverity::Warning,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for file in &index.rust_files {
            if let Some(m) = module_of(&file.relative_path) {
                seen.insert(m);
            }
        }
        let mut violations = Vec::new();
        for expected in INSIDE.iter().chain(APP.iter()) {
            if !seen.contains(*expected) {
                violations.push(AuditViolation {
                    code: "ARCH-006-LAYER-MODULE-ABSENT".to_string(),
                    message: format!(
                        "No top-level module named `{expected}` was observed in any crate, but \
                         ARCH-005 classifies layers by that name. ARCH-005 may be passing \
                         vacuously."
                    ),
                    severity: AuditSeverity::Warning,
                    category: AuditCategory::ResidualUnknown,
                    span: None,
                    remediation: Some(
                        "Update INSIDE/APP in audit_engine/module_inversion_rule.rs to match \
                         the new module names."
                            .to_string(),
                    ),
                });
            }
        }
        violations
    }
}
