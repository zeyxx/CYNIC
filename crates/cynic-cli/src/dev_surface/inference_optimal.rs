//! Thin CLI glue for `cynic dev inference-optimal`.
//!
//! Queries the data-centric empirical performance ledger, runs the closed-loop
//! constraint solver, and emits the optimal verified execution vector.

use std::path::PathBuf;
use std::process::ExitCode;

use cynic_inference::{
    ConstraintFilter, EmpiricalStorePort, FsEmpiricalStore, HardwareBudget, OptimizationGoal,
    math::roofline::assess_roofline,
};
use cynic_protocol::{ContextTier, INFERENCE_OPTIMAL_PROFILE_SCHEMA};

pub(super) fn run(
    json: bool,
    model: String,
    context: Option<String>,
    goal_str: String,
    ledger: Option<PathBuf>,
    export_toml: Option<PathBuf>,
) -> ExitCode {
    if !json {
        eprintln!("cynic dev inference-optimal requires --json");
        return ExitCode::FAILURE;
    }

    let hardware = probe_hardware_budget();
    let system_reserve_bytes = compute_dynamic_reserve(hardware.total_ram_bytes);

    let store = match ledger {
        Some(p) => FsEmpiricalStore::open(p),
        None => FsEmpiricalStore::open_default(),
    };

    let store = match store {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open empirical store: {e}");
            return ExitCode::FAILURE;
        }
    };

    let goal = match goal_str.to_lowercase().as_str() {
        "throughput" | "max_throughput" => OptimizationGoal::MaxThroughput,
        "latency" | "min_latency" => OptimizationGoal::MinLatency,
        "jitter" | "min_jitter" => OptimizationGoal::MinJitter,
        "context" | "max_context" => OptimizationGoal::MaxContext,
        _ => OptimizationGoal::Balanced,
    };

    let min_context = context.and_then(|c| match c.to_lowercase().as_str() {
        "4k" => Some(ContextTier::Ctx4k),
        "8k" => Some(ContextTier::Ctx8k),
        "16k" => Some(ContextTier::Ctx16k),
        "32k" => Some(ContextTier::Ctx32k),
        "64k" => Some(ContextTier::Ctx64k),
        "128k" => Some(ContextTier::Ctx128k),
        _ => None,
    });

    let constraints = ConstraintFilter {
        min_context,
        ..Default::default()
    };

    // Run solver synchronously
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let (scorecard, optimal_entry) = rt.block_on(async {
        let sc = store.get_scorecard(&model).await.unwrap_or_else(|_| {
            cynic_inference::ModelEmpiricalScorecard {
                model_alias: model.clone(),
                total_benchmarked_points: 0,
                safe_points_count: 0,
                max_viable_context: ContextTier::Ctx4k,
                peak_throughput_tok_sec: 0.0,
                min_ttft_ms: 0.0,
                best_balanced_config: None,
            }
        });

        let opt = store
            .resolve_optimal(&model, hardware, system_reserve_bytes, &constraints, goal)
            .await
            .unwrap_or(None);
        (sc, opt)
    });

    if let Some(out_path) = &export_toml {
        match &optimal_entry {
            Some(entry) => {
                let toml_str = format!(
                    r#"# Generated ephemeral projection from CYNIC Empirical Ledger
schema = "cynic.inference.config.v1"

[[backend]]
name = "{}"
model = "{}"
sovereign = true
base_url = "http://127.0.0.1:8080/v1"
context_size = {}
concurrency_slots = 1

[backend.launch]
threads_gen = {}
threads_batch = {}
batch_size = {}
ubatch_size = {}
flash_attn = {}
gpu_layers = {}
cache_type_k = "{:?}"
cache_type_v = "{:?}"
"#,
                    entry.model_alias,
                    entry.config.model_file,
                    entry.config.context_tier.token_count(),
                    entry.config.threads_gen,
                    entry.config.threads_batch,
                    entry.config.batch_size,
                    entry.config.ubatch_size,
                    entry.config.flash_attn,
                    entry.config.gpu_layers,
                    entry.config.kv_quant,
                    entry.config.kv_quant,
                );
                if let Err(e) = std::fs::write(out_path, toml_str) {
                    eprintln!("Failed to write export TOML to {}: {e}", out_path.display());
                    return ExitCode::FAILURE;
                }
            }
            None => {
                eprintln!(
                    "Cannot export TOML: no safe optimal configuration found in empirical ledger for model '{}'",
                    model
                );
                return ExitCode::FAILURE;
            }
        }
    }

    // A physical bound is emitted only from Roofline inputs retained with this
    // empirical record. Older ledgers remain valid and report the missing
    // observation explicitly.
    let roofline = optimal_entry
        .as_ref()
        .and_then(|entry| entry.roofline_inputs)
        .and_then(assess_roofline)
        .map(|assessment| serde_json::json!({ "status": "observed", "assessment": assessment }))
        .unwrap_or_else(|| {
            serde_json::json!({
                "status": "unknown",
                "blockers": [
                    "sustained_bandwidth_bytes_per_sec_not_measured",
                    "decode_bytes_per_token_not_measured",
                ],
            })
        });

    let report = serde_json::json!({
        "schema": INFERENCE_OPTIMAL_PROFILE_SCHEMA,
        "model": model,
        "scorecard": scorecard,
        "optimal_entry": optimal_entry,
        "live_hardware": {
            "total_ram_gb": hardware.total_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            "available_ram_gb": hardware.available_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            "system_reserve_gb": system_reserve_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
        },
        "roofline": roofline,
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );

    ExitCode::SUCCESS
}

fn probe_hardware_budget() -> HardwareBudget {
    let meminfo_path = std::path::Path::new("/proc/meminfo");
    if let Ok(content) = std::fs::read_to_string(meminfo_path) {
        let mut total_kb = 0u64;
        let mut avail_kb = 0u64;

        for line in content.lines() {
            if let Some(val) = parse_meminfo_kb(line, "MemTotal:") {
                total_kb = val;
            } else if let Some(val) = parse_meminfo_kb(line, "MemAvailable:") {
                avail_kb = val;
            }
        }

        if total_kb > 0 {
            return HardwareBudget {
                total_ram_bytes: total_kb * 1024,
                available_ram_bytes: avail_kb * 1024,
                gpu_vram_bytes: 8 * 1024 * 1024 * 1024,
            };
        }
    }

    HardwareBudget {
        total_ram_bytes: 32 * 1024 * 1024 * 1024,
        available_ram_bytes: 24 * 1024 * 1024 * 1024,
        gpu_vram_bytes: 8 * 1024 * 1024 * 1024,
    }
}

fn parse_meminfo_kb(line: &str, key: &str) -> Option<u64> {
    if line.starts_with(key) {
        line.split_whitespace()
            .nth(1)
            .and_then(|v| v.parse::<u64>().ok())
    } else {
        None
    }
}

fn compute_dynamic_reserve(total_ram_bytes: u64) -> u64 {
    let ten_percent = total_ram_bytes / 10;
    let min_reserve = 2 * 1024 * 1024 * 1024;
    let max_reserve = 6 * 1024 * 1024 * 1024;
    ten_percent.clamp(min_reserve, max_reserve)
}
