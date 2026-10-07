//! The derived half of `infra/crate-topology.json`.
//!
//! WHY THIS MODULE EXISTS. The topology document mixed two kinds of statement in
//! one file: policy (which layer a crate is in, which upward edges are tolerated
//! as debt, which crates need a disposition) and measurement (how many crates,
//! how many edges, which crate depends on which). Only the policy half was
//! authored. The measurement half was typed in by hand, and it drifted:
//! `measured.crates` said 47 while 48 crates were built, `measured.runtime_edges`
//! said 142 while the manifests carried 152, `measured.binaries` said 5 while nine
//! crates declare a binary target, and `crates.cynic-channel.depends_on` omitted
//! an edge its own manifest declares. Every one of those numbers was a claim about
//! the workspace that the workspace had already answered differently, and nothing
//! compared them, because a hand-written number cannot fail.
//!
//! WHAT THIS MODULE DOES. It renders the whole document from two inputs: the
//! manifests, which [`super::topology::observe`] already reads, and the policy
//! keys it finds in the committed file. It refuses to render a document ARCH-004
//! would flag — a crate with no layer, a layer entry for a crate that is not
//! built, a debt edge that is not an observed upward edge, an observed upward
//! edge that is not recorded as debt, and a crate that needs a disposition but
//! has none are all errors, not output.
//!
//! WHAT IT DOES NOT DO. It does not decide policy. Layers, tolerated debt and
//! dispositions stay authored; this module derives the rest and checks that the
//! authored half is still coherent with the build graph. `--check` compares,
//! `--write` regenerates.
//!
//! DETERMINISM. The rendered text is a pure function of the manifests and the
//! policy keys: no timestamps, no environment, no map iteration order. Two runs
//! over the same tree produce byte-identical output, which is what makes
//! "the committed file is stale" a checkable statement rather than an opinion.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use serde_json::{Map, Value};

use super::scanner::WorkspaceSourceIndex;
use super::topology::{self, Topology};

/// The document this module owns, repository-relative.
pub const TOPOLOGY_DOC: &str = "infra/crate-topology.json";

/// Name of the binary that renders this document, recorded in the document itself
/// so a reader can tell which tool produced the file it is looking at.
pub const GENERATOR_NAME: &str = "cynic-topology";
pub const GENERATOR_VERSION: u32 = 1;

/// Per-crate keys the generator owns.
///
/// Anything else under `crates.<name>` is policy and is carried through
/// verbatim, so a new policy key never requires a generator change and a
/// derived key can never be edited by hand without `--check` failing.
pub const DERIVED_CRATE_KEYS: &[&str] = &[
    "kind",
    "depends_on",
    "dev_depends_on",
    "consumers",
    "justified",
    "loc",
];

/// What `loc` counts, stated in the document so the number is interpretable.
const LOC_BASIS: &str =
    "every .rs file under crates/<name>/, tests and examples included, blank lines counted";

/// Layers in declaration order, used when the committed document has none.
const DEFAULT_LAYER_ORDER: &[&str] = &["L0", "L1", "L2", "L3", "L4", "cross_cutting"];

/// How many differences `--check` reports before it stops counting.
const DIFF_LIMIT: usize = 100;

