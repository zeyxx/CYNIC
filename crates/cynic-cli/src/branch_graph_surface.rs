//! Thin CLI projection for branch graph observation.
//! Clap parse lives in callers (`BranchCommand` / `GitCommand`); this module only injects
//! `LocalBranchGraphAdapter` and prints JSON/human (no policy).

use std::process::ExitCode;

use cynic_host::adapters::local_branch_graph::LocalBranchGraphAdapter;
use cynic_host::domain::branch_graph::port::BranchGraphObservationPort;

use crate::cli_json::emit_pretty_json;

pub(crate) fn execute_branch_graph(json: bool) -> ExitCode {
    let adapter = LocalBranchGraphAdapter;
    let graph = match adapter.observe_branch_graph() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("branch graph observation failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    if json {
        return emit_pretty_json(&graph);
    }
    println!(
        "Branch graph: root={} commit={} nodes={} schema={}",
        graph.root,
        if graph.root_commit.is_empty() { "-" } else { &graph.root_commit },
        graph.nodes.len(),
        graph.schema
    );
    if graph.nodes.is_empty() {
        println!("(no branches)");
    } else {
        println!("{:<45} {:<12} {:<15} UPSTREAM", "BRANCH", "COMMIT", "PARENT");
        println!("{}", "-".repeat(90));
        for n in &graph.nodes {
            let short = if n.commit.len() > 12 { &n.commit[..12] } else { &n.commit };
            let parent = n.parent.as_deref().unwrap_or("-");
            println!(
                "{:<45} {:<12} {:<15} {}",
                n.name,
                short,
                parent,
                if n.has_upstream { "yes" } else { "no" }
            );
        }
    }
    ExitCode::SUCCESS
}
