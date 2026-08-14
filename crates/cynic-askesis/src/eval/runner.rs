use chrono::Utc;

use crate::error::AskesisError;
use crate::eval::oracle::{judge_text, judge_tools};
use crate::eval::port::{CompletionPort, FixtureToolExecutor, ToolExecutionPort};
use crate::eval::types::{
    AgentEvalDataset, AgentEvalReport, CompletionRequest, DistributionSummary, EvalCase,
    EvalCaseResult, EvalUncertainty, PassRateSummary, TokenUsage,
};

pub async fn run_agent_eval<P: CompletionPort>(
    dataset: &AgentEvalDataset,
    backend: &P,
    target_percent: f64,
    repetitions: u32,
    seed_stride: i64,
) -> crate::Result<AgentEvalReport> {
    if repetitions == 0 {
        return Err(AskesisError::EvalPolicy(
            "repetitions must be greater than zero".into(),
        ));
    }
    let started_at = Utc::now().to_rfc3339();
    let executor = FixtureToolExecutor;
    let mut cases = Vec::with_capacity(dataset.cases.len() * repetitions as usize);
    for case in &dataset.cases {
        for run_index in 0..repetitions {
            cases.push(run_sample(case, backend, &executor, run_index, seed_stride).await?);
        }
    }
    let scores: Vec<f64> = cases.iter().filter_map(|case| case.score_percent).collect();
    let latencies: Vec<f64> = cases.iter().map(|case| case.latency_ms as f64).collect();
    let unknown_count = cases.len() - scores.len();
    let score_distribution = distribution(&scores, true);
    let pass_rate = pass_rate(cases.iter().filter(|case| case.passed).count(), cases.len());
    let target_confidence_met = score_distribution
        .ci95_low
        .map(|lower| lower >= target_percent)
        .unwrap_or(false);
    let passed = score_distribution
        .mean
        .map(|mean| mean >= target_percent)
        .unwrap_or(false)
        && target_confidence_met
        && unknown_count == 0;
    Ok(AgentEvalReport {
        schema: "cynic.askesis.agent_eval.report.v1",
        status: if passed { "ok" } else { "below_target" },
        dataset_path: dataset.path.clone(),
        dataset_sha256: dataset.sha256.clone(),
        endpoint: backend.endpoint().to_string(),
        model: backend.model().to_string(),
        started_at,
        finished_at: Utc::now().to_rfc3339(),
        repetitions,
        seed_stride,
        sample_count: cases.len(),
        cases,
        score_percent: score_distribution.mean,
        uncertainty: EvalUncertainty {
            score_percent: score_distribution,
            latency_ms: distribution(&latencies, false),
            pass_rate,
        },
        target_percent,
        target_confidence_met,
        passed,
        unknown_count,
        residual_risk: vec![
            "fixture tools prove call selection and control flow, not production MCP side effects"
                .into(),
            "v0 text oracles do not yet compile generated patches in isolated worktrees".into(),
            "RAM, swap, energy, and long-context retention require runtime harness proof".into(),
        ],
    })
}

