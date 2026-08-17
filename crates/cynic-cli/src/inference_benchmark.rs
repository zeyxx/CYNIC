use std::collections::BTreeSet;
use std::{process::ExitCode, time::Duration};

use serde::Serialize;

use crate::agent_harness_adapter::HostResourceObserver;
use crate::hermes_kanban_adapter::HermesKanbanCoordinator;
use cynic_inference::{
    AgentHarnessReport, BENCHMARK_SCHEMA, BenchmarkPlan, BenchmarkReport, HarnessExecutor,
    HarnessSample, HermesRunExecutor, HttpHarnessExecutor, MemorySample, RuntimeKind,
    RuntimePlanSummary, RuntimePlanner, ScenarioSpec, aggregate_reports, assess_baseline,
    canonicalize_dog_runtime_id,
    domain::hardware_profile::{GpuClass, HardwareProfile},
    read_hermes_api_key, run_scenario_with_coordination,
};

const BENCHMARK_MATRIX_SCHEMA: &str = "cynic.agent_harness.benchmark_matrix_report.v1";
const BENCHMARK_EXECUTION_SCHEMA: &str = "cynic.agent_harness.benchmark_execution.v1";

#[derive(Debug, Clone)]
enum MatrixMode {
    Conservative,
    Aggressive(u8),
}

#[derive(Serialize)]
struct MatrixPass {
    pass_id: String,
    mode: String,
    aggressive_fraction: Option<u8>,
    plan_id: String,
    status: String,
    failed_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<BenchmarkReport>,
}

#[derive(Serialize)]
struct MatrixRun {
    schema: &'static str,
    status: &'static str,
    plan_path: String,
    requested_modes: Vec<String>,
    host_gpu: String,
    available_ram_bytes: u64,
    passes: Vec<MatrixPass>,
}

pub(crate) fn execute(
    plan_path: &str,
    baseline_path: Option<&str>,
    llama_endpoint: &str,
    hermes_endpoint: &str,
    json: bool,
) -> ExitCode {
    let plan = match load_plan(plan_path) {
        Ok(plan) => plan,
        Err(error) => return fail(json, error),
    };
    let baseline = match baseline_path.map(load_baseline).transpose() {
        Ok(baseline) => baseline,
        Err(error) => return fail(json, error),
    };
    match run_benchmark_plan(plan, llama_endpoint, hermes_endpoint) {
        Ok(matrix) => {
            let baseline_assessment = baseline
                .as_ref()
                .map(|baseline| assess_baseline(&matrix, baseline));
            if json {
                if let Some(assessment) = &baseline_assessment {
                    let output = serde_json::json!({
                        "schema": BENCHMARK_EXECUTION_SCHEMA,
                        "report": &matrix,
                        "baseline_assessment": assessment,
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output).unwrap_or_else(|error| {
                            format!("{{\"status\":\"failed\",\"error\":\"{error}\"}}")
                        })
                    );
                } else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&matrix).unwrap_or_else(|error| {
                            format!("{{\"status\":\"failed\",\"error\":\"{error}\"}}")
                        })
                    );
                }
            } else {
                for report in &matrix.reports {
                    println!(
                        "{}: {} ({} samples)",
                        report.run_id,
                        report.status,
                        report.samples.len()
                    );
                }
                if let Some(assessment) = &baseline_assessment {
                    println!("baseline {}: {}", assessment.baseline_id, assessment.status);
                }
            }
            if matrix.summary.failed_count > 0
                || baseline_assessment
                    .as_ref()
                    .is_some_and(|assessment| assessment.passed != Some(true))
            {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => fail(json, error),
    }
}

