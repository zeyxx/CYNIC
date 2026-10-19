//! Live routed proof: split-half per-doctype alpha routing on the v2 matrix.
//!
//! Reads the frozen fixture plus the v2 sweep matrix (full alpha-per-query
//! table, no endpoints needed) and measures routed fusion with the predictor
//! in the loop: fit per-doctype MinMax/ECDF alphas on one stratified half,
//! evaluate on the other, swap, average. Reports routed MRR against the best
//! global alpha from the same matrix.
//!
//! Run with:
//! ```bash
//! cargo test -p cynic-inference --test routed_split_half -- --ignored --nocapture
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use cynic_eval::bench::retrieval_gain::{FIXTURE_SCHEMA_V1, Fixture};
use cynic_knowledge::{QueryRoute, RoutingTable, route_query};

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("contracts").exists() {
        if !dir.pop() {
            panic!("Could not find workspace root containing contracts/");
        }
    }
    dir
}

fn gold_group(doctype: &str) -> &'static str {
    if doctype == "rs" {
        "rs"
    } else if doctype == "json" {
        "json"
    } else {
        "md"
    }
}

fn predict_route(query: &str) -> QueryRoute {
    route_query(query)
}

fn route_group(route: QueryRoute) -> &'static str {
    match route {
        QueryRoute::Rs => "rs",
        QueryRoute::Json => "json",
        QueryRoute::Md => "md",
    }
}

#[test]
#[ignore = "needs the v2 sweep matrix artifact from continuous_convex_calibration"]
fn routed_split_half_proof() {
    let root = workspace_root();
    let raw = std::fs::read_to_string(
        root.join("crates/cynic-knowledge/tests/retrieval_gain_labels.json"),
    )
    .expect("read graded labels fixture");
    let fixture: Fixture = serde_json::from_str(&raw).expect("fixture is valid JSON");
    assert_eq!(fixture.schema, FIXTURE_SCHEMA_V1);

    let matrix_raw = std::fs::read_to_string(root.join("target/convex_sweep_report.json"))
        .expect("v2 sweep matrix must exist; run continuous_convex_calibration first");
    let matrix_report: serde_json::Value =
        serde_json::from_str(&matrix_raw).expect("matrix is valid JSON");
    assert_eq!(
        matrix_report["schema"].as_str().unwrap_or(""),
        "cynic.convex_sweep_report.v2",
        "routed proof needs the v2 matrix with per-query rows"
    );

    let queries: Vec<&str> = matrix_report["per_query"]
        .as_array()
        .expect("per_query array")
        .iter()
        .map(|q| q["id"].as_str().expect("query id"))
        .collect();
    let id_to_pos: HashMap<&str, usize> =
        queries.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let alphas: Vec<f64> = matrix_report["matrix"]
        .as_array()
        .expect("matrix array")
        .iter()
        .map(|m| m["alpha"].as_f64().expect("alpha"))
        .collect();
    assert!(!alphas.is_empty(), "sweep must cover alphas");

    let rr = |key: &str, alpha: f64, qid: &str| -> f64 {
        let pos = id_to_pos[qid];
        let row = matrix_report["matrix"]
            .as_array()
            .expect("matrix array")
            .iter()
            .find(|m| (m["alpha"].as_f64().unwrap_or(-1.0) - alpha).abs() < 1e-9)
            .expect("alpha in sweep");
        row[key].as_array().expect("rr vector")[pos]
            .as_f64()
            .expect("rr value")
    };

    let query_text: HashMap<&str, &str> = fixture
        .queries
        .iter()
        .filter(|q| !q.trap)
        .map(|q| (q.id.as_str(), q.query.as_str()))
        .collect();
    let gold_of: HashMap<&str, String> = matrix_report["per_query"]
        .as_array()
        .expect("per_query array")
        .iter()
        .map(|q| {
            (
                q["id"].as_str().expect("query id"),
                q["gold_doctype"].as_str().unwrap_or("?").to_string(),
            )
        })
        .collect();

    // Stratified split-half by gold group.
    let mut by_group: HashMap<&str, Vec<String>> = HashMap::new();
    for qid in queries.iter() {
        by_group
            .entry(gold_group(&gold_of[qid]))
            .or_default()
            .push(qid.to_string());
    }
    let mut halves: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    for ids in by_group.values() {
        for (k, qid) in ids.iter().enumerate() {
            halves[k % 2].push(qid.clone());
        }
    }
    assert_eq!(
        halves[0].len() + halves[1].len(),
        queries.len(),
        "halves must cover every query exactly once"
    );

    let fit_alpha = |train: &[String], group: &str, key: &str| -> f64 {
        let idx: Vec<&String> = train
            .iter()
            .filter(|qid| gold_group(&gold_of[qid.as_str()]) == group)
            .collect();
        assert!(!idx.is_empty(), "every group must train on queries");
        alphas
            .iter()
            .map(|a| {
                let mean = idx.iter().map(|qid| rr(key, *a, qid)).sum::<f64>() / idx.len() as f64;
                (*a, mean)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .expect("non-empty sweep")
            .0
    };

    let table = RoutingTable::measured_defaults();
    let score_half = |fit_on: &[String], test_on: &[String], key: &str| -> f64 {
        let total: f64 = test_on
            .iter()
            .map(|qid| {
                let predicted = route_group(predict_route(query_text[qid.as_str()]));
                let alpha = fit_alpha(fit_on, predicted, key);
                if predicted == "json" {
                    // JSON routes to pure BM25 (alpha 0.00 fuses to the sparse arm).
                    matrix_report["per_query"]
                        .as_array()
                        .expect("per_query array")[id_to_pos[qid.as_str()]]["rr_bm25"]
                        .as_f64()
                        .expect("rr_bm25")
                } else {
                    let _ = table.alpha_for(predict_route(query_text[qid.as_str()]));
                    rr(key, alpha, qid)
                }
            })
            .sum();
        total / test_on.len() as f64
    };

    for key in ["rr_minmax", "rr_ecdf"] {
        let s1 = score_half(&halves[0], &halves[1], key);
        let s2 = score_half(&halves[1], &halves[0], key);
        let global_best = alphas
            .iter()
            .map(|a| queries.iter().map(|qid| rr(key, *a, qid)).sum::<f64>() / queries.len() as f64)
            .fold(0.0_f64, f64::max);
        println!(
            "ROUTED {}: split-half {:.4} / {:.4} mean {:.4} vs best global {:.4}",
            key,
            s1,
            s2,
            (s1 + s2) / 2.0,
            global_best
        );
    }
}
