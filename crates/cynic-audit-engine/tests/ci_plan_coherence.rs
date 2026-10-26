//! The CI plan must describe the workflow that runs, and name only real crates.
//!
//! WHY: infra/ci-gates.json was a second description of the gates that no job
//! read, and it restated crate lists the topology already owns. A plan that
//! drifts from .forgejo/workflows/fgate.yml, or names a crate missing from
//! infra/crate-topology.json, lets the verdict prove something other than what
//! actually runs.
//!
//! HOW: parse the plan, the topology and the workflow, then assert: the job set
//! of the plan equals the job set of the workflow; every `requires` resolves and
//! the graph is acyclic; every scope is legal; every `proves` names a topology
//! crate and no gate restates a crate list. Non-vacuous: the plan must declare
//! the expected number of gates, so an empty plan cannot pass.

#![allow(clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const PLAN: &str = "infra/ci-gates.json";
const TOPOLOGY: &str = "infra/crate-topology.json";
const WORKFLOW: &str = ".forgejo/workflows/fgate.yml";
const EXPECTED_GATES: usize = 10;
const LEGAL_SCOPES: [&str; 3] = ["always", "affected", "runner_or_infra"];

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return dir;
        }
        assert!(dir.pop(), "workspace root not found");
    }
}

fn read_json(rel: &str) -> serde_json::Value {
    let raw = std::fs::read_to_string(workspace_root().join(rel)).expect("read json");
    serde_json::from_str(&raw).expect("valid json")
}

fn plan_gates() -> Vec<serde_json::Value> {
    let plan = read_json(PLAN);
    assert_eq!(plan["schema"], "cynic.ci_plan.v1", "plan schema");
    plan["gates"].as_array().expect("gates is an array").clone()
}

fn topology_crates() -> BTreeSet<String> {
    let topo = read_json(TOPOLOGY);
    topo["crates"]
        .as_object()
        .expect("topology crates is an object")
        .keys()
        .cloned()
        .collect()
}

/// Job keys under the top-level `jobs:` block of the workflow. Indentation is
/// exact: a job is two spaces, then a lowercase name, then a colon.
fn workflow_jobs() -> BTreeSet<String> {
    let raw = std::fs::read_to_string(workspace_root().join(WORKFLOW)).expect("read workflow");
    let mut in_jobs = false;
    let mut jobs = BTreeSet::new();
    for line in raw.lines() {
        if line == "jobs:" {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        if let Some(name) = line.strip_prefix("  ").and_then(|l| l.strip_suffix(':')) {
            if !name.starts_with(' ') && !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                jobs.insert(name.to_string());
            }
        }
    }
    jobs
}

#[test]
fn plan_declares_the_expected_number_of_gates() {
    let gates = plan_gates();
    assert_eq!(gates.len(), EXPECTED_GATES, "non-vacuous: the plan must not shrink silently");
}

#[test]
fn plan_jobs_equal_workflow_jobs() {
    let plan_jobs: BTreeSet<String> = plan_gates()
        .iter()
        .map(|g| g["job"].as_str().expect("job is a string").to_string())
        .collect();
    let wf_jobs = workflow_jobs();
    assert!(!wf_jobs.is_empty(), "workflow jobs not parsed");
    assert_eq!(plan_jobs, wf_jobs, "plan and workflow must declare the same jobs");
}

#[test]
fn requires_resolve_and_the_graph_is_acyclic() {
    let gates = plan_gates();
    let ids: BTreeSet<String> = gates.iter().map(|g| g["id"].as_str().expect("id").to_string()).collect();
    assert_eq!(ids.len(), gates.len(), "gate ids must be unique");

    let mut edges: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for g in &gates {
        let id = g["id"].as_str().expect("id").to_string();
        let reqs: Vec<String> = g["requires"]
            .as_array()
            .expect("requires is an array")
            .iter()
            .map(|r| r.as_str().expect("require is a string").to_string())
            .collect();
        for r in &reqs {
            assert!(ids.contains(r), "gate {id} requires unknown gate {r}");
        }
        edges.insert(id, reqs);
    }

    // Depth-first search: a back edge is a cycle.
    fn visit(n: &str, edges: &BTreeMap<String, Vec<String>>, stack: &mut Vec<String>, done: &mut BTreeSet<String>) {
        assert!(!stack.iter().any(|s| s == n), "cycle through gate {n}");
        if done.contains(n) {
            return;
        }
        stack.push(n.to_string());
        for r in &edges[n] {
            visit(r, edges, stack, done);
        }
        stack.pop();
        done.insert(n.to_string());
    }
    let mut done = BTreeSet::new();
    for id in edges.keys() {
        visit(id, &edges, &mut Vec::new(), &mut done);
    }
}

#[test]
fn scopes_are_legal_and_proves_names_a_topology_crate() {
    let crates = topology_crates();
    assert!(!crates.is_empty(), "topology has no crates");
    for g in plan_gates() {
        let id = g["id"].as_str().expect("id");
        let scope = g["scope"].as_str().expect("scope");
        assert!(LEGAL_SCOPES.contains(&scope), "gate {id}: illegal scope {scope}");
        assert!(g.get("crates").is_none(), "gate {id} restates a crate list; use `proves`");
        if scope == "affected" {
            let proves = g["proves"].as_str().unwrap_or_else(|| panic!("gate {id}: affected needs `proves`"));
            assert!(crates.contains(proves), "gate {id} proves {proves}, which is not in the topology");
        }
    }
}