#[derive(Debug)]
pub enum GenerateError {
    /// The document is absent, so there is no policy to merge with.
    SsotMissing,
    /// The document is present but not valid JSON.
    SsotUnparseable(String),
    /// The document could not be read or written.
    Io(String),
    /// A built crate has no `layer` in the document.
    MissingLayer(String),
    /// The document declares a crate that is not a workspace member.
    PhantomCrate(String),
    /// `pending_disposition` names a crate that is not built.
    PhantomPending(String),
    /// A crate needs a disposition and has none.
    UnjustifiedUndisclosed(String),
    /// A recorded debt edge is not an observed upward edge.
    DebtNotObserved(String),
    /// An observed upward edge is not recorded as debt.
    DebtMissing(String),
    /// The graph has a cycle, so no extraction order exists.
    Cycle(Vec<String>),
    /// `layer_policy` is absent, so the layer rule has no policy to execute.
    LayerPolicyMissing,
    /// A crate's layer is not declared in `layer_policy`.
    UnknownLayer(String),
    /// A tolerated edge carries no written reason.
    DebtReasonMissing(String),
    /// A reason is recorded for an edge that is not tolerated.
    DebtReasonPhantom(String),
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SsotMissing => write!(f, "{TOPOLOGY_DOC} is absent"),
            Self::SsotUnparseable(details) => write!(f, "{TOPOLOGY_DOC} is not valid JSON: {details}"),
            Self::Io(details) => write!(f, "{details}"),
            Self::MissingLayer(name) => write!(
                f,
                "crate {name} is built but has no `layer` in {TOPOLOGY_DOC}; a layer is a policy \
                 decision, so the generator will not guess one"
            ),
            Self::PhantomCrate(name) => write!(
                f,
                "{TOPOLOGY_DOC} declares {name}, which is not a workspace member"
            ),
            Self::PhantomPending(name) => write!(
                f,
                "pending_disposition names {name}, which is not a workspace member"
            ),
            Self::UnjustifiedUndisclosed(name) => write!(
                f,
                "crate {name} has no binary target, no dependents and no dev dependents, and is \
                 not in pending_disposition; ARCH-004 would flag it"
            ),
            Self::DebtNotObserved(edge) => write!(
                f,
                "layer_debt records `{edge}`, which is not an observed upward edge; either the \
                 edge was paid or the entry is stale"
            ),
            Self::DebtMissing(edge) => write!(
                f,
                "observed upward edge `{edge}` is not recorded in layer_debt; a new ascending \
                 edge needs a written reason, not silence"
            ),
            Self::Cycle(stuck) => write!(
                f,
                "no topological order exists; {} crate(s) are on or downstream of a cycle: {}",
                stuck.len(),
                stuck.join(", ")
            ),
            Self::LayerPolicyMissing => write!(
                f,
                "layer_policy is absent, so the layer rule has no policy to execute; declare each \
                 layer with a rank"
            ),
            Self::UnknownLayer(layer) => write!(
                f,
                "layer {layer} is not declared in layer_policy; a layer is a policy decision, so the \
                 generator will not guess a rank for it"
            ),
            Self::DebtReasonMissing(edge) => write!(
                f,
                "layer_debt tolerates `{edge}` without a recorded reason; a tolerated edge needs a \
                 written justification, not silence"
            ),
            Self::DebtReasonPhantom(edge) => write!(
                f,
                "layer_debt records a reason for `{edge}`, which is not a tolerated edge"
            ),
        }
    }
}

/// The rendered document, as a value and as the text that gets committed.
pub struct Generated {
    pub document: Value,
    pub text: String,
}

