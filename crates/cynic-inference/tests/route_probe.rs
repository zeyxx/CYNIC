//! Route-probe: can the BM25 top-1 hit doctype predict the gold doctype?
//!
//! Pure lexical pass, no embeddings. For each non-trap fixture query prints
//! the doctype of the top BM25 hit versus the gold doctype, plus the
//! confusion summary. This measures the feasibility of per-data-type
//! alpha routing without an oracle.
//!
//! Run with:
//! ```bash
//! cargo test -p cynic-inference --test route_probe -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use cynic_eval::bench::retrieval_gain::{FIXTURE_SCHEMA_V1, Fixture};
use cynic_knowledge::sync_workspace_corpus;

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("contracts").exists() {
        if !dir.pop() {
            panic!("Could not find workspace root containing contracts/");
        }
    }
    dir
}

fn doctype_of(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or("?")
}

#[test]
#[ignore = "needs workspace corpus (no endpoints required)"]
fn probe_routing_signal() {
    let root = workspace_root();
    let raw = std::fs::read_to_string(
        root.join("crates/cynic-knowledge/tests/retrieval_gain_labels.json"),
    )
    .expect("read graded labels fixture");
    let fixture: Fixture = serde_json::from_str(&raw).expect("fixture is valid JSON");
    assert_eq!(fixture.schema, FIXTURE_SCHEMA_V1);

    let cache_file = root.join("target/eval_corpus_cache.json");
    let (_cache, index) = sync_workspace_corpus(&root, Some(&cache_file), 5);

    let mut agree = 0;
    let mut total = 0;
    let mut confusion: std::collections::BTreeMap<(String, String), usize> =
        std::collections::BTreeMap::new();
    for labeled in fixture.queries.iter().filter(|q| !q.trap) {
        let mut gold: Vec<&str> = labeled
            .grades
            .iter()
            .filter(|(_, g)| **g == 2)
            .map(|(p, _)| doctype_of(p))
            .collect();
        gold.sort_unstable();
        gold.dedup();
        let gold_dt = gold.join("+");
        let top = index.search(&labeled.query, 1);
        let pred = top
            .first()
            .map(|h| doctype_of(&h.chunk.source_path).to_string())
            .unwrap_or_else(|| "?".to_string());
        // A prediction hits when it names any gold doctype (mixed golds).
        let hit = gold.iter().any(|g| *g == pred);
        if hit {
            agree += 1;
        }
        total += 1;
        *confusion
            .entry((pred.clone(), gold_dt.clone()))
            .or_insert(0) += 1;
        if !hit {
            println!("MISS {:<28} pred={} gold={}", labeled.id, pred, gold_dt);
        } else {
            println!("HIT {:<28} pred={} gold={}", labeled.id, pred, gold_dt);
        }
    }
    println!(
        "ROUTE SIGNAL: {}/{} top-1 doctype hits ({:.3})",
        agree,
        total,
        agree as f64 / total as f64
    );
    for ((pred, gold), n) in &confusion {
        println!("  pred={:<6} gold={:<10} n={}", pred, gold, n);
    }
}