pub(crate) fn execute_matrix(
    plan_path: &str,
    modes: &Vec<String>,
    aggressive_fractions: &Vec<u8>,
    llama_endpoint: &str,
    hermes_endpoint: &str,
    json: bool,
) -> ExitCode {
    let hw = HardwareProfile::probe();
    let plan = match load_plan(plan_path) {
        Ok(plan) => plan,
        Err(error) => return fail(json, error),
    };
    let resolved_modes = match resolve_matrix_modes(modes, aggressive_fractions, &hw) {
        Ok(resolved) => resolved,
        Err(error) => return fail(json, error),
    };
    let mut passes = Vec::new();
    for mode in resolved_modes {
        let pass_id = matrix_pass_id(&mode);
        let mut pass_plan = plan.clone();
        pass_plan.plan_id = format!("{}-{pass_id}", pass_plan.plan_id);
        for planned in &mut pass_plan.scenarios {
            planned.scenario.scenario_id = format!("{}-{pass_id}", planned.scenario.scenario_id);
        }

        let result = run_benchmark_plan(pass_plan.clone(), llama_endpoint, hermes_endpoint);
        match result {
            Ok(report) => {
                let failed_count = report.summary.failed_count;
                passes.push(MatrixPass {
                    pass_id,
                    mode: mode_name(&mode),
                    aggressive_fraction: match mode {
                        MatrixMode::Aggressive(fraction) => Some(fraction),
                        MatrixMode::Conservative => None,
                    },
                    plan_id: pass_plan.plan_id.clone(),
                    status: if failed_count == 0 {
                        "completed".to_string()
                    } else {
                        "failed".to_string()
                    },
                    failed_count,
                    error: None,
                    report: Some(report),
                });
            }
            Err(error) => {
                passes.push(MatrixPass {
                    pass_id,
                    mode: mode_name(&mode),
                    aggressive_fraction: match mode {
                        MatrixMode::Aggressive(fraction) => Some(fraction),
                        MatrixMode::Conservative => None,
                    },
                    plan_id: pass_plan.plan_id.clone(),
                    status: "error".to_string(),
                    failed_count: 0,
                    error: Some(error),
                    report: None,
                });
            }
        }
    }

    let status = if passes.iter().any(|pass| pass.status != "completed") {
        "failed"
    } else {
        "completed"
    };
    let run = MatrixRun {
        schema: BENCHMARK_MATRIX_SCHEMA,
        status,
        plan_path: plan_path.to_string(),
        requested_modes: modes.iter().map(normalize_mode_input).collect(),
        host_gpu: hw
            .gpu
            .vendor_name
            .clone()
            .unwrap_or_else(|| format!("{:?}", hw.gpu.class)),
        available_ram_bytes: hw.available_ram_bytes,
        passes,
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&run)
                .unwrap_or_else(|error| format!("{{\"status\":\"failed\",\"error\":\"{error}\"}}"))
        );
    } else {
        println!("matrix status={}", run.status);
        for pass in &run.passes {
            println!(
                "pass {} [{}] status={} failed={}",
                pass.pass_id, pass.mode, pass.status, pass.failed_count
            );
        }
    }
    if status == "completed" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn run_benchmark_plan(
    plan: BenchmarkPlan,
    llama_endpoint: &str,
    hermes_endpoint: &str,
) -> Result<BenchmarkReport, String> {
    let direct_executor = HttpHarnessExecutor::new(llama_endpoint, None, Duration::from_secs(300))
        .map_err(|error| format!("http_client:{error}"))?;
    let has_hermes_scenario = plan
        .scenarios
        .iter()
        .any(|planned| planned.scenario.runtime_kind == RuntimeKind::HermesAgent);
    let hermes_executor = match read_hermes_api_key() {
        Some(api_key) => Some(
            HermesRunExecutor::new(hermes_endpoint, api_key, Duration::from_secs(300))
                .map_err(|error| format!("hermes_http_client:{error}"))?,
        ),
        None => None,
    };
    let observer = HostResourceObserver;
    let coordinator = HermesKanbanCoordinator { board: None };
    let mut runtime_planner = None;
    let mut hermes_blocked_reason = if has_hermes_scenario && hermes_executor.is_none() {
        Some("missing_hermes_api_key".to_string())
    } else {
        None
    };

    if plan.scenarios.iter().any(|planned| {
        planned.scenario.runtime_kind == RuntimeKind::DirectLlama
            && planned.scenario.probe_runtime_plan
    }) {
        runtime_planner = Some(RuntimePlanner::from_env());
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("runtime:{error}"))?;
    if hermes_blocked_reason.is_none() && has_hermes_scenario && hermes_executor.is_some() {
        let readiness = runtime.block_on(
            hermes_executor
                .as_ref()
                .expect("hermes executor must exist when reason is none")
                .observe_readiness(),
        );
        if !readiness.health_ok || !readiness.capabilities_ok || readiness.model_count == 0 {
            hermes_blocked_reason = Some(format!(
                "hermes_not_ready:health_ok={},capabilities_ok={},model_count={}",
                readiness.health_ok, readiness.capabilities_ok, readiness.model_count
            ));
        }
    }

    let mut reports = Vec::new();
    for planned in plan.scenarios {
        let mut scenario = planned.scenario;
        let mut runtime_launch_plan: Option<RuntimePlanSummary> = None;

        if scenario.runtime_kind == RuntimeKind::DirectLlama && scenario.probe_runtime_plan {
            if let Some(planner) = runtime_planner.as_ref() {
                match resolve_direct_llama_runtime(&scenario, planner) {
                    Ok((resolved, plan)) => {
                        scenario = resolved;
                        runtime_launch_plan = Some(plan);
                    }
                    Err(error) => return Err(format!("runtime_plan:{error}")),
                }
            } else {
                return Err("runtime_plan_disabled".into());
            }
        }

        let run_id = format!("{}-{}", plan.plan_id, scenario.scenario_id);
        if scenario.runtime_kind == RuntimeKind::HermesAgent {
            if let Some(blocked_reason) = hermes_blocked_reason.as_deref() {
                reports.push(blocked_hermes_report(
                    run_id,
                    scenario,
                    blocked_reason,
                    None,
                ));
                continue;
            }
        }
        let executor: &dyn HarnessExecutor = match scenario.runtime_kind {
            RuntimeKind::HermesAgent => hermes_executor
                .as_ref()
                .expect("hermes_blocked_reason should capture missing/blocked executor"),
            _ => &direct_executor,
        };

        let coordination = scenario
            .coordination_kind
            .as_deref()
            .map(|_| &coordinator as &dyn cynic_inference::CoordinationPort);
        let mut report = runtime.block_on(run_scenario_with_coordination(
            run_id,
            "runtime_now",
            scenario,
            executor,
            &observer,
            coordination,
        ));
        report.runtime_launch_plan = runtime_launch_plan;
        reports.push(report);
    }

    Ok(aggregate_reports(plan.plan_id, reports))
}

