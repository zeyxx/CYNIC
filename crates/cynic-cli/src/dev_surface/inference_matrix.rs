//! Thin CLI glue for `cynic dev inference-matrix`.
//!
//! Delegates all domain logic to `cynic_inference::application::matrix_sweep_executor`.

use std::path::PathBuf;
use std::process::ExitCode;

use std::sync::Arc;
use cynic_inference::application::matrix_live_runner::{
    MatrixLiveRunner, MatrixLiveRunnerConfig,
};
use cynic_inference::application::matrix_sweep_executor::{
    HardwareBudget, ModelUnderTest, build_parametric_grid,
    config_to_llama_argv, standard_scenario_suite, MATRIX_SWEEP_REPORT_SCHEMA,
};
use cynic_inference::adapters::empirical_store::FsEmpiricalStore;
use cynic_protocol::{ContextTier, INFERENCE_MATRIX_SWEEP_SCHEMA};
pub(super) fn run(
    json: bool,
    model: PathBuf,
    params_billions: f32,
    context_filter: Option<String>,
    execute: bool,
    sample: Option<usize>,
    ledger: Option<PathBuf>,
    output: Option<PathBuf>,
) -> ExitCode {
    if !json {
        eprintln!("inference-matrix: requires --json");
        return ExitCode::FAILURE;
    }

    let weight_bytes = match model.metadata() {
        Ok(m) => m.len(),
        Err(e) => {
            eprintln!("inference-matrix: cannot stat model file: {e}");
            return ExitCode::FAILURE;
        }
    };

    let hardware = probe_hardware_budget();

    let model_under_test = ModelUnderTest {
        alias: model
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string(),
        path: model.clone(),
        weight_bytes,
        approx_params_billions: params_billions,
    };

    // Use observed available RAM and a dynamically computed system reserve.
    // Reserve = 10% of total RAM clamped to [2 GiB, 6 GiB].
    let system_reserve_bytes = compute_dynamic_reserve(hardware.total_ram_bytes);

    let mut grid = build_parametric_grid(&model_under_test, hardware, system_reserve_bytes);

    // Optional filter by context tier.
    if let Some(filter_str) = &context_filter {
        let allowed: std::collections::HashSet<String> = filter_str
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .collect();
        grid.retain(|pt| {
            let tier_str = match pt.context_tier {
                ContextTier::Ctx4k => "4k",
                ContextTier::Ctx8k => "8k",
                ContextTier::Ctx16k => "16k",
                ContextTier::Ctx32k => "32k",
                ContextTier::Ctx64k => "64k",
                ContextTier::Ctx128k => "128k",
            };
            allowed.contains(tier_str)
        });
    }
    if let Some(n) = sample {
        grid.truncate(n);
    }

    let scenarios = standard_scenario_suite();
    let total_grid_points = grid.len();

    let report = if execute {
        let store = match ledger {
            Some(p) => match FsEmpiricalStore::open(p) {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    eprintln!("inference-matrix: failed to open empirical ledger: {e}");
                    return ExitCode::FAILURE;
                }
            },
            None => match FsEmpiricalStore::open_default() {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    eprintln!("inference-matrix: failed to open default empirical ledger: {e}");
                    return ExitCode::FAILURE;
                }
            },
        };

        let runner = MatrixLiveRunner::new(
            MatrixLiveRunnerConfig::default(),
            store,
            hardware,
        );

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let results = rt.block_on(async {
            let mut executed = Vec::new();
            for (idx, pt) in grid.iter().enumerate() {
                eprintln!(
                    "[{}/{}] Executing point: context={:?}, kv={:?}, threads={}",
                    idx + 1,
                    total_grid_points,
                    pt.context_tier,
                    pt.kv_quant,
                    pt.threads_gen
                );
                match runner
                    .execute_point(
                        &model,
                        weight_bytes,
                        params_billions,
                        pt,
                        &scenarios,
                    )
                    .await
                {
                    Ok(entry) => executed.push(entry),
                    Err(e) => {
                        eprintln!("  Error on point: {e}");
                    }
                }
            }
            executed
        });

        serde_json::json!({
            "schema": MATRIX_SWEEP_REPORT_SCHEMA,
            "model_alias": model_under_test.alias,
            "params_billions": params_billions,
            "hardware_available_ram_gb": hardware.available_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            "hardware_total_ram_gb": hardware.total_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            "system_reserve_gb": system_reserve_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            "total_executed_points": results.len(),
            "benchmark_scenarios": scenarios.len(),
            "executed_entries": results,
        })
    } else {
        let plan: Vec<_> = grid
            .iter()
            .map(|pt| {
                let argv = config_to_llama_argv(model.to_str().unwrap_or(""), pt, 8080);
                serde_json::json!({
                    "config": pt,
                    "argv": argv,
                })
            })
            .collect();

        serde_json::json!({
            "schema": INFERENCE_MATRIX_SWEEP_SCHEMA,
            "model_alias": model_under_test.alias,
            "params_billions": params_billions,
            "hardware_available_ram_gb": hardware.available_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            "hardware_total_ram_gb": hardware.total_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            "system_reserve_gb": system_reserve_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            "total_grid_points": total_grid_points,
            "benchmark_scenarios": scenarios.len(),
            "grid_plan": plan,
        })
    };

    let json_str = serde_json::to_string_pretty(&report).unwrap_or_default();

    if let Some(out_path) = output {
        if let Err(e) = std::fs::write(&out_path, &json_str) {
            eprintln!("inference-matrix: failed to write output: {e}");
            return ExitCode::FAILURE;
        }
    } else {
        println!("{json_str}");
    }

    ExitCode::SUCCESS
}

/// Reads /proc/meminfo to get live RAM availability without any fixed hardcoded budget.
fn probe_hardware_budget() -> HardwareBudget {
    let mut total_ram_bytes: u64 = 0;
    let mut available_ram_bytes: u64 = 0;

    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if let Some(val) = parse_meminfo_kb(line, "MemTotal:") {
                total_ram_bytes = val * 1024;
            }
            if let Some(val) = parse_meminfo_kb(line, "MemAvailable:") {
                available_ram_bytes = val * 1024;
            }
        }
    }

    HardwareBudget {
        total_ram_bytes,
        available_ram_bytes,
        gpu_vram_bytes: 0, // Shared UMA — VRAM is a subset of total_ram_bytes.
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

/// Dynamic reserve: 10% of total RAM, clamped to [2 GiB, 6 GiB].
fn compute_dynamic_reserve(total_ram_bytes: u64) -> u64 {
    let ten_percent = total_ram_bytes / 10;
    let min_reserve = 2 * 1024 * 1024 * 1024u64; // 2 GiB
    let max_reserve = 6 * 1024 * 1024 * 1024u64; // 6 GiB
    ten_percent.clamp(min_reserve, max_reserve)
}
