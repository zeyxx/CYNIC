use super::{
    AgentHarnessReport, BENCHMARK_SCHEMA, CoordinationPort, CycleEnvelope, HarnessExecutor,
    HarnessObserver, RuntimeLayer, ScenarioSpec, StopDecision, evaluate_sample,
};
use cynic_protocol::CycleTransitionRecord;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

/// Execute a scenario in declared task order.  The runner owns no process or
/// host-specific knowledge; those remain injected ports.
pub async fn run_scenario<E: HarnessExecutor + ?Sized, O: HarnessObserver + ?Sized>(
    run_id: impl Into<String>,
    started_at: impl Into<String>,
    scenario: ScenarioSpec,
    executor: &E,
    observer: &O,
) -> AgentHarnessReport {
    run_scenario_with_coordination(run_id, started_at, scenario, executor, observer, None).await
}

pub async fn run_scenario_with_coordination<
    E: HarnessExecutor + ?Sized,
    O: HarnessObserver + ?Sized,
>(
    run_id: impl Into<String>,
    started_at: impl Into<String>,
    scenario: ScenarioSpec,
    executor: &E,
    observer: &O,
    coordination: Option<&dyn CoordinationPort>,
) -> AgentHarnessReport {
    let run_id = run_id.into();
    let mut scenario = scenario;
    if scenario.correlation_id.is_none() {
        scenario.correlation_id = Some(run_id.clone());
    }
    let mut report = AgentHarnessReport {
        schema: BENCHMARK_SCHEMA.into(),
        run_id: run_id.clone(),
        scenario: scenario.clone(),
        samples: Vec::new(),
        started_at: started_at.into(),
        finished_at: None,
        status: "running".into(),
        runtime_launch_plan: None,
        cycle: Some(CycleEnvelope::new(
            run_id,
            RuntimeLayer::Agent,
            "downward",
            "task_execution",
        )),
        runtime_transition: None,
        transition: Some(CycleTransitionRecord::planned(
            scenario
                .correlation_id
                .clone()
                .unwrap_or_else(|| scenario.scenario_id.clone()),
            serde_json::to_value(&scenario).unwrap_or_default(),
            json!({"status":"observed_before_execution"}),
            json!({"decision":"execute_declared_tasks"}),
        )),
    };
    let mut coordination_receipt = None;
    if let (Some(coordination), Some(task)) = (coordination, scenario.tasks.first()) {
        match coordination
            .observe_or_create(
                report.run_id.as_str(),
                scenario.coordination_id.as_deref(),
                task,
            )
            .await
        {
            Ok(receipt) => coordination_receipt = Some(receipt),
            Err(error) => {
                report.status = format!("blocked_coordination:{error}");
                return finalize(report);
            }
        }
    }
    for task in &scenario.tasks {
        let reply = executor.execute(&scenario, task).await;
        let sample = (
            task.task_id.clone(),
            reply,
            observer.observe(),
            scenario.model.parameters.context_per_slot,
        )
            .into();
        let decision = evaluate_sample(&sample, &scenario.thresholds);
        report.samples.push(sample);
        if let (Some(coordination), Some(receipt)) = (coordination, coordination_receipt.as_ref()) {
            if let Err(error) = coordination.heartbeat(receipt).await {
                report.status = format!("blocked_coordination_heartbeat:{error}");
                let _ = coordination.block(receipt, &report.status).await;
                return finalize(report);
            }
        }
        if let Some(transition) = report.transition.take() {
            report.transition = Some(transition.with_action(json!({
                "task_id": task.task_id,
                "decision": format!("{decision:?}").to_lowercase(),
            })));
        }
        if decision != StopDecision::Continue {
            report.status = format!("stopped_{decision:?}").to_lowercase();
            if let (Some(coordination), Some(receipt)) =
                (coordination, coordination_receipt.as_ref())
            {
                if let Err(error) = coordination.block(receipt, &report.status).await {
                    report.status = format!("blocked_coordination_block:{error}");
                }
            }
            return finalize(report);
        }
    }
    report.status = "completed".into();
    if let (Some(coordination), Some(receipt)) = (coordination, coordination_receipt.as_ref()) {
        if let Err(error) = coordination
            .complete(receipt, "benchmark scenario completed")
            .await
        {
            report.status = format!("blocked_coordination_completion:{error}");
            return finalize(report);
        }
    }
    if let Some(transition) = report.transition.take() {
        report.transition = Some(transition.with_proof(json!({
            "status": &report.status,
            "samples": &report.samples,
        })));
    }
    finalize(report)
}

/// Records the post-action observation time on every terminal outcome.  This
/// is intentionally a local read-back timestamp, never a claim about remote
/// runtime clock authority.
fn finalize(mut report: AgentHarnessReport) -> AgentHarnessReport {
    report.finished_at = Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis().to_string())
            .unwrap_or_else(|_| "clock_unavailable".to_string()),
    );
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::agent_harness::{
        InferenceReply, MemorySample, ModelParameters, ModelSpec, TaskKind, TaskSpec, Thresholds,
    };
    use async_trait::async_trait;

    struct Executor;

    #[async_trait]
    impl HarnessExecutor for Executor {
        async fn execute(&self, _: &ScenarioSpec, _: &TaskSpec) -> InferenceReply {
            InferenceReply {
                runtime_run_id: None,
                http_status: Some(200),
                prompt_tokens: Some(1),
                output_tokens: Some(1),
                prompt_tokens_per_second: None,
                output_tokens_per_second: None,
                wall_time_ms: Some(1),
                event_count: None,
                event_kinds: Vec::new(),
                event_stream_error: None,
                text_response: None,
                error: None,
            }
        }
    }

    struct Observer;

    impl HarnessObserver for Observer {
        fn observe(&self) -> MemorySample {
            MemorySample::default()
        }
    }

    fn scenario() -> ScenarioSpec {
        ScenarioSpec {
            scenario_id: "scenario".to_string(),
            probe_runtime_plan: false,
            correlation_id: None,
            runtime_kind: Default::default(),
            runtime_id: None,
            runtime_version: None,
            coordination_kind: None,
            coordination_id: None,
            tool_count: None,
            max_agent_steps: None,
            model: ModelSpec {
                role: "worker".to_string(),
                parameters: ModelParameters {
                    model_id: "model".to_string(),
                    model_path: "injected".to_string(),
                    model_digest: None,
                    architecture: None,
                    quantization: None,
                    context_requested: 1,
                    parallel_slots: 1,
                    context_per_slot: Some(1),
                    kv_cache_type_k: None,
                    kv_cache_type_v: None,
                    flash_attention: None,
                    gpu_layers: None,
                    fit_mode: None,
                    threads: None,
                    batch_size: None,
                    ubatch_size: None,
                    temperature: None,
                    top_p: None,
                    reasoning_budget: None,
                },
            },
            tasks: vec![TaskSpec {
                task_id: "task".to_string(),
                kind: TaskKind::Audit,
                prompt: "observe".to_string(),
                max_output_tokens: 1,
                expected_proof: "response".to_string(),
            }],
            thresholds: Thresholds {
                min_available_memory_bytes: None,
                min_available_memory_percent: None,
                max_rss_bytes: None,
                max_rss_percent: None,
                max_wall_time_ms: None,
            },
        }
    }

    #[tokio::test]
    async fn terminal_report_includes_post_action_observation_time() {
        let report = run_scenario("run", "started", scenario(), &Executor, &Observer).await;
        assert_eq!(report.status, "completed");
        assert!(report.finished_at.is_some());
    }
}