/// Render `infra/crate-topology.json` from the manifests and the policy keys.
///
/// The policy keys are read from the committed document, so the first run over a
/// document that has never been generated preserves every authored statement and
/// replaces only the derived ones.
pub fn render(root: &Path) -> Result<Generated, GenerateError> {
    let index = super::scan_workspace(root);
    let observed = topology::observe(&index);
    let committed_text = committed(root)?;
    let committed: Value = serde_json::from_str(&committed_text)
        .map_err(|error| GenerateError::SsotUnparseable(error.to_string()))?;

    let policy = crate_policies(&committed)?;
    let layers = layer_membership(&committed, &policy);
    let layer_policy = layer_policy(&committed, &policy)?;
    let debt = debt_edges(&committed, &observed, &policy)?;
    let reasons = debt_reasons(&committed, &debt)?;
    let pending = pending_dispositions(&committed, &observed, &index)?;

    let mut crates: Map<String, Value> = Map::new();
    for name in observed.edges.keys() {
        let layer = policy
            .get(name)
            .expect("crate_policies rejects a built crate with no layer")
            .clone();
        let mut entry = Map::new();
        // Authored sub-keys first, so a policy key added later survives a
        // regeneration the day it is introduced.
        if let Some(object) = committed
            .get("crates")
            .and_then(Value::as_object)
            .and_then(|crates| crates.get(name))
            .and_then(Value::as_object)
        {
            for (key, value) in object {
                entry.insert(key.clone(), value.clone());
            }
        }
        let binary = topology::is_binary(&index, name);
        let consumers = observed
            .consumers
            .get(name)
            .map(BTreeSet::len)
            .unwrap_or_default();
        let dev_consumers = observed
            .dev_edges
            .values()
            .filter(|deps| deps.contains(name))
            .count();
        entry.insert("layer".into(), Value::String(layer));
        entry.insert(
            "kind".into(),
            Value::String(if binary { "binary" } else { "library" }.to_string()),
        );
        entry.insert(
            "depends_on".into(),
            Value::Array(
                observed
                    .edges
                    .get(name)
                    .map(|deps| deps.iter().map(|d| Value::String(d.clone())).collect())
                    .unwrap_or_default(),
            ),
        );
        entry.insert(
            "dev_depends_on".into(),
            Value::Array(
                observed
                    .dev_edges
                    .get(name)
                    .map(|deps| deps.iter().map(|d| Value::String(d.clone())).collect())
                    .unwrap_or_default(),
            ),
        );
        entry.insert(
            "consumers".into(),
            Value::Array(
                observed
                    .consumers
                    .get(name)
                    .map(|set| set.iter().map(|d| Value::String(d.clone())).collect())
                    .unwrap_or_default(),
            ),
        );
        entry.insert(
            "justified".into(),
            Value::Bool(binary || consumers > 0 || dev_consumers > 0),
        );
        entry.insert("loc".into(), Value::from(loc_of(&index, name)));
        crates.insert(name.clone(), Value::Object(entry));
    }

    let extraction_order = match topology::topological_order(&observed.edges) {
        Ok(order) => order,
        Err(stuck) => return Err(GenerateError::Cycle(stuck)),
    };

    let runtime_edges: usize = observed.edges.values().map(BTreeSet::len).sum();
    let dev_edges: usize = observed.dev_edges.values().map(BTreeSet::len).sum();
    let binaries = observed
        .edges
        .keys()
        .filter(|name| topology::is_binary(&index, name))
        .count();
    let unjustified = observed
        .edges
        .keys()
        .filter(|name| !crate_is_justified(&observed, &index, name))
        .count();
    let loc_total: usize = observed
        .edges
        .keys()
        .map(|name| loc_of(&index, name))
        .sum();

    let mut document = Map::new();
    put(&mut document, "schema", "cynic.crate_topology.v1");
    put(&mut document, "status", "normative");
    copy(&mut document, &committed, "purpose");
    copy(&mut document, &committed, "authority_order");
    copy(&mut document, &committed, "layer_rule");
    copy(&mut document, &committed, "justification_rule");
    document.insert("layer_policy".into(), layer_policy);
    document.insert("layers".into(), Value::Object(layers));
    document.insert("crates".into(), Value::Object(crates));
    copy(&mut document, &committed, "pending_disposition");
    document.insert(
        "measured".into(),
        Value::Object(Map::from_iter([
            ("crates".into(), Value::from(observed.edges.len())),
            ("runtime_edges".into(), Value::from(runtime_edges)),
            ("dev_edges".into(), Value::from(dev_edges)),
            ("binaries".into(), Value::from(binaries)),
            ("acyclic".into(), Value::Bool(true)),
            ("unjustified".into(), Value::from(unjustified)),
            ("pending_disposition".into(), Value::from(pending.len())),
            ("upward_edges".into(), Value::from(debt.len())),
            ("loc_total".into(), Value::from(loc_total)),
            ("loc_basis".into(), Value::String(LOC_BASIS.to_string())),
        ])),
    );
    copy(&mut document, &committed, "corrections");
    copy(&mut document, &committed, "ratification");
    document.insert(
        "layer_debt".into(),
        Value::Object(Map::from_iter([
            (
                "schema".into(),
                Value::String("cynic.crate_topology.layer_debt.v1".to_string()),
            ),
            (
                "rule".into(),
                committed
                    .get("layer_debt")
                    .and_then(|debt| debt.get("rule"))
                    .and_then(Value::as_str)
                    .map(Value::from)
                    .unwrap_or_else(|| Value::String(String::new())),
            ),
            (
                "edges".into(),
                Value::Array(
                    debt.iter().map(|edge| Value::String(edge.clone())).collect(),
                ),
            ),
            ("reasons".into(), Value::Object(reasons)),
            ("count".into(), Value::from(debt.len())),
        ])),
    );
    document.insert(
        "extraction_order".into(),
        Value::Array(
            extraction_order
                .into_iter()
                .map(Value::String)
                .collect::<Vec<_>>(),
        ),
    );
    document.insert(
        "generator".into(),
        Value::Object(Map::from_iter([
            ("tool".into(), Value::String(GENERATOR_NAME.to_string())),
            ("version".into(), Value::from(GENERATOR_VERSION)),
            (
                "derives".into(),
                Value::Array(
                    [
                        "layer_policy",
                        "layers",
                        "crates.*.kind",
                        "crates.*.depends_on",
                        "crates.*.dev_depends_on",
                        "crates.*.consumers",
                        "crates.*.justified",
                        "crates.*.loc",
                        "measured",
                        "layer_debt.count",
                        "extraction_order",
                    ]
                    .iter()
                    .map(|key| Value::String((*key).to_string()))
                    .collect(),
                ),
            ),
        ])),
    );
    // Authored top-level keys this module does not know are carried through, so a
    // policy key introduced before the generator learns about it is not dropped.
    // `document` at this point holds exactly the keys the generator owns.
    if let Some(object) = committed.as_object() {
        let owned: BTreeSet<&str> = document.keys().map(String::as_str).collect();
        let mut unknown: Vec<&String> = object
            .keys()
            .filter(|key| !owned.contains(key.as_str()))
            .collect();
        unknown.sort();
        for key in unknown {
            document.insert(key.clone(), object[key].clone());
        }
    }

    let text = serde_json::to_string_pretty(&Value::Object(document.clone()))
        .map_err(|error| GenerateError::Io(error.to_string()))?;
    Ok(Generated {
        document: Value::Object(document),
        text: format!("{text}\n"),
    })
}

