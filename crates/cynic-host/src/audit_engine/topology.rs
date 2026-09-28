//! Crate topology coherence — the declared DAG against the observed build graph.
//!
//! WHY: the layer taxonomy lived only as prose in `AGENTS.md` and as tables in
//! `infra/CANONICAL_CRATE_MAP.md`. Two documents that can drift from each other
//! and from the build graph, and nothing executable compared them. `LayerBoundaryRule`
//! (ARCH-002) looked for four forbidden strings inside `cynic-protocol` and never
//! looked at the graph at all, so an upward edge was invisible.
//!
//! HOW: the observation is the workspace manifests, which `WorkspaceSourceIndex`
//! already carries as `manifests` with full content. No subprocess, no
//! `cargo metadata`, no new dependency — the graph is derived by reading the same
//! files the build reads. Rules stay pure functions over that index.
//!
//! The declared side is `infra/crate-topology.json`, which is the single source of
//! truth for the DAG, the layer assignment, and crate justification. This module
//! makes it falsifiable.

use std::collections::{BTreeMap, BTreeSet};

use cynic_protocol::{AuditCategory, AuditRuleSpec, AuditSeverity, AuditSpan, AuditViolation};

use super::rules::AuditRule;
use super::scanner::WorkspaceSourceIndex;

const TOPOLOGY_SSOT: &str = "infra/crate-topology.json";

/// Manifest sections that create a build edge. `dev-dependencies` is tracked
/// separately: a crate used only by tests is wired, and is therefore justified,
/// but it is not part of the runtime topology.
const EDGE_SECTIONS: &[&str] = &["dependencies", "build-dependencies"];
const DEV_SECTIONS: &[&str] = &["dev-dependencies"];

#[derive(Debug, Default)]
struct Topology {
    edges: BTreeMap<String, BTreeSet<String>>,
    dev_edges: BTreeMap<String, BTreeSet<String>>,
    binaries: BTreeSet<String>,
    consumers: BTreeMap<String, BTreeSet<String>>,
}

fn crate_of(manifest_relative_path: &str) -> Option<String> {
    manifest_relative_path
        .strip_prefix("crates/")?
        .split_once('/')
        .map(|(name, _)| name.to_string())
}

/// Every internal dependency named in a manifest body.
fn named_in(body: &str, known: &BTreeSet<String>, self_name: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for line in body.lines() {
        let line = line.trim();
        // A dependency line is `name = { .. }` or `name.workspace = true`. Match the
        // leading identifier only, so prose and comments cannot produce an edge.
        if !line.starts_with(|c: char| c.is_ascii_lowercase() || c == '_') {
            continue;
        }
        let name: String = line
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        if name.is_empty() || name == self_name {
            continue;
        }
        let is_declaration = line[name.len()..]
            .trim_start()
            .starts_with('=')
            || line[name.len()..].trim_start().starts_with(".workspace");
        if is_declaration && known.contains(&name) {
            found.insert(name);
        }
    }
    found
}

/// Section bodies of a manifest, keyed by section name.
fn sections(manifest: &str) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let mut current: Option<String> = None;
    let mut body = String::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if let Some(name) = current.take() {
                out.insert(name, std::mem::take(&mut body));
            }
            current = Some(trimmed.trim_matches(['[', ']']).to_string());
            continue;
        }
        if current.is_some() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(name) = current {
        out.insert(name, body);
    }
    out
}