fn blocked_hermes_report(
    run_id: String,
    scenario: ScenarioSpec,
    reason: &str,
    runtime_transition: Option<cynic_protocol::CycleTransitionRecord>,
) -> AgentHarnessReport {
    let started_at = "blocked".to_string();
    let status = if reason == "missing_hermes_api_key" {
        "blocked_missing_hermes_api_key".to_string()
    } else {
        "blocked_hermes_runtime".to_string()
    };
    let task_id = scenario
        .tasks
        .first()
        .map(|task| task.task_id.clone())
        .unwrap_or_else(|| "blocked".into());
    AgentHarnessReport {
        schema: BENCHMARK_SCHEMA.into(),
        run_id,
        scenario,
        samples: vec![HarnessSample {
            task_id,
            runtime_run_id: None,
            http_status: None,
            prompt_tokens: None,
            output_tokens: None,
            prompt_tokens_per_second: None,
            output_tokens_per_second: None,
            wall_time_ms: Some(0),
            event_count: Some(0),
            event_kinds: Vec::new(),
            event_stream_error: Some(reason.to_string()),
            memory: MemorySample {
                rss_bytes: None,
                available_memory_bytes: None,
                total_memory_bytes: None,
                available_memory_percent: None,
                rss_percent: None,
                vram_bytes: None,
                gtt_bytes: None,
            },
            effective_context_per_slot: None,
            status: status.clone(),
            error: Some(reason.to_string()),
        }],
        started_at: started_at.clone(),
        finished_at: Some(started_at),
        status,
        cycle: None,
        transition: None,
        runtime_transition,
        runtime_launch_plan: None,
    }
}

