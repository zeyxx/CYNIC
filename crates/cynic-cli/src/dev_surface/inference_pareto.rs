//! Thin CLI glue for `cynic dev inference-pareto`.
//!
//! Computes and renders multi-objective Pareto frontiers from empirical benchmark telemetry.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use cynic_inference::{ParetoPointSummary, extract_pareto_frontier, render_pareto_chart, render_pareto_table};
use serde::Serialize;

#[derive(Serialize)]
struct ParetoReportView {
    schema: &'static str,
    status: &'static str,
    total_evaluated_count: usize,
    pareto_frontier_count: usize,
    pareto_frontier: Vec<ParetoPointSummary>,
    all_configurations: Vec<ParetoPointSummary>,
}

pub(super) fn run(json: bool, results_file: Option<PathBuf>) -> ExitCode {
    let path = results_file.unwrap_or_else(|| PathBuf::from("benchmark-5700g-empirical-results.json"));
    if !path.exists() {
        eprintln!("inference-pareto: results file not found: {}", path.display());
        return ExitCode::FAILURE;
    }

    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("inference-pareto: cannot read {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let raw_points: Vec<ParetoPointSummary> = match serde_json::from_slice(&bytes) {
        Ok(pts) => pts,
        Err(e) => {
            eprintln!("inference-pareto: invalid json in {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let frontier: Vec<ParetoPointSummary> = extract_pareto_frontier(&raw_points, |p| p.to_objective_vector())
        .into_iter()
        .cloned()
        .collect();

    if json {
        let view = ParetoReportView {
            schema: "cynic.inference.pareto_report.v1",
            status: "ok",
            total_evaluated_count: raw_points.len(),
            pareto_frontier_count: frontier.len(),
            pareto_frontier: frontier,
            all_configurations: raw_points,
        };
        println!("{}", serde_json::to_string_pretty(&view).unwrap_or_default());
    } else {
        println!("\n#####################################################################################");
        println!("  RAPPORT D'ANALYSE DE LA FRONTIÈRE DE PARETO (CYNIC NATIVE FRAMEWORK)");
        println!("#####################################################################################");
        println!("\nTotal de configurations testées : {}", raw_points.len());
        println!("Configurations sur la frontière de Pareto : {}", frontier.len());
        println!("\n🏆 CONFIGURATIONS SUR LA FRONTIÈRE DE PARETO :");
        print!("{}", render_pareto_table(&frontier));
        print!("{}", render_pareto_chart(&raw_points));
    }

    ExitCode::SUCCESS
}