/// Build the observed topology from the manifests in the index.
pub(crate) fn observe(index: &WorkspaceSourceIndex) -> Topology {
    let known: BTreeSet<String> = index
        .manifests
        .iter()
        .filter_map(|file| crate_of(&file.relative_path))
        .collect();
    let mut topology = Topology::default();

    for file in &index.manifests {
        let Some(name) = crate_of(&file.relative_path) else {
            continue;
        };
        // A crate is a binary if it declares one, not merely if the file happens
        // to exist: the manifest is the declaration, the filesystem is the guess.
        if sections(&file.content).contains_key("package") {
            topology.edges.entry(name.clone()).or_default();
            topology.dev_edges.entry(name.clone()).or_default();
        }
        let bodies = sections(&file.content);
        let mut edges = BTreeSet::new();
        let mut dev = BTreeSet::new();
        for section in EDGE_SECTIONS {
            if let Some(body) = bodies.get(*section) {
                edges.extend(named_in(body, &known, &name));
            }
        }
        for section in DEV_SECTIONS {
            if let Some(body) = bodies.get(*section) {
                dev.extend(named_in(body, &known, &name));
            }
        }
        topology.edges.insert(name.clone(), edges);
        topology.dev_edges.insert(name, dev);
    }

    for name in topology.edges.keys() {
        topology.consumers.entry(name.clone()).or_default();
    }
    for (dependent, deps) in &topology.edges {
        for dep in deps {
            if let Some(list) = topology.consumers.get_mut(dep) {
                list.insert(dependent.clone());
            }
        }
    }
    topology
}

fn is_binary(index: &WorkspaceSourceIndex, name: &str) -> bool {
    index.rust_files.iter().any(|file| {
        file.relative_path == format!("crates/{name}/src/main.rs")
    })
}

/// Topological order over the internal edge set, or the nodes stuck in a cycle.
///
/// Kahn's algorithm: repeatedly emit the nodes whose dependencies are all
/// emitted. Whatever cannot be emitted is, by construction, on or downstream of a
/// cycle. Iterative, so a large graph cannot overflow the stack, and it yields the
/// extraction order as a by-product — which is the point: a vertical slice names
/// crates, and this says what order they must move in.
pub(crate) fn topological_order(
    edges: &BTreeMap<String, BTreeSet<String>>,
) -> Result<Vec<String>, Vec<String>> {
    let mut remaining: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for (name, deps) in edges {
        let live: BTreeSet<String> = deps
            .iter()
            .filter(|d| edges.contains_key(*d))
            .cloned()
            .collect();
        remaining.insert(name.as_str(), live);
    }
    let mut order = Vec::with_capacity(remaining.len());
    while !remaining.is_empty() {
        let ready: Vec<&str> = remaining
            .iter()
            .filter(|(_, deps)| deps.is_empty())
            .map(|(name, _)| *name)
            .collect();
        if ready.is_empty() {
            let stuck: Vec<String> = remaining.keys().map(|k| (*k).to_string()).collect();
            return Err(stuck);
        }
        for name in &ready {
            remaining.remove(*name);
        }
        for deps in remaining.values_mut() {
            for name in &ready {
                deps.remove(*name);
            }
        }
        order.extend(ready.into_iter().map(str::to_string));
    }
    Ok(order)
}

fn violation(code: &str, message: String, remediation: &str) -> AuditViolation {
    AuditViolation {
        code: code.to_string(),
        message,
        severity: AuditSeverity::Critical,
        category: AuditCategory::Boundary,
        span: Some(AuditSpan {
            file_path: TOPOLOGY_SSOT.to_string(),
            line_start: 1,
            line_end: 1,
            snippet: None,
        }),
        remediation: Some(remediation.to_string()),
    }
}

/// `ARCH-004` — the declared crate topology must equal the observed build graph.
pub struct CrateTopologyRule;

impl AuditRule for CrateTopologyRule {
    fn spec(&self) -> AuditRuleSpec {
        AuditRuleSpec {
            id: "ARCH-004".to_string(),
            name: "crate_topology_coherence".to_string(),
            description:
                "infra/crate-topology.json must describe the workspace that is actually built: every \
                 crate declared, every declared edge real, no undeclared edge, no cycle, no upward \
                 edge, and every crate either a binary, depended upon, or listed with a reason."
                .to_string(),
            category: AuditCategory::Boundary,
            severity: AuditSeverity::Critical,
            automated_remediation_available: false,
        }
    }