fn resolve_matrix_modes(
    requested: &[String],
    aggressive_fractions: &[u8],
    hw: &HardwareProfile,
) -> Result<Vec<MatrixMode>, String> {
    let normalized: Vec<String> = if requested.is_empty() {
        vec!["auto".to_string()]
    } else {
        requested
            .iter()
            .map(|mode| mode.to_ascii_lowercase())
            .collect()
    };

    let mut has_conservative = false;
    let mut has_aggressive = false;
    for raw_mode in normalized.iter() {
        match raw_mode.as_str() {
            "conservative" => has_conservative = true,
            "aggressive" => has_aggressive = true,
            "auto" => {
                has_conservative = true;
                if host_supports_aggressive(hw) {
                    has_aggressive = true;
                }
            }
            _ => {
                return Err(format!(
                    "invalid_matrix_mode:{raw_mode} (expected conservative|aggressive|auto)"
                ));
            }
        }
    }

    let mut fractions: Vec<u8> = aggressive_fractions
        .iter()
        .copied()
        .filter(|value| (1..=100).contains(value))
        .collect();
    if fractions.is_empty() {
        fractions.push(50);
    }
    let mut deduped = BTreeSet::new();
    for fraction in fractions {
        deduped.insert(fraction);
    }
    let fractions: Vec<u8> = deduped.into_iter().collect();

    if !has_conservative && !has_aggressive {
        has_conservative = true;
    }

    let mut modes = Vec::new();
    if has_conservative {
        modes.push(MatrixMode::Conservative);
    }
    if has_aggressive {
        for fraction in fractions {
            modes.push(MatrixMode::Aggressive(fraction));
        }
    }
    Ok(modes)
}

fn host_supports_aggressive(hw: &HardwareProfile) -> bool {
    matches!(hw.gpu.class, GpuClass::Integrated)
        && hw
            .gpu
            .vram_total_bytes
            .zip(hw.gtt_total_bytes)
            .is_some_and(|(vram, gtt)| gtt >= vram)
}

fn matrix_pass_id(mode: &MatrixMode) -> String {
    match mode {
        MatrixMode::Conservative => "conservative".to_string(),
        MatrixMode::Aggressive(fraction) => format!("aggressive-{fraction}"),
    }
}

fn mode_name(mode: &MatrixMode) -> String {
    match mode {
        MatrixMode::Conservative => "conservative".to_string(),
        MatrixMode::Aggressive(_) => "aggressive".to_string(),
    }
}

fn normalize_mode_input(mode: &String) -> String {
    mode.to_ascii_lowercase()
}

fn load_plan(plan_path: &str) -> Result<BenchmarkPlan, String> {
    let raw = std::fs::read_to_string(plan_path).map_err(|error| format!("read_plan:{error}"))?;
    let mut plan: BenchmarkPlan =
        serde_json::from_str(&raw).map_err(|error| format!("parse_plan:{error}"))?;
    normalize_plan_runtime_aliases(&mut plan);
    plan.validate()
        .map_err(|errors| format!("invalid_plan:{}", errors.join(",")))?;
    Ok(plan)
}