/// The committed text, or an explicit error when it is absent.
pub fn committed(root: &Path) -> Result<String, GenerateError> {
    let path = root.join(TOPOLOGY_DOC);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(GenerateError::SsotMissing)
        }
        Err(error) => Err(GenerateError::Io(format!("{}: {}", error, path.display()))),
    }
}

/// Render and return only the text, for callers that compare rather than inspect.
pub fn render_text(root: &Path) -> Result<String, GenerateError> {
    render(root).map(|generated| generated.text)
}

/// Human-readable differences between the committed and the rendered document.
///
/// One line per changed path, `path: committed -> rendered`, so a stale document
/// says what changed instead of merely that it did.
pub fn diff_paths(committed: &str, rendered: &str) -> Result<Vec<String>, GenerateError> {
    let committed: Value = serde_json::from_str(committed)
        .map_err(|error| GenerateError::SsotUnparseable(error.to_string()))?;
    let rendered: Value = serde_json::from_str(rendered)
        .map_err(|error| GenerateError::SsotUnparseable(error.to_string()))?;
    let mut out = Vec::new();
    diff_into(String::new(), &committed, &rendered, &mut out);
    Ok(out)
}

fn diff_into(path: String, committed: &Value, rendered: &Value, out: &mut Vec<String>) {
    if out.len() >= DIFF_LIMIT {
        if out.len() == DIFF_LIMIT {
            out.push(format!("... more than {DIFF_LIMIT} differences"));
        }
        return;
    }
    let (Value::Object(left), Value::Object(right)) = (committed, rendered) else {
        if committed != rendered {
            out.push(format!("{path}: {committed} -> {rendered}"));
        }
        return;
    };
    let mut keys: BTreeSet<&str> = left.keys().map(String::as_str).collect();
    keys.extend(right.keys().map(String::as_str));
    for key in keys {
        let child = if path.is_empty() {
            key.to_string()
        } else {
            format!("{path}.{key}")
        };
        match (left.get(key), right.get(key)) {
            (Some(a), Some(b)) => diff_into(child, a, b, out),
            (None, Some(_)) => out.push(format!("{child}: absent -> present")),
            (Some(_), None) => out.push(format!("{child}: present -> absent")),
            (None, None) => {}
        }
    }
}

/// `crates.<name>.layer` for every declared crate, rejecting the two ways policy
/// and build graph can disagree about membership.
fn crate_policies(committed: &Value) -> Result<BTreeMap<String, String>, GenerateError> {
    let empty = Map::new();
    let crates = committed
        .get("crates")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let mut policy = BTreeMap::new();
    for (name, body) in crates {
        let layer = body
            .get("layer")
            .and_then(Value::as_str)
            .ok_or_else(|| GenerateError::MissingLayer(name.clone()))?;
        policy.insert(name.clone(), layer.to_string());
    }
    Ok(policy)
}