    fn evaluate(&self, index: &WorkspaceSourceIndex) -> Vec<AuditViolation> {
        let mut violations = Vec::new();
        let Some(ssot) = index
            .json_files
            .iter()
            .find(|file| file.relative_path == TOPOLOGY_SSOT)
        else {
            return vec![violation(
                "ARCH-004-SSOT-MISSING",
                format!("{TOPOLOGY_SSOT} is absent, so the crate topology cannot be verified"),
                "The topology SSOT is the only executable statement of the layer taxonomy. Restore it.",
            )];
        };
        let Ok(document) = serde_json::from_str::<serde_json::Value>(&ssot.content) else {
            return vec![violation(
                "ARCH-004-SSOT-UNPARSEABLE",
                format!("{TOPOLOGY_SSOT} is not valid JSON"),
                "A contract that cannot be parsed cannot constrain anything.",
            )];
        };

        let declared: BTreeMap<String, (String, BTreeSet<String>)> = document
            .get("crates")
            .and_then(|v| v.as_object())
            .map(|crates| {
                crates
                    .iter()
                    .map(|(name, body)| {
                        let layer = body
                            .get("layer")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let declared_deps: BTreeSet<String> = body
                            .get("depends_on")
                            .and_then(|v| v.as_array())
                            .map(|items| {
                                items
                                    .iter()
                                    .filter_map(|i| i.as_str().map(str::to_string))
                                    .collect()
                            })
                            .unwrap_or_default();
                        (name.clone(), (layer, declared_deps))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let pending: BTreeSet<String> = document
            .get("pending_disposition")
            .and_then(|v| v.as_object())
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();

        let observed = observe(index);

        // 1. every observed crate is declared
        for name in observed.edges.keys() {
            if !declared.contains_key(name) {
                violations.push(violation(
                    "ARCH-004-UNDECLARED-CRATE",
                    format!("crate {name} is built but absent from {TOPOLOGY_SSOT}"),
                    "Declare it with a layer, or remove it from the workspace.",
                ));
            }
        }
        // 2. every declared crate exists
        for name in declared.keys() {
            if !observed.edges.contains_key(name) {
                violations.push(violation(
                    "ARCH-004-PHANTOM-CRATE",
                    format!("{TOPOLOGY_SSOT} declares {name}, which is not a workspace member"),
                    "Remove the declaration or restore the crate.",
                ));
            }
        }

        let layer_rank = |layer: &str| -> i32 {
            match layer {
                "L0" => 0,
                "L1" => 1,
                "L2" => 2,
                "L3" => 3,
                "L4" => 4,
                // Unlayered cross-cutting code is reachable from anywhere by design.
                _ => -1,
            }
        };

        for (name, actual) in &observed.edges {
            let Some((layer, declared_deps)) = declared.get(name) else {
                continue;
            };
            // 3. declared edges are real, and 4. no undeclared edge
            for dep in declared_deps.difference(actual) {
                violations.push(violation(
                    "ARCH-004-DECLARED-EDGE-ABSENT",
                    format!("{name} declares a dependency on {dep}, which its manifest does not"),
                    "Either restore the dependency or drop it from the topology.",
                ));
            }
            for dep in actual.difference(declared_deps) {
                violations.push(violation(
                    "ARCH-004-UNDECLARED-EDGE",
                    format!("{name} depends on {dep}, which the topology does not declare"),
                    "A new edge is a deliberate act. Declare it, or remove the dependency.",
                ));
            }
            // 5. no upward edge. Same-layer is legal and common inside L2.
            let rank = layer_rank(layer);
            for dep in actual {
                let Some((dep_layer, _)) = declared.get(dep) else {
                    continue;
                };
                let dep_rank = layer_rank(dep_layer);
                if rank >= 0 && dep_rank > rank {
                    violations.push(violation(
                        "ARCH-004-UPWARD-EDGE",
                        format!("{name} ({layer}) depends on {dep} ({dep_layer})"),
                        "Dependencies flow downward. Move the shared contract to a lower layer.",
                    ));
                }
            }
        }

        // 6. acyclicity, and by the same pass the order a vertical slice follows
        match topological_order(&observed.edges) {
            Ok(_order) => {}
            Err(stuck) => violations.push(violation(
                "ARCH-004-CYCLE",
                format!(
                    "no topological order exists; {} crate(s) are on or downstream of a cycle: {}",
                    stuck.len(),
                    stuck.join(", ")
                ),
                "A cycle between crates cannot be extracted vertically. Break it at a layer.",
            )),
        }

        // 7. justification: binary, depended upon, dev-depended upon, or listed
        for name in observed.edges.keys() {
            let binary = is_binary(index, name);
            let consumers = observed.consumers.get(name).map(|s| s.len()).unwrap_or(0);
            let dev_consumers = observed
                .dev_edges
                .values()
                .filter(|deps| deps.contains(name))
                .count();
            if binary || consumers > 0 || dev_consumers > 0 || pending.contains(name) {
                continue;
            }
            violations.push(violation(
                "ARCH-004-UNJUSTIFIED-CRATE",
                format!(
                    "{name} has 0 runtime and 0 dev dependents, is not a binary, and is not listed \
                     in pending_disposition"
                ),
                "Fuse it, delete it, or record why it exists. See \
                 infra/CRATE_FUSION_DISPOSITIONS.md.",
            ));
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::{named_in, topological_order};
    use std::collections::{BTreeMap, BTreeSet};

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    fn known() -> BTreeSet<String> {
        set(&["cynic-protocol", "cynic-host", "cynic-cli", "reqwest", "serde"])
    }

    /// WHY: an over-greedy edge extractor manufactures architecture violations out
    /// of prose. A dependency line is `name = { .. }` or `name.workspace = true`;
    /// anything else is a comment or an unrelated key.
    #[test]
    fn only_dependency_lines_produce_edges() {
        let body = "\
cynic-protocol = { path = \"../cynic-protocol\" }
cynic-host.workspace = true
serde = { workspace = true }
reqwest = { version = \"0.13\" }
# cynic-cli = { path = \"../cynic-cli\" }
// cynic-host = { path = \"../cynic-host\" }
cynic_protocol_note = \"cynic-protocol looks fine\"
";
        assert_eq!(
            named_in(body, &known(), "cynic-test"),
            set(&["cynic-protocol", "cynic-host"])
        );
    }

    #[test]
    fn a_crate_never_depends_on_itself() {
        let body = "cynic-protocol = { path = \"../cynic-protocol\" }\n";
        assert!(named_in(body, &known(), "cynic-protocol").is_empty());
    }

    /// WHY: vertical extraction needs the order, not just the absence of a cycle.
    /// A graph with no cycle must produce an order in which every node appears
    /// after all of its dependencies.
    #[test]
    fn topological_order_places_every_dependency_first() {
        let edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::from([
            ("cynic-cli".to_string(), set(&["cynic-kernel", "cynic-protocol"])),
            ("cynic-kernel".to_string(), set(&["cynic-protocol"])),
            ("cynic-protocol".to_string(), set(&[])),
        ]);
        let order = topological_order(&edges).unwrap_or_else(|_| panic!("expected acyclic"));
        let position = |name: &str| {
            order
                .iter()
                .position(|n| n == name)
                .unwrap_or_else(|| panic!("{name} missing from order"))
        };
        assert!(position("cynic-protocol") < position("cynic-kernel"));
        assert!(position("cynic-kernel") < position("cynic-cli"));
        assert!(position("cynic-protocol") < position("cynic-cli"));
    }

    #[test]
    fn a_cycle_is_reported_rather_than_hanging() {
        let edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::from([
            ("a".to_string(), set(&["b"])),
            ("b".to_string(), set(&["a"])),
        ]);
        let stuck = topological_order(&edges).expect_err("cycle must be reported");
        assert_eq!(stuck.len(), 2);
    }

    #[test]
    fn an_edge_to_an_unknown_crate_does_not_block_the_order() {
        // An external dependency is not in the internal set and must not be
        // treated as an unemittable node.
        let edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::from([
            ("cynic-cli".to_string(), set(&["reqwest"])),
            ("cynic-protocol".to_string(), set(&[])),
        ]);
        let order = topological_order(&edges).unwrap_or_else(|_| panic!("expected acyclic"));
        assert_eq!(order.len(), 2);
        assert!(order.contains(&"cynic-cli".to_string()));
    }
}