fn load_baseline(path: &str) -> Result<cynic_inference::BenchmarkBaseline, String> {
    let raw = std::fs::read_to_string(path).map_err(|error| format!("read_baseline:{error}"))?;
    let baseline: cynic_inference::BenchmarkBaseline =
        serde_json::from_str(&raw).map_err(|error| format!("parse_baseline:{error}"))?;
    if baseline.schema != cynic_inference::BENCHMARK_BASELINE_SCHEMA {
        return Err(format!("invalid_baseline_schema:{}", baseline.schema));
    }
    if baseline.baseline_id.trim().is_empty()
        || baseline.plan_id.trim().is_empty()
        || baseline.model_id.trim().is_empty()
        || !baseline.model_digest.starts_with("sha256:")
    {
        return Err("invalid_baseline_identity".into());
    }
    Ok(baseline)
}

fn normalize_plan_runtime_aliases(plan: &mut BenchmarkPlan) {
    for scenario in &mut plan.scenarios {
        if scenario.scenario.runtime_kind != RuntimeKind::HermesAgent {
            continue;
        }
        if scenario.scenario.runtime_id.is_none() {
            continue;
        }
        let runtime_id = scenario
            .scenario
            .runtime_id
            .as_ref()
            .expect("runtime id exists");
        scenario.scenario.runtime_id = Some(canonicalize_dog_runtime_id(runtime_id));
    }
}

fn resolve_direct_llama_runtime(
    scenario: &ScenarioSpec,
    planner: &RuntimePlanner,
) -> Result<(ScenarioSpec, RuntimePlanSummary), String> {
    let alias = scenario
        .runtime_id
        .as_deref()
        .unwrap_or(&scenario.model.parameters.model_id)
        .to_string();
    let runtime_plan = planner
        .plan_for_alias(&alias)
        .map_err(|error| format!("alias={alias} {error:?}"))?;

    let mut model = scenario.model.clone();
    if model.parameters.threads.is_none() {
        model.parameters.threads = u16::try_from(runtime_plan.threads).ok();
    }
    if model.parameters.gpu_layers.is_none() {
        model.parameters.gpu_layers = Some(runtime_plan.n_gpu_layers);
    }
    if model.parameters.context_per_slot.is_none() {
        model.parameters.context_per_slot = Some(runtime_plan.ctx_size);
    }
    if model.parameters.batch_size.is_none() {
        model.parameters.batch_size = parse_numeric_flag(&runtime_plan.engine_args, "--batch-size")
            .or(Some(runtime_plan.threads_batch));
    }
    if model.parameters.ubatch_size.is_none() {
        model.parameters.ubatch_size =
            parse_numeric_flag(&runtime_plan.engine_args, "--ubatch-size")
                .or(Some(runtime_plan.threads_batch / 2));
    }
    if model.parameters.flash_attention.is_none() {
        if runtime_plan.flash_attn.eq_ignore_ascii_case("on") {
            model.parameters.flash_attention = Some(true);
        } else if runtime_plan.flash_attn.eq_ignore_ascii_case("off") {
            model.parameters.flash_attention = Some(false);
        }
    }

    if model.parameters.kv_cache_type_k.is_none() {
        if let Some(value) = parse_string_flag(&runtime_plan.engine_args, "-ctk")
            .or_else(|| parse_string_flag(&runtime_plan.engine_args, "--cache-type-k"))
        {
            model.parameters.kv_cache_type_k = Some(value);
        }
    }
    if model.parameters.kv_cache_type_v.is_none() {
        if let Some(value) = parse_string_flag(&runtime_plan.engine_args, "-ctv")
            .or_else(|| parse_string_flag(&runtime_plan.engine_args, "--cache-type-v"))
        {
            model.parameters.kv_cache_type_v = Some(value);
        }
    }

    let summary = RuntimePlanSummary {
        alias,
        model_path: runtime_plan.model_path,
        host: runtime_plan.host,
        port: runtime_plan.port,
        ctx_size: runtime_plan.ctx_size,
        threads: runtime_plan.threads,
        threads_batch: runtime_plan.threads_batch,
        n_gpu_layers: runtime_plan.n_gpu_layers,
        flash_attn: runtime_plan.flash_attn,
        memory_max: runtime_plan.memory_max,
        cpu_quota: runtime_plan.cpu_quota,
        kv_cache_type_k: model.parameters.kv_cache_type_k.clone(),
        kv_cache_type_v: model.parameters.kv_cache_type_v.clone(),
        batch_size: model.parameters.batch_size,
        ubatch_size: model.parameters.ubatch_size,
        rationale: runtime_plan.rationale.clone(),
        overrides_applied: runtime_plan.overrides_applied.clone(),
    };

    Ok((
        ScenarioSpec {
            model,
            ..scenario.clone()
        },
        summary,
    ))
}