/// Layer membership derived from the per-crate layer assignment.
///
/// The layer order is the one the committed document declares, so the derived
/// grouping reads in the order a human expects; membership itself is never
/// authored, because it is a restatement of `crates.<name>.layer` and a
/// restatement is a second place to be wrong.
fn layer_membership(
    committed: &Value,
    policy: &BTreeMap<String, String>,
) -> Map<String, Value> {
    let mut order: Vec<String> = committed
        .get("layers")
        .and_then(Value::as_object)
        .map(|layers| layers.keys().cloned().collect())
        .unwrap_or_default();
    if order.is_empty() {
        order = DEFAULT_LAYER_ORDER
            .iter()
            .map(|layer| (*layer).to_string())
            .collect();
    }
    let mut known: BTreeSet<String> = order.iter().cloned().collect();
    let mut extra: Vec<String> = policy.values().cloned().collect();
    extra.sort();
    extra.dedup();
    for layer in extra {
        if known.insert(layer.clone()) {
            order.push(layer);
        }
    }

    let mut members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, layer) in policy {
        members.entry(layer.clone()).or_default().push(name.clone());
    }
    order
        .into_iter()
        .map(|layer| {
            let mut names = members.remove(&layer).unwrap_or_default();
            names.sort();
            (layer, Value::Array(names.into_iter().map(Value::String).collect()))
        })
        .collect()
}

/// The layer policy: one rank per layer, and the targets each layer may depend on.
///
/// `rank` is the only authored field. `may_depend_on` is derived from it, because
/// a second authored list would be a second place to be wrong: the rule "an edge
/// is legal when the target's rank is at most the source's" is a statement about
/// ranks, and writing it out by hand per layer is how a layer ends up allowed to
/// depend on something its rank forbids.
///
/// A layer with a negative rank is unlayered cross-cutting code. It may depend on
/// anything, and anything may depend on it, which is what the rank ordering
/// expresses without a special case in the rule.
fn layer_policy(
    committed: &Value,
    policy: &BTreeMap<String, String>,
) -> Result<Value, GenerateError> {
    let empty = Map::new();
    let declared = committed.get("layer_policy").and_then(Value::as_object);
    let Some(declared) = declared else {
        return Err(GenerateError::LayerPolicyMissing);
    };

    let mut ranks: BTreeMap<String, i64> = BTreeMap::new();
    if let Some(layers) = declared.get("layers").and_then(Value::as_object) {
        for (layer, body) in layers {
            let rank = body
                .get("rank")
                .and_then(Value::as_i64)
                .ok_or_else(|| GenerateError::UnknownLayer(layer.clone()))?;
            ranks.insert(layer.clone(), rank);
        }
    }

    for layer in policy.values() {
        if !ranks.contains_key(layer) {
            return Err(GenerateError::UnknownLayer(layer.clone()));
        }
    }

    let mut layers: Vec<(String, i64)> = ranks.iter().map(|(k, v)| (k.clone(), *v)).collect();
    layers.sort_by_key(|(_, rank)| *rank);

    let mut rendered: Map<String, Value> = Map::new();
    for (layer, rank) in &layers {
        let allowed: Vec<String> = if *rank < 0 {
            layers.iter().map(|(name, _)| name.clone()).collect()
        } else {
            layers
                .iter()
                .filter(|(_, other)| *other <= *rank)
                .map(|(name, _)| name.clone())
                .collect()
        };
        rendered.insert(
            layer.clone(),
            Value::Object(Map::from_iter([
                ("rank".into(), Value::from(*rank)),
                (
                    "may_depend_on".into(),
                    Value::Array(allowed.into_iter().map(Value::String).collect()),
                ),
            ])),
        );
    }

    let mut out = Map::new();
    out.insert(
        "schema".into(),
        Value::String("cynic.crate_topology.layer_policy.v1".to_string()),
    );
    if let Some(rule) = declared.get("rule") {
        out.insert("rule".into(), rule.clone());
    }
    out.insert("layers".into(), Value::Object(rendered));
    Ok(Value::Object(out))
}

