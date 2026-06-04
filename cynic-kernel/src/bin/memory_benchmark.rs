#![allow(
    clippy::all,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::unwrap_used,
    clippy::uninlined_format_args,
    clippy::println_empty_string,
    dead_code,
    unused_imports,
    unused_variables
)]
use cynic_kernel::domain::ccm::{self, Crystal, CrystalState};
use cynic_kernel::domain::dog::PHI_INV;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, Deserialize)]
struct RawSignal {
    ticker: String,
    direction: String,
    #[serde(rename = "type")]
    kind: String,
    confidence: String,
    context: String,
}

#[derive(Debug, Deserialize)]
struct SignalBlock {
    block_id: String,
    channel_name: String,
    signals: Vec<RawSignal>,
}

fn confidence_to_score(conf: &str) -> f64 {
    match conf.to_lowercase().as_str() {
        "high" => 0.9,
        "medium" => 0.6,
        "low" => 0.4,
        _ => 0.5,
    }
}

fn main() {
    println!("══════════════════════════════════════════════════════════════════");
    println!("  CYNIC CCM MEMORY BENCHMARK — Epistemic Fast-Track Validation");
    println!("══════════════════════════════════════════════════════════════════");

    let path = "cynic-python/datasets/telegram/signals_v1.jsonl";
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: dataset not found at {path}: {e}");
            return;
        }
    };

    let reader = BufReader::new(file);
    let mut crystals: HashMap<String, Crystal> = HashMap::new();
    let mut stats = BTreeMap::new();

    println!("[1/3] Ingesting historical signals...");
    let mut total_signals = 0;
    for line in reader.lines() {
        let line = line.unwrap();
        let block: SignalBlock = serde_json::from_str(&line).unwrap();

        for signal in block.signals {
            if signal.ticker == "N/A" {
                continue;
            }
            total_signals += 1;

            let id = format!("{}:{}", signal.ticker, signal.direction);
            let score = confidence_to_score(&signal.confidence);
            let source = &block.channel_name;

            let crystal = crystals.entry(id.clone()).or_insert_with(|| Crystal {
                id: id.clone(),
                content: format!("{} {}", signal.ticker, signal.direction),
                domain: "trading".into(),
                confidence: score,
                observations: 0,
                state: CrystalState::Forming,
                created_at: "".into(),
                updated_at: "".into(),
                contributing_verdicts: vec![],
                certainty: 0.0,
                variance_m2: 0.0,
                mean_quorum: 1.0,
                howl_count: 0,
                wag_count: 0,
                growl_count: 0,
                bark_count: 0,
                source_diversity: 0,
                relations: BTreeMap::new(),
                embedding: None,
                contributing_sources: BTreeMap::new(),
                shattered_at: None,
                shatter_reason: None,
                shatter_source: None,
            });

            // Update crystal via domain engine
            let next_obs = crystal.observations + 1;
            let old_mean = crystal.confidence;
            crystal.confidence = (old_mean * crystal.observations as f64 + score) / next_obs as f64;

            let delta = score - old_mean;
            let delta2 = score - crystal.confidence;
            crystal.variance_m2 += delta * delta2;
            crystal.observations = next_obs;

            *crystal
                .contributing_sources
                .entry(source.clone())
                .or_insert(0) += 1;
            crystal.source_diversity = crystal.contributing_sources.len() as u32;

            crystal.certainty = ccm::compute_certainty(
                crystal.variance_m2,
                crystal.observations,
                crystal.source_diversity,
            );

            let old_state = crystal.state.clone();
            crystal.state = ccm::classify(crystal.certainty, crystal.observations);

            if old_state == CrystalState::Forming && crystal.state == CrystalState::Crystallized {
                stats.insert(
                    id.clone(),
                    (
                        crystal.observations,
                        crystal.source_diversity,
                        crystal.confidence,
                    ),
                );
            }
        }
    }

    println!(
        "[2/3] Processing complete. {} signals processed.",
        total_signals
    );
    println!("[3/3] Results — Crystallized Truths:");
    println!("");
    println!(
        "  {:<20} | {:<5} | {:<5} | {:<10} | {:<10}",
        "Truth ID", "Obs", "Sources", "Confidence", "Status"
    );
    println!("  {}", "─".repeat(65));

    let mut crystallized_count = 0;
    for (id, (obs, diversity, conf)) in &stats {
        crystallized_count += 1;
        println!(
            "  {:<20} | {:<5} | {:<7} | {:<10.3} | FAST-TRACKED",
            id, obs, diversity, conf
        );
    }

    if crystallized_count == 0 {
        println!("  (No truths reached crystallization threshold in this run)");
    }

    println!("");
    println!("  Summary:");
    println!("  - Total Distinct Patterns: {}", crystals.len());
    println!("  - Crystallized (Mature):   {}", crystallized_count);
    let fast_tracked = stats.values().filter(|(obs, _, _)| *obs < 21).count();
    println!("  - Fast-Tracked (< 21 obs): {}", fast_tracked);
    println!("══════════════════════════════════════════════════════════════════");
}