fn parse_numeric_flag(args: &[String], flag: &str) -> Option<u32> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            if let Some(raw) = iter.next() {
                if let Ok(value) = raw.parse::<u32>() {
                    return Some(value);
                }
            }
            return None;
        }
    }
    None
}

fn parse_string_flag(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            if let Some(raw) = iter.next() {
                return Some(raw.to_string());
            }
            return None;
        }
    }
    None
}

fn fail(json: bool, error: String) -> ExitCode {
    if json {
        println!(
            "{{\"status\":\"failed\",\"error\":{}}}",
            serde_json::to_string(&error).unwrap_or_else(|_| "\"unknown\"".into())
        );
    } else {
        eprintln!("benchmark failed: {error}");
    }
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_plan_normalizes_legacy_hermes_agent_aliases() {
        let mut file = std::env::temp_dir();
        file.push("cynic-inference-benchmark-legacy-plan.json");
        let scenario = serde_json::json!({
            "scenario_id":"legacy-hermes",
            "runtime_kind":"hermes_agent",
            "runtime_id":"hermes_agent",
            "model": {
                "parameters": {
                    "model_id":"qwythos-9b-q4",
                    "model_path":"injected-by-profile",
                    "context_requested": 128,
                    "parallel_slots": 1
                },
                "role":"agent"
            },
            "tasks": [
                {
                    "task_id":"legacy",
                    "kind":"audit",
                    "prompt":"noop",
                    "max_output_tokens": 32,
                    "expected_proof":""
                }
            ],
            "thresholds": {}
        });
        let scenario_upper = serde_json::json!({
            "scenario_id":"legacy-hermes-uppercase",
            "runtime_kind":"hermes_agent",
            "runtime_id":"  HERMES_AGENT ",
            "model": {
                "parameters": {
                    "model_id":"qwythos-9b-q4",
                    "model_path":"injected-by-profile",
                    "context_requested": 128,
                    "parallel_slots": 1
                },
                "role":"agent"
            },
            "tasks": [
                {
                    "task_id":"legacy",
                    "kind":"audit",
                    "prompt":"noop",
                    "max_output_tokens": 32,
                    "expected_proof":""
                }
            ],
            "thresholds": {}
        });
        let plan = serde_json::json!({
            "schema":"cynic.agent_harness.plan.v1",
            "plan_id":"legacy_alias",
            "scenarios":[
                {"scenario": scenario, "task_order":["legacy"]},
                {"scenario": scenario_upper, "task_order":["legacy"]}
            ]
        });
        fs::write(&file, serde_json::to_string(&plan).expect("serialize plan"))
            .expect("write temp plan");

        let loaded = load_plan(file.to_string_lossy().as_ref())
            .unwrap_or_else(|error| panic!("load_plan failed: {error}"));
        assert_eq!(
            loaded.scenarios[0].scenario.runtime_id,
            Some("hermes-agent".to_string())
        );
        assert_eq!(
            loaded.scenarios[1].scenario.runtime_id,
            Some("hermes-agent".to_string())
        );

        let _ = fs::remove_file(file);
    }
}