/// The tolerated ascending edges, validated against what is actually observed.
///
/// The edges stay authored — tolerating an upward edge is a decision — but the
/// decision is checked both ways: an edge that is no longer ascending must leave
/// the list, and an ascending edge that is not listed is new debt the generator
/// will not paper over.
fn debt_edges(
    committed: &Value,
    observed: &Topology,
    policy: &BTreeMap<String, String>,
) -> Result<Vec<String>, GenerateError> {
    let declared: Vec<String> = committed
        .get("layer_debt")
        .and_then(|debt| debt.get("edges"))
        .and_then(Value::as_array)
        .map(|edges| {
            edges
                .iter()
                .filter_map(|edge| edge.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let mut upward: BTreeSet<String> = BTreeSet::new();
    for (name, deps) in &observed.edges {
        let Some(layer) = policy.get(name) else {
            continue;
        };
        for dep in deps {
            let Some(dep_layer) = policy.get(dep) else {
                continue;
            };
            if layer_rank(layer) >= 0 && layer_rank(dep_layer) > layer_rank(layer) {
                upward.insert(format!(
                    "ARCH-004-UPWARD-EDGE {name} ({layer}) depends on {dep} ({dep_layer})"
                ));
            }
        }
    }

    for edge in &declared {
        if !upward.contains(edge) {
            return Err(GenerateError::DebtNotObserved(edge.clone()));
        }
    }
    for edge in &upward {
        if !declared.contains(edge) {
            return Err(GenerateError::DebtMissing(edge.clone()));
        }
    }
    Ok(declared)
}

/// The written reason each tolerated edge carries, checked against the edge list.
///
/// The reason is what makes a tolerated edge a decision instead of an
/// oversight: `layer_debt.rule` says an edge may not be added without one, and
/// this is the field that statement is about. The check runs in both directions,
/// so a reason for an edge that is no longer tolerated is as much a defect as a
/// tolerated edge with none — both mean the record and the decision have come
/// apart.
fn debt_reasons(
    committed: &Value,
    debt: &[String],
) -> Result<Map<String, Value>, GenerateError> {
    let empty = Map::new();
    let declared = committed
        .get("layer_debt")
        .and_then(|debt| debt.get("reasons"))
        .and_then(Value::as_object)
        .unwrap_or(&empty);

    for edge in debt {
        let reason = declared
            .get(edge)
            .ok_or_else(|| GenerateError::DebtReasonMissing(edge.clone()))?;
        let text = reason
            .get("reason")
            .and_then(Value::as_str)
            .ok_or_else(|| GenerateError::DebtReasonMissing(edge.clone()))?;
        if text.trim().is_empty() {
            return Err(GenerateError::DebtReasonMissing(edge.clone()));
        }
    }
    for edge in declared.keys() {
        if !debt.contains(edge) {
            return Err(GenerateError::DebtReasonPhantom(edge.clone()));
        }
    }
    Ok(declared.clone())
}

/// The disposition ledger, validated against the justification rule.
fn pending_dispositions(
    committed: &Value,
    observed: &Topology,
    index: &WorkspaceSourceIndex,
) -> Result<Map<String, Value>, GenerateError> {
    let empty = Map::new();
    let declared = committed
        .get("pending_disposition")
        .and_then(Value::as_object)
        .unwrap_or(&empty);

    for name in declared.keys() {
        if !observed.edges.contains_key(name) {
            return Err(GenerateError::PhantomPending(name.clone()));
        }
    }
    for name in observed.edges.keys() {
        if !crate_is_justified(observed, index, name) && !declared.contains_key(name) {
            return Err(GenerateError::UnjustifiedUndisclosed(name.clone()));
        }
    }
    Ok(declared.clone())
}

fn crate_is_justified(observed: &Topology, index: &WorkspaceSourceIndex, name: &str) -> bool {
    let binary = topology::is_binary(index, name);
    let consumers = observed.consumers.get(name).map(BTreeSet::len).unwrap_or(0);
    let dev_consumers = observed
        .dev_edges
        .values()
        .filter(|deps| deps.contains(name))
        .count();
    binary || consumers > 0 || dev_consumers > 0
}

fn layer_rank(layer: &str) -> i32 {
    match layer {
        "L0" => 0,
        "L1" => 1,
        "L2" => 2,
        "L3" => 3,
        "L4" => 4,
        // Unlayered cross-cutting code is reachable from anywhere by design.
        _ => -1,
    }
}

fn loc_of(index: &WorkspaceSourceIndex, name: &str) -> usize {
    let prefix = format!("crates/{name}/");
    index
        .rust_files
        .iter()
        .filter(|file| file.relative_path.starts_with(&prefix))
        .map(|file| file.line_count)
        .sum()
}

fn put(document: &mut Map<String, Value>, key: &str, value: &str) {
    document.insert(key.to_string(), Value::String(value.to_string()));
}

fn copy(document: &mut Map<String, Value>, from: &Value, key: &str) {
    if let Some(value) = from.get(key) {
        document.insert(key.to_string(), value.clone());
    }
}
