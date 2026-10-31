//! The committed topology document must be exactly what the generator produces.
//!
//! WHY THIS FILE EXISTS. `infra/crate-topology.json` is part policy and part
//! measurement. The policy half is authored; the measurement half is derived from
//! the manifests. While both lived in one hand-written file, the measurement half
//! drifted: the crate count, the edge count and the binary count were all wrong
//! by the time anyone looked, and one declared edge was missing entirely. A
//! document that is partly generated and partly typed is a document with two
//! authors and no way to tell which half is lying.
//!
//! WHAT IT ASSERTS. That the committed file is byte-identical to a fresh render,
//! and that the render satisfies the invariants the generator refuses to break:
//! the debt count equals the recorded edges, the extraction order really is a
//! topological order, and every crate the rule would flag has a disposition.
//!
//! HOW TO READ A FAILURE. The panic message lists the differing paths. Every one
//! of them is either a manifest that changed without a regeneration, or a policy
//! key the generator rejected — and a rejected policy key is a real finding, not
//! a formatting problem: it means the document claims something the build graph
//! does not.

#![allow(clippy::expect_used)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use cynic_audit_engine::topology_generate;

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return dir;
        }
        assert!(dir.pop(), "workspace root not found");
    }
}

fn render() -> serde_json::Value {
    topology_generate::render(&workspace_root())
        .unwrap_or_else(|error| panic!("the topology generator rejected the committed document: {error}"))
        .document
}

#[test]
fn the_committed_document_is_what_the_generator_produces() {
    let root = workspace_root();
    let rendered = topology_generate::render_text(&root)
        .unwrap_or_else(|error| panic!("the topology generator failed: {error}"));
    let committed = topology_generate::committed(&root)
        .unwrap_or_else(|error| panic!("reading {} failed: {error}", topology_generate::TOPOLOGY_DOC));

    let diff = topology_generate::diff_paths(&committed, &rendered)
        .unwrap_or_else(|error| panic!("the committed document is unreadable: {error}"));
    if diff.is_empty() {
        return;
    }
    panic!(
        "{} is stale: the manifests and the policy keys no longer produce it.\n\
         differences (committed -> rendered):\n{}\n\
         regenerate with: cargo run -q -p cynic-cli --features topology_write -- audit topology --write --apply",
        topology_generate::TOPOLOGY_DOC,
        diff.join("\n"),
    );
}

/// The generator refuses to emit a document ARCH-004 would flag, so a render that
/// succeeded is a document whose own numbers agree with each other. This holds
/// the invariants that are cheap to state and expensive to discover later.
#[test]
fn the_rendered_document_is_internally_coherent() {
    let document = render();
    let crates = document
        .get("crates")
        .and_then(serde_json::Value::as_object)
        .expect("crates is an object");
    let measured = document
        .get("measured")
        .and_then(serde_json::Value::as_object)
        .expect("measured is an object");

    assert_eq!(
        measured["crates"].as_u64(),
        Some(crates.len() as u64),
        "measured.crates does not count the crates it declares"
    );

    let runtime_edges: u64 = crates
        .values()
        .map(|crate_| {
            crate_
                .get("depends_on")
                .and_then(serde_json::Value::as_array)
                .map(|edges| edges.len() as u64)
                .unwrap_or_default()
        })
        .sum();
    assert_eq!(
        measured["runtime_edges"].as_u64(),
        Some(runtime_edges),
        "measured.runtime_edges does not count the declared edges"
    );

    let debt = document
        .get("layer_debt")
        .and_then(serde_json::Value::as_object)
        .expect("layer_debt is an object");
    let edges = debt
        .get("edges")
        .and_then(serde_json::Value::as_array)
        .expect("layer_debt.edges is an array");
    assert_eq!(
        debt["count"].as_u64(),
        Some(edges.len() as u64),
        "layer_debt.count does not count layer_debt.edges"
    );
    assert_eq!(
        measured["upward_edges"].as_u64(),
        Some(edges.len() as u64),
        "the recorded debt and the observed upward edges disagree"
    );

    // Every crate the justification rule would flag must carry a disposition,
    // otherwise ARCH-004 reports it and the document is not describing the
    // workspace it is committed to. Dev consumers are counted from the incoming
    // `dev_depends_on` edges, not from the crate's own outgoing ones: a crate
    // nothing dev-depends on is unwired however many crates it dev-depends on.
    let pending: BTreeSet<String> = document
        .get("pending_disposition")
        .and_then(serde_json::Value::as_object)
        .map(|entries| entries.keys().cloned().collect())
        .unwrap_or_default();
    let dev_consumers = |name: &str| -> usize {
        crates
            .values()
            .filter(|body| {
                body["dev_depends_on"]
                    .as_array()
                    .map(|edges| edges.iter().any(|edge| edge.as_str() == Some(name)))
                    .unwrap_or(false)
            })
            .count()
    };
    let undisclosed: Vec<String> = crates
        .iter()
        .filter(|(name, body)| {
            let binary = body["kind"].as_str() == Some("binary");
            let consumers = body["consumers"].as_array().map(Vec::len).unwrap_or(0);
            !binary && consumers == 0 && dev_consumers(name) == 0 && !pending.contains(*name)
        })
        .map(|(name, _)| name.clone())
        .collect();
    assert!(
        undisclosed.is_empty(),
        "crates needing a disposition have none: {undisclosed:?}"
    );
}