async fn run_sample<P: CompletionPort, T: ToolExecutionPort>(
    case: &EvalCase,
    backend: &P,
    executor: &T,
    run_index: u32,
    seed_stride: i64,
) -> crate::Result<EvalCaseResult> {
    let mut policy = case.policy.clone();
    policy.seed = policy
        .seed
        .saturating_add(seed_stride.saturating_mul(run_index as i64));
    let mut messages = Vec::new();
    if let Some(system) = &case.system {
        messages.push(serde_json::json!({"role": "system", "content": system}));
    }
    messages.push(serde_json::json!({"role": "user", "content": case.prompt}));
    let mut executions = Vec::new();
    let mut calls = Vec::new();
    let mut timings = Vec::new();
    let mut total_latency = 0u64;
    let mut prompt_tokens = 0u64;
    let mut completion_tokens = 0u64;
    let mut total_tokens = 0u64;
    let mut final_content = String::new();
    let mut finish_reason = None;

    for tool_turn in 0..=case.max_tool_turns {
        let completion = backend
            .complete(&CompletionRequest {
                messages: messages.clone(),
                tools: case.tools.clone(),
                policy: policy.clone(),
            })
            .await?;
        total_latency = total_latency.saturating_add(completion.latency_ms);
        prompt_tokens = prompt_tokens.saturating_add(completion.usage.prompt_tokens.unwrap_or(0));
        completion_tokens =
            completion_tokens.saturating_add(completion.usage.completion_tokens.unwrap_or(0));
        total_tokens = total_tokens.saturating_add(completion.usage.total_tokens.unwrap_or(0));
        if let Some(value) = completion.timings {
            timings.push(value);
        }
        finish_reason = completion.finish_reason.clone();
        if completion.tool_calls.is_empty() || tool_turn == case.max_tool_turns {
            final_content = completion.content;
            calls.extend(completion.tool_calls);
            break;
        }
        messages.push(serde_json::json!({
            "role": "assistant",
            "content": completion.content,
            "tool_calls": completion.tool_calls.iter().map(|call| serde_json::json!({
                "id": call.id,
                "type": "function",
                "function": {"name": call.name, "arguments": call.arguments_json}
            })).collect::<Vec<_>>()
        }));
        for call in completion.tool_calls {
            let execution = executor.execute(&call, &case.tools).await;
            messages.push(serde_json::json!({
                "role": "tool",
                "tool_call_id": call.id,
                "content": execution.result.to_string()
            }));
            calls.push(call);
            executions.push(execution);
        }
    }

    let (mut oracle_checks, _) = judge_text(&case.oracle, &final_content);
    if let Some(tool_oracle) = &case.tool_oracle {
        oracle_checks.extend(judge_tools(tool_oracle, &executions));
    }
    let score_percent = (!oracle_checks.is_empty()).then(|| {
        oracle_checks.iter().filter(|check| check.passed).count() as f64 * 100.0
            / oracle_checks.len() as f64
    });
    let passed = score_percent
        .map(|score| score >= case.target_percent)
        .unwrap_or(false);
    Ok(EvalCaseResult {
        id: case.id.clone(),
        domain: case.domain.clone(),
        run_index,
        effective_seed: policy.seed,
        policy,
        required_context_tokens: case.required_context_tokens,
        response: final_content,
        finish_reason,
        usage: TokenUsage {
            prompt_tokens: Some(prompt_tokens),
            completion_tokens: Some(completion_tokens),
            total_tokens: Some(total_tokens),
        },
        latency_ms: total_latency,
        timings,
        tool_calls: calls,
        tool_executions: executions,
        oracle_checks,
        score_percent,
        target_percent: case.target_percent,
        passed,
    })
}

fn distribution(values: &[f64], bounded_percent: bool) -> DistributionSummary {
    if values.is_empty() {
        return DistributionSummary {
            count: 0,
            mean: None,
            stddev: None,
            min: None,
            max: None,
            ci95_low: None,
            ci95_high: None,
        };
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = if values.len() > 1 {
        values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / (values.len() - 1) as f64
    } else {
        0.0
    };
    let stddev = variance.sqrt();
    let margin = 1.96 * stddev / (values.len() as f64).sqrt();
    let clamp = |value: f64| {
        if bounded_percent {
            value.clamp(0.0, 100.0)
        } else {
            value.max(0.0)
        }
    };
    DistributionSummary {
        count: values.len(),
        mean: Some(mean),
        stddev: Some(stddev),
        min: values.iter().copied().reduce(f64::min),
        max: values.iter().copied().reduce(f64::max),
        ci95_low: Some(clamp(mean - margin)),
        ci95_high: Some(clamp(mean + margin)),
    }
}

fn pass_rate(passed: usize, count: usize) -> PassRateSummary {
    if count == 0 {
        return PassRateSummary {
            count,
            passed,
            percent: None,
            wilson95_low: None,
            wilson95_high: None,
        };
    }
    let n = count as f64;
    let p = passed as f64 / n;
    let z = 1.96;
    let denominator = 1.0 + z * z / n;
    let center = (p + z * z / (2.0 * n)) / denominator;
    let margin = z * ((p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt()) / denominator;
    PassRateSummary {
        count,
        passed,
        percent: Some(p * 100.0),
        wilson95_low: Some((center - margin).max(0.0) * 100.0),
        wilson95_high: Some((center + margin).min(1.0) * 100.0),
    }
}