/// The extraction order is the reason the document exists: a vertical slice names
/// crates, and this is the order they must move in. An order that places a crate
/// before its dependency is not an order.
#[test]
fn the_extraction_order_respects_every_edge() {
    let document = render();
    let crates = document
        .get("crates")
        .and_then(serde_json::Value::as_object)
        .expect("crates is an object");
    let order = document
        .get("extraction_order")
        .and_then(serde_json::Value::as_array)
        .expect("extraction_order is an array");

    let mut position = std::collections::HashMap::new();
    for (index, name) in order.iter().enumerate() {
        position.insert(name.as_str().expect("order holds names"), index);
    }
    assert_eq!(
        position.len(),
        crates.len(),
        "extraction_order does not list every crate exactly once"
    );

    for (name, body) in crates {
        let deps = body
            .get("depends_on")
            .and_then(serde_json::Value::as_array)
            .expect("depends_on is an array");
        for dep in deps {
            let dep = dep.as_str().expect("edges name crates");
            let before = position
                .get(dep)
                .unwrap_or_else(|| panic!("{name} depends on {dep}, which is not in the order"));
            let after = position[name.as_str()];
            assert!(
                *before < after,
                "extraction order places {name} before its dependency {dep}"
            );
        }
    }
}

/// The document must own exactly the keys it claims to own.
///
/// `generator.derives` is the generator's statement about which keys are machine
/// owned. A key listed there and absent from the render is a generator that
/// forgot to write; a derived key missing from the list is a key a reader could
/// safely edit by hand that the generator will overwrite. Both are bugs, and
/// neither shows up as a stale document, because the file is regenerated.
#[test]
fn the_derived_key_inventory_matches_what_is_rendered() {
    let document = render();
    let crates = document
        .get("crates")
        .and_then(serde_json::Value::as_object)
        .expect("crates is an object");
    let derives = document
        .get("generator")
        .and_then(|generator| generator.get("derives"))
        .and_then(serde_json::Value::as_array)
        .expect("generator.derives is an array");

    let top_level: BTreeSet<String> = document
        .as_object()
        .expect("the document is an object")
        .keys()
        .map(String::to_string)
        .collect();
    for key in derives {
        let key = key.as_str().expect("derives holds names");
        // `crates.*.kind` and `layer_debt.count` are paths, not top-level keys.
        if key.contains('*') || key.contains('.') {
            continue;
        }
        assert!(
            top_level.contains(key),
            "generator.derives lists `{key}`, which the render does not contain"
        );
    }

    for (name, body) in crates {
        for key in topology_generate::DERIVED_CRATE_KEYS {
            assert!(
                body.get(key).is_some(),
                "crate {name} is missing derived key `{key}`"
            );
        }
    }
}

/// Regeneration helper: write the rendered document to disk when explicitly called.
#[test]
#[ignore]
fn regenerate_topology_from_substrate() {
    let root = workspace_root();
    let rendered = topology_generate::render_text(&root).expect("render");
    let path = root.join(topology_generate::TOPOLOGY_DOC);
    std::fs::write(&path, rendered).expect("write");
}
